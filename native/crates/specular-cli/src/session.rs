//! Which conversation a command belongs to, so the many short commands an
//! agent runs read as one cursor in the app.

use std::fmt::Write as _;
use std::io::Read as _;
use std::process::Command;

use specular_api::SESSION_ENV;

/// How far up the process tree to look for the agent.
const MAX_DEPTH: usize = 15;

/// Sixteen bytes from the system's random source, with the clock as a
/// fallback when there is none.
pub(crate) fn random_bytes() -> [u8; 16] {
    let mut bytes = [0_u8; 16];
    let read = std::fs::File::open("/dev/urandom").and_then(|mut file| file.read_exact(&mut bytes));
    if read.is_err() {
        let nanos = (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH))
            .map_or(0, |since| since.as_nanos());
        bytes = (nanos ^ u128::from(std::process::id())).to_le_bytes();
    }
    bytes
}

/// A random id in the UUID layout.
pub(crate) fn uuid() -> String {
    let hex = random_bytes().iter().fold(String::new(), |mut hex, byte| {
        let _ = write!(hex, "{byte:02x}");
        hex
    });
    format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    )
}

/// The parent and the command name of process `pid`.
fn parent_of(pid: u32) -> Option<(u32, String)> {
    let out = Command::new("ps")
        .args(["-o", "ppid=,comm=", "-p", &pid.to_string()])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let (ppid, command) = text.trim().split_once(char::is_whitespace)?;
    Some((ppid.parse().ok()?, command.trim().to_owned()))
}

/// Whether `command` is an agent's CLI whose process lasts one
/// conversation. Every shell it starts descends from it.
fn is_agent_host(command: &str) -> bool {
    command
        .split(|c: char| !c.is_ascii_alphanumeric())
        .any(|word| word.eq_ignore_ascii_case("claude"))
}

/// `ancestor-<pid>` for the nearest agent above this process, so two
/// conversations running at once get two ids.
fn agent_ancestor() -> Option<String> {
    let mut pid = std::os::unix::process::parent_id();
    for _ in 0..MAX_DEPTH {
        let (ppid, command) = parent_of(pid)?;
        if is_agent_host(&command) {
            return Some(format!("ancestor-{pid}"));
        }
        if ppid <= 1 || ppid == pid {
            return None;
        }
        pid = ppid;
    }
    None
}

/// This command's session: `SPECULAR_SESSION_ID`, else the agent above it,
/// else one id kept in a file for every command run by hand.
pub(crate) fn id() -> String {
    if let Some(id) = std::env::var(SESSION_ENV).ok().filter(|id| !id.is_empty()) {
        return id;
    }
    if let Some(id) = agent_ancestor() {
        return id;
    }
    let file = std::env::temp_dir().join("specular-session.id");
    if let Some(id) = (std::fs::read_to_string(&file).ok())
        .map(|text| text.trim().to_owned())
        .filter(|id| !id.is_empty())
    {
        return id;
    }
    let id = uuid();
    let _ = std::fs::write(&file, &id);
    id
}
