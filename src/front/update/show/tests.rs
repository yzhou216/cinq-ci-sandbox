// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of opening and saving shows, and of keeping their unsaved changes
//! as long as they last

use std::fs;

use tempfile::TempDir;

use super::{file::write, *};
use crate::back::{Command, CueId, Edit, Kind, Place};

/// Show file of one list of one memo cue
const SHOW: &str = "[[list]]\nname = \"Main\"\n\n[[list.cue]]\nid = 1\nnumber = \"1\"\nkind = \"memo\"\nname = \"Note\"\n";

/// File of the storm show, in `/shows`
const STORM: &str = "/shows/storm.cinq.toml";

/// File of the calm show, in `/shows`
const CALM: &str = "/shows/calm.cinq.toml";

/// Returns what reading the show in `file` found: `text` in its file, and
/// `kept` among the unsaved changes
fn found(file: Option<&str>, text: Option<&str>, kept: Option<&str>) -> Read {
    Read {
        file: file.map(PathBuf::from),
        directory: PathBuf::from("/shows"),
        text: Ok(text.map(str::to_owned)),
        kept: Ok(kept.map(str::to_owned)),
    }
}

/// Returns the state of the interface with the storm show open
fn storm() -> App {
    App::default().opened(found(Some(STORM), Some(SHOW), None))
}

/// Renames the memo cue of the storm show
fn renamed(app: App, name: &str) -> App {
    app.send(Command::Edit(CueId(1), Edit::Name(name.to_owned())))
}

/// Returns what saving the show of `app` in `file` finds when it succeeds
fn saving(app: &App, file: &str) -> Saved {
    let file = PathBuf::from(file);
    Saved {
        show: app.show.id,
        text: app.workspace.show(file.parent()),
        file,
        result: Ok(()),
    }
}

/// Returns a keeper of unsaved changes in a new temporary directory, along
/// with the directory, which goes once the test drops it
fn keeper() -> (Keeper, TempDir) {
    let directory = TempDir::new().expect("a temporary directory can be made");
    (Keeper::new(Some(directory.path().to_owned())), directory)
}

#[test]
fn show_changed_and_changed_back_has_nothing_to_save() {
    let app = storm();
    assert!(!app.show.unsaved());
    let changed = renamed(app, "Cue");
    assert!(changed.show.unsaved());
    assert!(!changed.send(Command::Undo).show.unsaved());
}

#[test]
fn opening_brings_back_kept_changes_as_one_change_that_undo_takes_back() {
    let kept = SHOW.replace("Note", "Changed");
    let app = App::default().opened(found(Some(STORM), Some(SHOW), Some(&kept)));
    assert!(app.show.unsaved());
    let name = app.workspace.cue(CueId(1)).map(|cue| cue.name.clone());
    assert_eq!(name.as_deref(), Some("Changed"));
    assert!(!app.send(Command::Undo).show.unsaved());
}

#[test]
fn opening_again_the_show_on_screen_brings_back_its_changes_on_screen() {
    let older = SHOW.replace("Note", "Older");
    let app = renamed(storm(), "Newer").opened(found(Some(STORM), Some(SHOW), Some(&older)));
    let name = app.workspace.cue(CueId(1)).map(|cue| cue.name.clone());
    assert_eq!(name.as_deref(), Some("Newer"));
    // Nothing to tell of changes that stay on screen
    assert_eq!(app.told, None);
    assert!(!app.send(Command::Undo).show.unsaved());
}

#[test]
fn new_show_brings_back_the_kept_changes_of_the_untitled_show() {
    let kept = SHOW.replace("Note", "Changed");
    let app = App::default().opened(found(None, None, Some(&kept)));
    let name = app.workspace.cue(CueId(1)).map(|cue| cue.name.clone());
    assert_eq!(name.as_deref(), Some("Changed"));
    // Selection is the standby of the cues that came back
    assert_eq!(app.selected, Some(CueId(1)));
}

#[test]
fn opening_a_file_that_is_not_there_starts_a_new_show_that_saving_puts_there() {
    let app = App::default().opened(found(Some("/shows/new.cinq.toml"), None, None));
    assert_eq!(app.show.name(), "new.cinq.toml");
    assert_eq!(app.workspace.lists().len(), 1);
    assert!(!app.show.unsaved());
}

#[test]
fn file_that_is_not_a_show_leaves_the_show_on_screen() {
    let app = storm().opened(found(Some("/shows/broken.cinq.toml"), Some("[[list"), None));
    assert_eq!(app.show.name(), "storm.cinq.toml");
    assert!(app.file_problem.is_some());
}

#[test]
fn opening_another_show_tells_where_the_unsaved_changes_went() {
    let told = "Kept the unsaved changes to storm.cinq.toml | Control+o: open";
    let app = renamed(storm(), "Cue").opened(found(Some(CALM), Some(SHOW), None));
    assert_eq!(app.told.as_deref(), Some(told));
    // Also when the show opened brings back unsaved changes of its own
    let kept = SHOW.replace("Note", "Calm");
    let app = renamed(storm(), "Cue").opened(found(Some(CALM), Some(SHOW), Some(&kept)));
    assert_eq!(app.told.as_deref(), Some(told));
    // Without the key for a new show, whose changes only starting cinq
    // without a show brings back
    let untitled = App::default().opened(found(None, None, Some(SHOW)));
    let app = untitled.opened(found(Some(CALM), Some(SHOW), None));
    assert_eq!(
        app.told.as_deref(),
        Some("Kept the unsaved changes to untitled")
    );
}

#[test]
fn saving_leaves_unsaved_only_the_changes_made_since() {
    let new = App::default()
        .opened(found(None, None, None))
        .send(Command::Add {
            list: 0,
            kind: Kind::Memo,
            at: Place::End,
        });
    let file = "/shows/new.cinq.toml";
    let save = saving(&new, file);
    let after = renamed(new, "Later").saved(save);
    assert_eq!(after.show.name(), "new.cinq.toml");
    assert!(after.show.unsaved());
    let again = saving(&after, file);
    assert!(!after.saved(again).show.unsaved());
}

#[test]
fn failed_saving_leaves_the_changes_unsaved() {
    let changed = renamed(storm(), "Cue");
    let failed = Saved {
        result: Err("disk full".to_owned()),
        ..saving(&changed, STORM)
    };
    let app = changed.saved(failed);
    assert!(app.show.unsaved());
    assert_eq!(
        app.file_problem.as_deref(),
        Some("Cannot save /shows/storm.cinq.toml: disk full")
    );
}

#[test]
fn keeper_keeps_unsaved_changes_until_they_are_gone() {
    let (mut keeper, directory) = keeper();
    let kept = kept_file(directory.path(), Some(Path::new(STORM)));
    let app = storm();
    assert_eq!(keeper.keep(&app.show), None);
    keeper.settled();
    assert!(!kept.exists());
    let changed = renamed(app, "Cue");
    assert_eq!(keeper.keep(&changed.show), None);
    keeper.settled();
    assert_eq!(
        fs::read_to_string(&kept).ok(),
        Some(changed.show.text.clone())
    );
    assert_eq!(keeper.keep(&changed.send(Command::Undo).show), None);
    keeper.settled();
    assert!(!kept.exists());
}

#[test]
fn keeper_leaves_the_kept_changes_of_a_show_that_another_takes_the_place_of() {
    let (mut keeper, directory) = keeper();
    let kept = kept_file(directory.path(), Some(Path::new(STORM)));
    let changed = renamed(storm(), "Cue");
    keeper.keep(&changed.show);
    let other = changed.opened(found(Some(CALM), Some(SHOW), None));
    assert_eq!(keeper.keep(&other.show), None);
    keeper.settled();
    assert!(kept.exists());
}

#[test]
fn kept_changes_move_along_with_their_show() {
    let (mut keeper, directory) = keeper();
    let kept = kept_file(directory.path(), Some(Path::new(STORM)));
    let moved = "/elsewhere/storm.cinq.toml";
    let changed = renamed(storm(), "Cue");
    keeper.keep(&changed.show);
    let save = saving(&changed, moved);
    let saved = renamed(changed, "Later").saved(save);
    assert_eq!(keeper.keep(&saved.show), None);
    keeper.settled();
    assert!(!kept.exists());
    assert!(kept_file(directory.path(), Some(Path::new(moved))).exists());
}

#[test]
fn show_named_through_a_parent_directory_keeps_its_changes_as_by_its_plain_path() {
    let kept = |file: &str| kept_file(Path::new("/unsaved"), Some(Path::new(file)));
    assert_eq!(kept("/shows/old/../storm.cinq.toml"), kept(STORM));
}

#[test]
fn save_that_ends_after_another_show_opens_leaves_that_show_alone() {
    let changed = renamed(storm(), "Cue");
    let save = saving(&changed, STORM);
    let calm = changed.opened(found(Some(CALM), Some(SHOW), None));
    let saved = calm.show.saved.clone();
    let after = calm.saved(save);
    assert_eq!(after.show.name(), "calm.cinq.toml");
    assert_eq!(after.show.saved, saved);
    assert!(!after.show.unsaved());
}

#[test]
fn show_read_back_holds_what_was_written_along_with_its_kept_changes() {
    let directory = TempDir::new().expect("a temporary directory can be made");
    let file = directory.path().join("storm.cinq.toml");
    let unsaved_directory = directory.path().join("unsaved");
    write(&file, SHOW, true).expect("the show can be written");
    write(&kept_file(&unsaved_directory, Some(&file)), "kept", false).expect("changes can be kept");
    let read = Read::new(Some(file), Some(&unsaved_directory));
    assert_eq!(read.text, Ok(Some(SHOW.to_owned())));
    assert_eq!(read.kept, Ok(Some("kept".to_owned())));
}

#[test]
fn chosen_file_gets_the_ending_of_a_show_unless_it_is_toml() {
    assert_eq!(with_extension(Path::new("/shows/storm")), Path::new(STORM));
    assert_eq!(
        with_extension(Path::new("/shows/storm.toml")),
        Path::new("/shows/storm.toml")
    );
    assert_eq!(
        with_extension(Path::new("/shows/storm.v2")),
        Path::new("/shows/storm.v2.cinq.toml")
    );
}

#[test]
fn keeper_tells_at_its_next_turn_why_a_change_failed() {
    let directory = TempDir::new().expect("a temporary directory can be made");
    // File where the directory of unsaved changes would go
    let blocked = directory.path().join("unsaved");
    fs::write(&blocked, "").expect("the file can be written");
    let mut keeper = Keeper::new(Some(blocked));
    let changed = renamed(storm(), "Cue");
    assert_eq!(keeper.keep(&changed.show), None);
    keeper.settled();
    let failed = keeper.keep(&changed.show);
    assert!(
        failed.as_deref().is_some_and(
            |why| why.starts_with("Cannot keep the unsaved changes to storm.cinq.toml: ")
        ),
        "{failed:?}"
    );
}

#[cfg(unix)]
#[test]
fn saving_keeps_the_permissions_of_the_show_and_the_link_to_it() {
    use std::os::unix::fs::{PermissionsExt, symlink};

    let directory = TempDir::new().expect("a temporary directory can be made");
    let show = directory.path().join("storm.cinq.toml");
    fs::write(&show, SHOW).expect("the show can be written");
    fs::set_permissions(&show, fs::Permissions::from_mode(0o640))
        .expect("the permissions can be set");
    let link = directory.path().join("link.cinq.toml");
    symlink(&show, &link).expect("the link can be made");
    write(&link, "changed", true).expect("the show can be saved");
    assert_eq!(fs::read_to_string(&show).ok().as_deref(), Some("changed"));
    assert!(fs::symlink_metadata(&link).is_ok_and(|metadata| metadata.file_type().is_symlink()));
    let mode = fs::metadata(&show).map(|metadata| metadata.permissions().mode() & 0o777);
    assert_eq!(mode.ok(), Some(0o640));
}

#[test]
fn keeper_makes_every_change_sent_to_it_before_it_goes() {
    let (mut keeper, directory) = keeper();
    let kept = kept_file(directory.path(), Some(Path::new(STORM)));
    let changed = renamed(storm(), "Cue");
    keeper.keep(&changed.show);
    drop(keeper);
    assert_eq!(fs::read_to_string(&kept).ok(), Some(changed.show.text));
}
