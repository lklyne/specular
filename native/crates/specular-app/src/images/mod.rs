//! Loading the images file entities show, off the main thread.
//!
//! One worker thread reads, decodes and builds the mip levels. The main
//! thread is left the texture upload, which it does when it takes the
//! result.

mod decode;
mod resolve;

use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};

use specular_compositor::{ImageMips, ImageSpec};
use specular_interact::ImageKey;

/// Why an image has no pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LoadFailure {
    /// There is no file at the path.
    Missing,
    /// The file could not be read or decoded, or it is not on disk at all.
    Failed,
}

/// One finished load.
#[derive(Debug)]
pub(crate) struct Loaded {
    pub(crate) key: ImageKey,
    pub(crate) result: Result<ImageMips, LoadFailure>,
}

struct Job {
    key: ImageKey,
    path: PathBuf,
    spec: ImageSpec,
}

/// The decode thread and the queue of what it has finished.
#[derive(Debug)]
pub(crate) struct ImageLoader {
    /// The space folder relative paths start from.
    space: Option<PathBuf>,
    jobs: Sender<Job>,
    done: Receiver<Loaded>,
    /// For failures known without asking the thread.
    done_here: Sender<Loaded>,
}

impl ImageLoader {
    /// Starts the decode thread. `space` is the folder the `.canvas` file is
    /// in, if the document came from a file.
    pub(crate) fn new(space: Option<PathBuf>) -> io::Result<Self> {
        let (jobs, job_queue) = mpsc::channel::<Job>();
        let (done_tx, done) = mpsc::channel();
        let done_here = done_tx.clone();
        std::thread::Builder::new()
            .name("image-decode".to_owned())
            .spawn(move || {
                // Ends when the loader, and with it the job sender, is gone.
                for job in job_queue {
                    let result = load(&job.path, job.spec);
                    if done_tx
                        .send(Loaded {
                            key: job.key,
                            result,
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            })?;
        Ok(Self {
            space,
            jobs,
            done,
            done_here,
        })
    }

    /// Queues `file`, as a file entity names it, to be loaded for a
    /// compositor with `spec`. The answer comes from [`take`](Self::take).
    pub(crate) fn request(&self, key: ImageKey, file: &str, spec: ImageSpec) {
        let path = resolve::resolve(file, self.space.as_deref());
        if path.is_none() {
            tracing::debug!(file, "image is not a file on disk");
        }
        let sent = path.is_some_and(|path| self.jobs.send(Job { key, path, spec }).is_ok());
        if !sent {
            let result = Err(LoadFailure::Failed);
            // The receiver is ours, so this cannot fail.
            let _ = self.done_here.send(Loaded { key, result });
        }
    }

    /// One finished load, if there is one.
    pub(crate) fn take(&self) -> Option<Loaded> {
        self.done.try_recv().ok()
    }
}

fn load(path: &Path, spec: ImageSpec) -> Result<ImageMips, LoadFailure> {
    let bytes = std::fs::read(path).map_err(|error| {
        tracing::debug!(path = %path.display(), "image cannot be read: {error}");
        if error.kind() == io::ErrorKind::NotFound {
            LoadFailure::Missing
        } else {
            LoadFailure::Failed
        }
    })?;
    let decoded = decode::decode(&bytes, spec.max_dimension).map_err(|error| {
        tracing::debug!(path = %path.display(), "image cannot be decoded: {error}");
        LoadFailure::Failed
    })?;
    ImageMips::build(decoded.size, &decoded.rgba, spec).map_err(|error| {
        tracing::debug!(path = %path.display(), "image cannot be uploaded: {error}");
        LoadFailure::Failed
    })
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use image::ImageFormat;
    use specular_core::PixelSize;

    use super::*;

    const SPEC: ImageSpec = ImageSpec {
        linear_light: false,
        max_dimension: 8192,
    };

    /// A fresh space folder under the system temp dir, removed on drop.
    struct Space(PathBuf);

    impl Space {
        fn new(name: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("specular-images-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(dir.join("assets")).unwrap();
            Self(dir)
        }
    }

    impl Drop for Space {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn wait(loader: &ImageLoader) -> Loaded {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(loaded) = loader.take() {
                return loaded;
            }
            assert!(
                Instant::now() < deadline,
                "the decode thread never answered"
            );
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    #[test]
    fn a_file_in_the_space_folder_loads_with_its_mip_levels() {
        let space = Space::new("loads");
        let png = decode::tests::encoded(8, 4, ImageFormat::Png);
        std::fs::write(space.0.join("assets/shot.png"), png).unwrap();
        let loader = ImageLoader::new(Some(space.0.clone())).unwrap();
        loader.request(ImageKey(3), "assets/shot.png", SPEC);
        let loaded = wait(&loader);
        assert_eq!(loaded.key, ImageKey(3));
        assert_eq!(loaded.result.unwrap().size(), PixelSize::new(8, 4));
    }

    #[test]
    fn a_file_that_is_not_there_is_missing() {
        let space = Space::new("missing");
        let loader = ImageLoader::new(Some(space.0.clone())).unwrap();
        loader.request(ImageKey(1), "assets/gone.png", SPEC);
        assert_eq!(wait(&loader).result.unwrap_err(), LoadFailure::Missing);
    }

    #[test]
    fn a_file_that_is_no_image_and_a_web_url_fail() {
        let space = Space::new("fails");
        std::fs::write(space.0.join("assets/drawing.svg"), "<svg/>").unwrap();
        let loader = ImageLoader::new(Some(space.0.clone())).unwrap();
        loader.request(ImageKey(1), "assets/drawing.svg", SPEC);
        assert_eq!(wait(&loader).result.unwrap_err(), LoadFailure::Failed);
        loader.request(ImageKey(2), "https://example.com/a.png", SPEC);
        let loaded = wait(&loader);
        assert_eq!(
            (loaded.key, loaded.result.unwrap_err()),
            (ImageKey(2), LoadFailure::Failed)
        );
    }

    #[test]
    fn loads_are_answered_in_the_order_they_were_asked() {
        let space = Space::new("order");
        for name in ["a", "b", "c"] {
            let png = decode::tests::encoded(4, 4, ImageFormat::Png);
            std::fs::write(space.0.join(format!("assets/{name}.png")), png).unwrap();
        }
        let loader = ImageLoader::new(Some(space.0.clone())).unwrap();
        for (key, name) in ["a", "b", "c"].into_iter().enumerate() {
            loader.request(ImageKey(key as u64), &format!("assets/{name}.png"), SPEC);
        }
        let keys = [wait(&loader).key, wait(&loader).key, wait(&loader).key];
        assert_eq!(keys, [ImageKey(0), ImageKey(1), ImageKey(2)]);
    }
}
