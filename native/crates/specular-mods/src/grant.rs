//! What a mod may reach: everything the person may (ADR 0055, "Trust").
//! `gpui-shell` grants nothing by default; a mod is trusted code, so it
//! gets files, network, processes, storage and the clipboard.

use std::path::{Path, PathBuf};

use gpui_shell::{Capabilities, ExecuteGrant};

pub(crate) fn trusted(mod_dir: &Path, data_dir: &Path) -> Capabilities {
    let mut roots: Vec<PathBuf> = vec![mod_dir.to_path_buf(), data_dir.to_path_buf()];
    if let Some(home) = std::env::var_os("HOME") {
        roots.push(PathBuf::from(home));
    }
    Capabilities::new()
        .read_roots(roots.clone())
        .write_roots(roots)
        .execute(ExecuteGrant::Unrestricted)
        .network_hosts(["*".to_owned()])
        .storage(true)
        .clipboard_read(true)
        .clipboard_write(true)
}
