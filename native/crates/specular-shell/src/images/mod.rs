//! Loading the images file entities show, off the main thread.
//!
//! One worker thread reads, decodes and builds the mip levels. The main
//! thread is left the texture upload, which it does when it takes the
//! result. The same thread looks at the loaded files' stamps twice a second,
//! as the note thread does, and says which changed.

mod decode;
mod device_frames;
mod gif;
#[cfg(target_os = "macos")]
mod heif;
mod svg;
mod upload;

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, SystemTime};

use specular_compositor::{ImageMips, ImageSpec};
use specular_core::PixelSize;
use specular_interact::ImageKey;

pub(crate) use self::decode::dimensions;
pub(crate) use self::upload::Uploaded;

/// How often the loaded files are looked at for a change from outside.
const CHECK_EVERY: Duration = Duration::from_millis(500);

/// Why an image has no pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LoadFailure {
    /// There is no file at the path.
    Missing,
    /// The file could not be read or decoded, or it is not on disk at all.
    Failed,
}

/// What a file decoded to, with the mip levels built.
#[derive(Debug)]
pub(crate) enum Content {
    /// A picture.
    Still(ImageMips),
    /// A gif with several frames.
    Animated {
        /// The size it is drawn at.
        size: PixelSize,
        /// Each frame's levels.
        frames: Vec<ImageMips>,
        /// How long each frame is shown.
        delays_ms: Vec<u32>,
    },
    /// An svg drawn at one size.
    Vector {
        /// The size the svg gives itself.
        intrinsic: PixelSize,
        /// The drawing.
        raster: ImageMips,
    },
}

/// One finished load.
#[derive(Debug)]
pub(crate) struct Loaded {
    pub(crate) key: ImageKey,
    pub(crate) result: Result<Content, LoadFailure>,
}

struct Job {
    key: ImageKey,
    path: PathBuf,
    spec: ImageSpec,
    /// For an svg, the pixels to draw it at.
    want: Option<PixelSize>,
    /// Whether this is a load of the file and not a redraw of an svg, so
    /// the file's stamp is taken again.
    fresh: bool,
    /// The bytes, for a drawing the binary holds and no file does.
    embedded: Option<&'static [u8]>,
}

enum Command {
    Load(Job),
    Forget(ImageKey),
}

/// What changes when a file does.
type Stamp = Option<(SystemTime, u64)>;

fn stamp(path: &Path) -> Stamp {
    let meta = std::fs::metadata(path).ok()?;
    Some((meta.modified().ok()?, meta.len()))
}

/// The decode thread and the queue of what it has finished.
#[derive(Debug)]
pub(crate) struct ImageLoader {
    /// The space folder relative paths start from.
    space: Option<PathBuf>,
    commands: Sender<Command>,
    done: Receiver<Loaded>,
    changed: Receiver<ImageKey>,
    /// For failures known without asking the thread.
    done_here: Sender<Loaded>,
}

impl ImageLoader {
    /// Starts the decode thread. `space` is the folder the `.canvas` file is
    /// in, if the document came from a file.
    pub(crate) fn new(space: Option<PathBuf>) -> io::Result<Self> {
        let (commands, command_queue) = mpsc::channel::<Command>();
        let (done_tx, done) = mpsc::channel();
        let (changed_tx, changed) = mpsc::channel();
        let done_here = done_tx.clone();
        std::thread::Builder::new()
            .name("image-decode".to_owned())
            .spawn(move || {
                let mut watched: HashMap<ImageKey, (PathBuf, Stamp)> = HashMap::new();
                // Ends when the loader, and with it the command sender, is gone.
                loop {
                    match command_queue.recv_timeout(CHECK_EVERY) {
                        Ok(Command::Load(job)) => {
                            let at = stamp(&job.path);
                            if job.fresh {
                                watched.insert(job.key, (job.path.clone(), at));
                            } else {
                                (watched.entry(job.key)).or_insert_with(|| (job.path.clone(), at));
                            }
                            let result = load(&job);
                            let loaded = Loaded {
                                key: job.key,
                                result,
                            };
                            if done_tx.send(loaded).is_err() {
                                break;
                            }
                        }
                        Ok(Command::Forget(key)) => {
                            watched.remove(&key);
                        }
                        Err(RecvTimeoutError::Timeout) => {
                            for (key, (path, seen)) in &mut watched {
                                let now = stamp(path);
                                if now != *seen {
                                    *seen = now;
                                    if changed_tx.send(*key).is_err() {
                                        return;
                                    }
                                }
                            }
                        }
                        Err(RecvTimeoutError::Disconnected) => break,
                    }
                }
            })?;
        Ok(Self {
            space,
            commands,
            done,
            changed,
            done_here,
        })
    }

    /// Queues `file`, as a file entity names it, to be loaded for a
    /// compositor with `spec`, and watched for changes. The answer comes from
    /// [`take`](Self::take).
    pub(crate) fn request(&self, key: ImageKey, file: &str, spec: ImageSpec) {
        self.send(key, file, spec, None, true);
    }

    /// Queues an svg to be drawn again to fill `want` pixels.
    pub(crate) fn redraw(&self, key: ImageKey, file: &str, spec: ImageSpec, want: PixelSize) {
        self.send(key, file, spec, Some(want), false);
    }

    fn send(
        &self,
        key: ImageKey,
        file: &str,
        spec: ImageSpec,
        want: Option<PixelSize>,
        fresh: bool,
    ) {
        let drawing = device_frames::drawing(file);
        let path = match &drawing {
            Some(_) => Some(PathBuf::from(file)),
            None => specular_interact::resolve_file(file, self.space.as_deref()),
        };
        if path.is_none() {
            tracing::debug!(file, "image is not a file on disk");
        }
        let job = |path| {
            Command::Load(Job {
                key,
                path,
                spec,
                want: drawing.as_ref().map(|drawing| drawing.want).or(want),
                fresh,
                embedded: drawing.as_ref().map(|drawing| drawing.svg),
            })
        };
        let sent = path.is_some_and(|path| self.commands.send(job(path)).is_ok());
        if !sent {
            let result = Err(LoadFailure::Failed);
            // The receiver is ours, so this cannot fail.
            let _ = self.done_here.send(Loaded { key, result });
        }
    }

    /// Stops watching `key`'s file.
    pub(crate) fn forget(&self, key: ImageKey) {
        // The thread only ends with the loader.
        let _ = self.commands.send(Command::Forget(key));
    }

    /// One finished load, if there is one.
    pub(crate) fn take(&self) -> Option<Loaded> {
        self.done.try_recv().ok()
    }

    /// One image whose file changed on disk, if there is one.
    pub(crate) fn take_changed(&self) -> Option<ImageKey> {
        self.changed.try_recv().ok()
    }
}

fn load(job: &Job) -> Result<Content, LoadFailure> {
    let path = job.path.as_path();
    let read;
    let bytes = if let Some(bytes) = job.embedded {
        bytes
    } else {
        read = std::fs::read(path).map_err(|error| {
            tracing::debug!(path = %path.display(), "image cannot be read: {error}");
            if error.kind() == io::ErrorKind::NotFound {
                LoadFailure::Missing
            } else {
                LoadFailure::Failed
            }
        })?;
        &read
    };
    content(bytes, path, job.spec, job.want).map_err(|error| {
        tracing::debug!(path = %path.display(), "image cannot be decoded: {error}");
        LoadFailure::Failed
    })
}

fn content(
    bytes: &[u8],
    path: &Path,
    spec: ImageSpec,
    want: Option<PixelSize>,
) -> Result<Content, String> {
    let built = |size, rgba: &[u8]| ImageMips::build(size, rgba, spec).map_err(|e| e.to_string());
    if svg::is_svg(path) {
        let drawn = svg::render(bytes, want, spec.max_dimension)?;
        let raster = built(drawn.raster.size, &drawn.raster.rgba)?;
        return Ok(Content::Vector {
            intrinsic: drawn.intrinsic,
            raster,
        });
    }
    if gif::is_gif(bytes)
        && let Some(gif) = gif::decode(bytes).map_err(|e| e.to_string())?
    {
        let frames = (gif.rgba.iter())
            .map(|rgba| built(gif.frame_size, rgba))
            .collect::<Result<_, _>>()?;
        return Ok(Content::Animated {
            size: gif.size,
            frames,
            delays_ms: gif.delays_ms,
        });
    }
    let decoded = decode::decode(bytes, spec.max_dimension).map_err(|e| e.to_string())?;
    built(decoded.size, &decoded.rgba).map(Content::Still)
}
