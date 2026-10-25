//! HEIC and HEIF through `ImageIO`. The `image` crate has no decoder for
//! them and macOS ships one, which is what opens an iPhone's photos.
#![expect(
    clippy::multiple_unsafe_ops_per_block,
    reason = "ImageIO calls come in runs on the same live objects; one SAFETY note covers a run"
)]

use std::ffi::c_void;

use specular_core::PixelSize;

use super::decode::Decoded;

/// A CoreFoundation object.
type Ref = *const c_void;

/// `CGRect`: an origin and a size, four doubles.
#[repr(C)]
struct CgRect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

/// `kCFNumberSInt32Type`.
const SINT32: isize = 3;
/// `kCGImageAlphaPremultipliedLast`, in the default byte order: R, G, B, A.
const RGBA_PREMULTIPLIED: u32 = 1;

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    static kCFBooleanTrue: Ref;
    static kCFTypeDictionaryKeyCallBacks: c_void;
    static kCFTypeDictionaryValueCallBacks: c_void;
    fn CFDataCreate(allocator: Ref, bytes: *const u8, length: isize) -> Ref;
    fn CFNumberCreate(allocator: Ref, kind: isize, value: *const c_void) -> Ref;
    fn CFDictionaryGetValue(dictionary: Ref, key: Ref) -> Ref;
    fn CFNumberGetValue(number: Ref, kind: isize, value: *mut c_void) -> u8;
    fn CFDictionaryCreate(
        allocator: Ref,
        keys: *const Ref,
        values: *const Ref,
        count: isize,
        key_callbacks: *const c_void,
        value_callbacks: *const c_void,
    ) -> Ref;
    fn CFRelease(object: Ref);
}

#[link(name = "ImageIO", kind = "framework")]
unsafe extern "C" {
    static kCGImageSourceCreateThumbnailFromImageAlways: Ref;
    static kCGImageSourceCreateThumbnailWithTransform: Ref;
    static kCGImageSourceThumbnailMaxPixelSize: Ref;
    static kCGImagePropertyPixelWidth: Ref;
    static kCGImagePropertyPixelHeight: Ref;
    static kCGImagePropertyOrientation: Ref;
    fn CGImageSourceCreateWithData(data: Ref, options: Ref) -> Ref;
    fn CGImageSourceCopyPropertiesAtIndex(source: Ref, index: usize, options: Ref) -> Ref;
    fn CGImageSourceCreateThumbnailAtIndex(source: Ref, index: usize, options: Ref) -> Ref;
}

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    static kCGColorSpaceSRGB: Ref;
    fn CGColorSpaceCreateWithName(name: Ref) -> Ref;
    fn CGImageGetWidth(image: Ref) -> usize;
    fn CGImageGetHeight(image: Ref) -> usize;
    fn CGBitmapContextCreate(
        data: *mut c_void,
        width: usize,
        height: usize,
        bits_per_component: usize,
        bytes_per_row: usize,
        space: Ref,
        bitmap_info: u32,
    ) -> Ref;
    fn CGContextDrawImage(context: Ref, rect: CgRect, image: Ref);
}

/// An object this module made, released when it goes.
struct Owned(Ref);

impl Owned {
    /// `made`, or `None` when the call that makes it returned nothing.
    fn new(made: Ref) -> Option<Self> {
        (!made.is_null()).then_some(Self(made))
    }
}

impl Drop for Owned {
    fn drop(&mut self) {
        // SAFETY: the object came from a Create call and is released once.
        unsafe { CFRelease(self.0) };
    }
}

/// The whole number under `key` in `properties`, a dictionary of `ImageIO`.
///
/// # Safety
///
/// `properties` is a live `CFDictionary` and `key` a `CFString`.
unsafe fn number(properties: Ref, key: Ref) -> Option<u32> {
    let mut value = 0_i32;
    // SAFETY: the caller's promise; a value that is there and is no number
    // is refused by the null and the status checks.
    let read = unsafe {
        let found = CFDictionaryGetValue(properties, key);
        !found.is_null() && CFNumberGetValue(found, SINT32, (&raw mut value).cast()) != 0
    };
    read.then(|| u32::try_from(value).ok()).flatten()
}

/// The width and height of the image in `bytes` as it is shown, turned the
/// way its orientation says. Only the header is read.
pub(super) fn dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    // SAFETY: as in `decode`: each object is checked and owned, and the
    // keys are ImageIO's own constants.
    unsafe {
        let data = Owned::new(CFDataCreate(
            std::ptr::null(),
            bytes.as_ptr(),
            isize::try_from(bytes.len()).ok()?,
        ))?;
        let source = Owned::new(CGImageSourceCreateWithData(data.0, std::ptr::null()))?;
        let properties = Owned::new(CGImageSourceCopyPropertiesAtIndex(
            source.0,
            0,
            std::ptr::null(),
        ))?;
        let width = number(properties.0, kCGImagePropertyPixelWidth)?;
        let height = number(properties.0, kCGImagePropertyPixelHeight)?;
        // Orientations 5 to 8 lie on their side.
        let turned = number(properties.0, kCGImagePropertyOrientation).is_some_and(|way| way >= 5);
        Some(if turned {
            (height, width)
        } else {
            (width, height)
        })
    }
}

/// Decodes `bytes` upright, as its orientation says, no larger than
/// `max_dimension` on its longer side. `None` when `ImageIO` cannot.
pub(super) fn decode(bytes: &[u8], max_dimension: u32) -> Option<Decoded> {
    let longest = i32::try_from(max_dimension).unwrap_or(i32::MAX);
    // SAFETY: every object is checked for null before it is used and lives
    // in an `Owned` until the end of this function. `bytes` is copied by
    // `CFDataCreate`, the keys are ImageIO's own constants, and the
    // dictionary retains its values.
    let image = unsafe {
        let data = Owned::new(CFDataCreate(
            std::ptr::null(),
            bytes.as_ptr(),
            isize::try_from(bytes.len()).ok()?,
        ))?;
        let source = Owned::new(CGImageSourceCreateWithData(data.0, std::ptr::null()))?;
        let longest = Owned::new(CFNumberCreate(
            std::ptr::null(),
            SINT32,
            (&raw const longest).cast(),
        ))?;
        let keys = [
            kCGImageSourceCreateThumbnailFromImageAlways,
            kCGImageSourceCreateThumbnailWithTransform,
            kCGImageSourceThumbnailMaxPixelSize,
        ];
        let values = [kCFBooleanTrue, kCFBooleanTrue, longest.0];
        let options = Owned::new(CFDictionaryCreate(
            std::ptr::null(),
            keys.as_ptr(),
            values.as_ptr(),
            3,
            &raw const kCFTypeDictionaryKeyCallBacks,
            &raw const kCFTypeDictionaryValueCallBacks,
        ))?;
        Owned::new(CGImageSourceCreateThumbnailAtIndex(source.0, 0, options.0))?
    };
    // SAFETY: `image` is a live `CGImage`.
    let (width, height) = unsafe { (CGImageGetWidth(image.0), CGImageGetHeight(image.0)) };
    let size = PixelSize::new(u32::try_from(width).ok()?, u32::try_from(height).ok()?);
    if size.is_empty() {
        return None;
    }
    let mut rgba = vec![0_u8; width * height * 4];
    // SAFETY: the context draws into `rgba`, which is `height` rows of
    // `width * 4` bytes and outlives the context.
    unsafe {
        let space = Owned::new(CGColorSpaceCreateWithName(kCGColorSpaceSRGB))?;
        let context = Owned::new(CGBitmapContextCreate(
            rgba.as_mut_ptr().cast(),
            width,
            height,
            8,
            width * 4,
            space.0,
            RGBA_PREMULTIPLIED,
        ))?;
        let all = CgRect {
            x: 0.0,
            y: 0.0,
            width: width as f64,
            height: height as f64,
        };
        CGContextDrawImage(context.0, all, image.0);
    }
    straighten(&mut rgba);
    Some(Decoded { size, rgba })
}

/// Turns premultiplied pixels into ones with straight alpha.
fn straighten(rgba: &mut [u8]) {
    for pixel in rgba.as_chunks_mut::<4>().0 {
        let [red, green, blue, alpha] = pixel;
        let alpha = u32::from(*alpha);
        if alpha == 0 || alpha == 255 {
            continue;
        }
        for channel in [red, green, blue] {
            *channel = (u32::from(*channel) * 255 / alpha).min(255) as u8;
        }
    }
}
