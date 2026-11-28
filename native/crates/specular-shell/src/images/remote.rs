//! Pictures named by an `http(s)` URL, as a mod item's display list may name
//! them (ADR 0057): fetched on the decode thread, never watched.

use super::LoadFailure;

/// The most a picture may weigh.
const LIMIT: u64 = 32 * 1024 * 1024;

/// Whether `file` is a URL to fetch rather than a path to read.
pub(super) fn is_remote(file: &str) -> bool {
    file.starts_with("https://") || file.starts_with("http://")
}

/// The bytes at `url`.
pub(super) fn fetch(url: &str) -> Result<Vec<u8>, LoadFailure> {
    let response = ureq::get(url).call().map_err(|error| {
        tracing::debug!(url, "picture cannot be fetched: {error}");
        match error {
            ureq::Error::StatusCode(404 | 410) => LoadFailure::Missing,
            _ => LoadFailure::Failed,
        }
    })?;
    let mut body = response.into_body();
    body.with_config()
        .limit(LIMIT)
        .read_to_vec()
        .map_err(|error| {
            tracing::debug!(url, "picture cannot be read: {error}");
            LoadFailure::Failed
        })
}
