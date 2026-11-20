//! The Rust video player entry points on the shipped logo movie behind a
//! temporary virtual file system mount.

use std::path::{Path, PathBuf};

use serial_test::serial;

use super::ffi::{rust_get_video_position, rust_play_video, rust_stop_video, rust_video_playing};
use crate::io::fixture::TempMount;

fn logo_dir() -> PathBuf {
    Path::new(env!("UQM_RUST_ROOT")).join("../sc2/content/addons/3dovideo/logo")
}

pub(super) fn logo_directory() -> TempMount {
    let mount = TempMount::new();
    for ext in ["duk", "frm", "hdr", "tbl"] {
        let bytes = std::fs::read(logo_dir().join(format!("logo.{ext}"))).unwrap();
        mount.write(&format!("logo.{ext}"), &bytes);
    }
    mount
}

/// Stops whatever the Rust player is holding when a test ends.
struct StopOnDrop;

impl Drop for StopOnDrop {
    fn drop(&mut self) {
        rust_stop_video();
    }
}

#[test]
#[serial]
fn the_player_entry_points_start_report_and_stop_a_movie() {
    let _stop = StopOnDrop;
    let mount = logo_directory();
    assert!(!rust_video_playing());
    assert_eq!(rust_get_video_position(), 0);
    assert!(rust_play_video(&mount.dir(), "logo", 4, 8, false));
    assert!(rust_video_playing());
    assert_eq!(rust_get_video_position(), 0);
    rust_stop_video();
    assert!(!rust_video_playing());
    assert_eq!(rust_get_video_position(), 0);
}

#[test]
#[serial]
fn the_player_accepts_a_suffixed_name_and_replaces_the_current_movie() {
    let _stop = StopOnDrop;
    let mount = logo_directory();
    assert!(rust_play_video(&mount.dir(), "logo", 0, 0, true));
    assert!(rust_play_video(&mount.dir(), "logo.duk", 0, 0, false));
    assert!(rust_video_playing());
}

#[test]
#[serial]
fn the_player_refuses_missing_files_and_corrupt_movies() {
    let _stop = StopOnDrop;
    let mount = logo_directory();
    assert!(!rust_play_video(&mount.dir(), "absent", 0, 0, false));
    assert!(!rust_video_playing());
    drop(mount);

    for ext in ["duk", "frm", "hdr", "tbl"] {
        let mount = logo_directory();
        std::fs::remove_file(mount.path_of(&format!("logo.{ext}"))).unwrap();
        assert!(
            !rust_play_video(&mount.dir(), "logo", 0, 0, false),
            "without logo.{ext}"
        );
    }

    let mount = logo_directory();
    mount.write("logo.hdr", &[9, 9, 9]);
    assert!(!rust_play_video(&mount.dir(), "logo", 0, 0, false));
    assert!(!rust_video_playing());
}
