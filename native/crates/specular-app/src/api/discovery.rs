//! How the CLI finds the app: a port, a secret, and the file that names
//! both, at the Electron app's path with the Electron app's shape.
//!
//! The two apps can run side by side. Whichever starts first has the port
//! and the file; this one then listens on a port the system picks and
//! writes its own file, and says how to point the CLI at it.

use std::fmt::Write as _;
use std::io::{Read as _, Write as _};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Context as _;
use serde_json::Value;
use specular_api::{DEFAULT_PORT, DISCOVERY_FILE, NATIVE_DISCOVERY_FILE, VERSION};

const PROBE_TIMEOUT: Duration = Duration::from_secs(1);

/// The port to try first: `SPECULAR_PORT`, else the Electron app's.
pub(super) fn preferred_port() -> u16 {
    (std::env::var("SPECULAR_PORT").ok())
        .and_then(|port| port.parse().ok())
        .unwrap_or(DEFAULT_PORT)
}

/// A secret no other process can guess: 32 hex digits from the system's
/// random source.
pub(super) fn new_secret() -> String {
    let mut bytes = [0_u8; 16];
    let read = std::fs::File::open("/dev/urandom").and_then(|mut file| file.read_exact(&mut bytes));
    if read.is_err() {
        // No random device: the hasher's keys are seeded by the OS too.
        use std::hash::{BuildHasher as _, Hasher as _};
        for half in bytes.chunks_mut(8) {
            let word = std::collections::hash_map::RandomState::new()
                .build_hasher()
                .finish();
            half.copy_from_slice(&word.to_le_bytes());
        }
    }
    bytes.iter().fold(String::new(), |mut hex, byte| {
        // Writing to a `String` cannot fail.
        let _ = write!(hex, "{byte:02x}");
        hex
    })
}

fn specular_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".specular"))
}

/// `SPECULAR_DISCOVERY_FILE`, which a test instance sets to keep off the
/// shared file. A relative one is under the temp folder, as in Electron.
fn override_file() -> Option<PathBuf> {
    let file = PathBuf::from(std::env::var_os("SPECULAR_DISCOVERY_FILE")?);
    Some(if file.is_absolute() {
        file
    } else {
        std::env::temp_dir().join(file)
    })
}

/// Whether a Specular app answers `/health` on `port`.
fn specular_answers(port: u16) -> bool {
    let probe = || -> std::io::Result<String> {
        let address = SocketAddr::from(([127, 0, 0, 1], port));
        let mut stream = TcpStream::connect_timeout(&address, PROBE_TIMEOUT)?;
        stream.set_read_timeout(Some(PROBE_TIMEOUT))?;
        stream.set_write_timeout(Some(PROBE_TIMEOUT))?;
        stream
            .write_all(b"GET /health HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")?;
        let mut answer = String::new();
        stream.read_to_string(&mut answer)?;
        Ok(answer)
    };
    probe().is_ok_and(|answer| {
        let body = answer.split_once("\r\n\r\n").map_or("", |(_, body)| body);
        let health: Option<Value> = serde_json::from_str(body.trim()).ok();
        health.is_some_and(|health| health["version"] == VERSION)
    })
}

/// Whether the file at `path` names a Specular app that is running.
fn names_a_live_app(path: &Path) -> bool {
    let named = || -> Option<u16> {
        let payload: Value = serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()?;
        u16::try_from(payload["port"].as_u64()?).ok()
    };
    named().is_some_and(specular_answers)
}

/// A bound listener and the discovery file that goes with it.
pub(super) struct Bound {
    pub(super) listener: tiny_http::Server,
    pub(super) port: u16,
    /// Where to write the discovery file. `None` when there is no home
    /// folder to keep it in.
    pub(super) file: Option<PathBuf>,
}

fn listen(port: u16) -> Result<(tiny_http::Server, u16), anyhow::Error> {
    let listener =
        tiny_http::Server::http(("127.0.0.1", port)).map_err(anyhow::Error::from_boxed)?;
    let port = (listener.server_addr().to_ip()).map_or(port, |address| address.port());
    Ok((listener, port))
}

/// Binds `preferred`, and takes the shared discovery file, unless another
/// Specular already has either. Then it binds a port the system picks and
/// takes this app's own file.
pub(super) fn bind(preferred: u16) -> anyhow::Result<Bound> {
    let shared = override_file().or_else(|| Some(specular_dir()?.join(DISCOVERY_FILE)));
    let own = specular_dir().map(|dir| dir.join(NATIVE_DISCOVERY_FILE));
    let taken = shared.as_deref().is_some_and(names_a_live_app);
    if !taken && let Ok((listener, port)) = listen(preferred) {
        return Ok(Bound {
            listener,
            port,
            file: shared,
        });
    }
    let holder = if taken || specular_answers(preferred) {
        "another Specular app (the Electron app?)"
    } else {
        "another process"
    };
    let (listener, port) = listen(0).context("binding a port for the API")?;
    let file = override_file().or(own);
    let hint = file.as_deref().map_or_else(String::new, |file| {
        format!(
            "; point the CLI here with SPECULAR_DISCOVERY_FILE={}",
            file.display()
        )
    });
    tracing::warn!(
        "{holder} holds port {preferred} or the discovery file, so the API is on port {port}{hint}"
    );
    Ok(Bound {
        listener,
        port,
        file,
    })
}

/// The discovery file this app wrote.
pub(super) struct Discovery {
    path: PathBuf,
    text: String,
}

impl Discovery {
    /// Writes the file, readable by its owner only: the secret in it is
    /// full authority over the app. A failure is logged and the server
    /// runs on, findable by whoever is told the port and secret.
    pub(super) fn write(path: Option<PathBuf>, port: u16, secret: &str) -> Option<Self> {
        let path = path?;
        let text = format!("{:#}", specular_api::discovery(port, secret));
        match write_private(&path, &text) {
            Ok(()) => Some(Self { path, text }),
            Err(error) => {
                tracing::warn!("discovery file not written: {error:#}");
                None
            }
        }
    }

    /// Removes the file, unless another app has since written its own
    /// there.
    pub(super) fn remove(self) {
        if std::fs::read_to_string(&self.path).is_ok_and(|text| text == self.text) {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

fn write_private(path: &Path, text: &str) -> anyhow::Result<()> {
    use std::os::unix::fs::{DirBuilderExt as _, OpenOptionsExt as _, PermissionsExt as _};
    if let Some(dir) = path.parent() {
        (std::fs::DirBuilder::new().recursive(true).mode(0o700))
            .create(dir)
            .with_context(|| format!("creating {}", dir.display()))?;
    }
    let mut file = (std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true))
    .mode(0o600)
    .open(path)
    .with_context(|| format!("opening {}", path.display()))?;
    // The mode above only applies to a file this call created.
    file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    file.write_all(text.as_bytes())
        .with_context(|| format!("writing {}", path.display()))
}
