//! The image effects: asking the decode thread for a file, uploading what
//! it finished, and telling the app.

use std::path::{Path, PathBuf};

use specular_core::PixelSize;
use specular_interact::{Event, ImageKey, ImageNotice};

use super::runtime::{Runtime, ShellWindow};
use crate::images::{ImageLoader, LoadFailure};

/// Starts the decode thread for the space at `space`, the folder relative
/// paths start from. With none, only absolute paths resolve.
pub(super) fn start_loader(space: Option<PathBuf>) -> Option<ImageLoader> {
    match ImageLoader::new(space) {
        Ok(loader) => Some(loader),
        Err(error) => {
            tracing::warn!("images will not load: cannot start the decode thread: {error}");
            None
        }
    }
}

/// The absolute folder `canvas` is in.
pub(super) fn space_folder(canvas: &Path) -> Option<PathBuf> {
    let folder = std::path::absolute(canvas).ok()?.parent()?.to_owned();
    Some(folder)
}

impl<W: ShellWindow> Runtime<W> {
    pub(super) fn load_image(&mut self, image: ImageKey, file: &str) {
        self.images.insert(image);
        self.loads.images.insert(image);
        let spec = self.gpu.as_ref().map(|gpu| gpu.compositor().image_spec());
        if let (Some(loader), Some(spec)) = (self.image_loader.as_ref(), spec) {
            loader.request(image, file, spec);
        }
    }

    /// Draws an svg again at `width` by `height` logical pixels, which are
    /// scaled to the window's. The picture on screen stays until it is done.
    pub(super) fn raster_image(&mut self, image: ImageKey, file: &str, size: PixelSize) {
        let spec = self.gpu.as_ref().map(|gpu| gpu.compositor().image_spec());
        let scale = self.gpu.as_ref().map_or(1.0, W::scale_factor);
        let device = |logical: u32| ((logical as f32 * scale).ceil() as u32).max(1);
        self.loads.images.insert(image);
        if let (Some(loader), Some(spec)) = (self.image_loader.as_ref(), spec) {
            let want = PixelSize::new(device(size.width), device(size.height));
            loader.redraw(image, file, spec, want);
        }
    }

    pub(super) fn drop_image(&mut self, image: ImageKey) {
        self.images.remove(&image);
        self.loads.images.remove(&image);
        if let Some(loader) = self.image_loader.as_ref() {
            loader.forget(image);
        }
        if let Some(gpu) = self.gpu.as_mut() {
            self.uploaded.remove(gpu.compositor_mut(), image);
        }
    }

    /// Uploads one image the decode thread finished and tells the app. One a
    /// turn, so a folder of large images does not land in a single frame.
    /// Also tells the app of a file that changed on disk. Returns whether
    /// the thread had anything to hand over.
    pub(super) fn take_loaded_image(&mut self) -> bool {
        let changed = self
            .image_loader
            .as_ref()
            .and_then(ImageLoader::take_changed);
        if let Some(image) = changed
            && self.images.contains(&image)
        {
            self.loads.images.insert(image);
            self.dispatch(Event::Image {
                image,
                notice: ImageNotice::Changed,
            });
        }
        let Some(loaded) = self.image_loader.as_ref().and_then(ImageLoader::take) else {
            return changed.is_some();
        };
        self.loads.images.remove(&loaded.key);
        // Let go of while it was decoding.
        if !self.images.contains(&loaded.key) {
            return true;
        }
        let notice = match (loaded.result, self.gpu.as_mut()) {
            (Ok(content), Some(gpu)) => {
                match self
                    .uploaded
                    .install(gpu.compositor_mut(), loaded.key, &content)
                {
                    Ok(notice) => notice,
                    Err(error) => {
                        tracing::warn!("image cannot be uploaded: {error}");
                        ImageNotice::Failed
                    }
                }
            }
            (Err(LoadFailure::Missing), _) => ImageNotice::Missing,
            (Err(LoadFailure::Failed), _) | (Ok(_), None) => ImageNotice::Failed,
        };
        self.dispatch(Event::Image {
            image: loaded.key,
            notice,
        });
        true
    }
}
