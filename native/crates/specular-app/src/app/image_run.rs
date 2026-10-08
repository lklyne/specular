//! The image effects: asking the decode thread for a file, uploading what
//! it finished, and telling the app.

use std::path::{Path, PathBuf};

use specular_interact::{Event, ImageKey, ImageNotice};
use specular_scene::ImageId;

use super::Shell;
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

impl Shell {
    pub(super) fn load_image(&mut self, image: ImageKey, file: &str) {
        self.images.insert(image);
        let spec = self.gpu.as_ref().map(|gpu| gpu.compositor.image_spec());
        if let (Some(loader), Some(spec)) = (self.image_loader.as_ref(), spec) {
            loader.request(image, file, spec);
        }
    }

    pub(super) fn drop_image(&mut self, image: ImageKey) {
        self.images.remove(&image);
        if let Some(gpu) = self.gpu.as_mut() {
            gpu.compositor.remove_image(ImageId(image.0));
        }
    }

    /// Uploads one image the decode thread finished and tells the app. One a
    /// turn, so a folder of large images does not land in a single frame.
    pub(super) fn take_loaded_image(&mut self) {
        let Some(loaded) = self.image_loader.as_ref().and_then(ImageLoader::take) else {
            return;
        };
        // Let go of while it was decoding.
        if !self.images.contains(&loaded.key) {
            return;
        }
        let notice = match (loaded.result, self.gpu.as_mut()) {
            (Ok(mips), Some(gpu)) => {
                match gpu.compositor.set_image_mips(ImageId(loaded.key.0), &mips) {
                    Ok(()) => ImageNotice::Ready {
                        width: mips.size().width,
                        height: mips.size().height,
                    },
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
    }
}
