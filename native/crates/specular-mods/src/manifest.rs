//! Reading a mod's `mod.json` from its folder. The manifest itself, and the
//! API version it names, are `specular_interact::mods`'s.

use std::path::Path;

use specular_interact::mods::{FILE, ManifestError, ModManifest};

/// Reads `dir/mod.json`.
pub fn read_manifest(dir: &Path) -> Result<ModManifest, ManifestError> {
    let path = dir.join(FILE);
    let source = std::fs::read_to_string(&path).map_err(|e| ManifestError {
        path: path.clone(),
        problem: e.to_string(),
    })?;
    ModManifest::parse(&source, &path)
}
