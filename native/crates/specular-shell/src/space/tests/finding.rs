//! Finding a space and what it holds: the settings that name it, the
//! starter space, and the listing.

use super::TempDir;
use crate::space::locate::{SpaceChoice, SpaceStart, Startup, scratch_folder, startup};

#[test]
fn the_scratch_space_is_opened_only_by_name_and_never_the_users() {
    let dir = TempDir::new("startup-scratch");
    let (electron, remembered) = (dir.0.join("electron"), dir.0.join("remembered"));
    std::fs::create_dir_all(&electron).unwrap();
    std::fs::create_dir_all(&remembered).unwrap();
    let scratch = scratch_folder(Some(&dir.0));
    assert_eq!(scratch, dir.0.join("scratch-space"));
    assert_eq!(
        startup(
            &SpaceChoice::Scratch,
            Some(electron),
            Some(remembered),
            scratch.clone()
        ),
        Startup::Open(SpaceStart {
            folder: scratch,
            file: None,
            scratch: true,
        })
    );
}
