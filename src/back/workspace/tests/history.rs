// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of undo and redo, which take back and make again the changes to
//! the show and leave playback alone

use super::*;

fn name(workspace: &Workspace, id: u32) -> Option<&str> {
    workspace.cue(CueId(id)).map(|cue| cue.name.as_str())
}

fn delete(ids: &[u32]) -> Command {
    Command::Delete(ids.iter().map(|&id| CueId(id)).collect())
}

fn rename(id: u32, name: &str) -> Command {
    Command::Edit(CueId(id), Edit::Name(name.to_owned()))
}

fn move_standby(id: u32) -> Command {
    Command::Standby {
        list: 0,
        cue: Some(CueId(id)),
    }
}

#[test]
fn deleted_cues_come_back_in_place_as_one_change() {
    let deleted = workspace(vec![audio(1), audio(2), audio(3), audio(4)]).apply(delete(&[2, 3]));
    assert_eq!(ids(&deleted), [1, 4]);
    let undone = deleted.apply(Command::Undo);
    assert_eq!(ids(&undone), [1, 2, 3, 4]);
    assert_eq!(ids(&undone.apply(Command::Redo)), [1, 4]);
}

#[test]
fn standby_goes_back_to_a_deleted_cue_unless_go_has_moved_it_since() {
    let deleted = workspace(vec![audio(1), audio(2), audio(3)]).apply(delete(&[1]));
    assert_eq!(standby(&deleted), Some(2));
    assert_eq!(standby(&deleted.clone().apply(Command::Undo)), Some(1));
    let played = deleted.apply(GO).apply(Command::Undo);
    assert_eq!(ids(&played), [1, 2, 3]);
    assert_eq!(standby(&played), Some(3));
}

#[test]
fn added_cue_goes_away_and_comes_back_with_its_identity_and_the_standby_it_took() {
    let played = workspace(vec![audio(1)]).apply(GO);
    let id = played.next_cue_id().expect("identities remain").0;
    let added = played.apply(add(0, Kind::Audio, Place::End));
    assert_eq!(standby(&added), Some(id));
    let undone = added.apply(Command::Undo);
    assert_eq!((ids(&undone), standby(&undone)), (vec![1], None));
    let redone = undone.apply(Command::Redo);
    assert_eq!((ids(&redone), standby(&redone)), (vec![1, id], Some(id)));
}

#[test]
fn redo_puts_no_added_cue_that_has_played_back_on_standby() {
    let played = Workspace::new(Vec::new())
        .apply(add(0, Kind::Memo, Place::End))
        .apply(GO);
    assert_eq!(standby(&played), None);
    let redone = played.apply(Command::Undo).apply(Command::Redo);
    assert_eq!(ids(&redone).len(), 1);
    assert_eq!(standby(&redone), None);
}

#[test]
fn undo_takes_back_the_name_that_a_chosen_sound_file_gave() {
    let chosen = Edit::Sound {
        file: "thunder.wav".to_owned(),
        name: "thunder".to_owned(),
    };
    let undone = workspace(vec![audio(1)])
        .apply(Command::Edit(CueId(1), chosen))
        .apply(Command::Undo);
    let cue = undone.cue(CueId(1)).unwrap();
    assert_eq!(cue.file, "sound.ogg");
    assert_eq!(cue.name, "Sound");
}

#[test]
fn moves_of_one_cue_in_a_row_make_one_change() {
    let moved = workspace(vec![audio(1), audio(2), audio(3)])
        .apply(moving(1, 1))
        .apply(moving(1, 1));
    assert_eq!(ids(&moved), [2, 3, 1]);
    let undone = moved.apply(Command::Undo);
    assert_eq!(ids(&undone), [1, 2, 3]);
    assert_eq!(ids(&undone.apply(Command::Redo)), [2, 3, 1]);
}

#[test]
fn keys_typed_into_one_setting_make_one_change() {
    let typed = workspace(vec![audio(1)])
        .apply(rename(1, "S"))
        .apply(rename(1, "St"))
        .apply(rename(1, "Storm"))
        .apply(Command::Edit(CueId(1), Edit::Level(-6.0)));
    let level = typed.apply(Command::Undo);
    assert_eq!(level.cue(CueId(1)).map(|cue| cue.level), Some(0.0));
    assert_eq!(name(&level, 1), Some("Storm"));
    assert_eq!(name(&level.apply(Command::Undo), 1), Some("Sound"));
}

#[test]
fn edits_or_moves_in_a_row_that_end_where_they_began_leave_nothing_to_undo() {
    let typed = workspace(vec![audio(1)])
        .apply(rename(1, "Soundx"))
        .apply(rename(1, "Sound"));
    assert!(typed.undoable().is_none());
    let moved = workspace(vec![audio(1), audio(2)])
        .apply(moving(1, 1))
        .apply(moving(1, -1));
    assert_eq!(ids(&moved), [1, 2]);
    assert!(moved.undoable().is_none());
}

#[test]
fn undo_takes_back_only_the_setting_it_changed() {
    let reloaded = workspace(vec![audio(1)])
        .apply(rename(1, "Storm"))
        .apply(loaded(1, "sound.ogg", Media::Loaded { duration: 20.0 }))
        .apply(Command::Undo);
    let cue = reloaded.cue(CueId(1)).expect("the cue is still there");
    assert_eq!(cue.name, "Sound");
    assert_eq!(cue.media, Media::Loaded { duration: 20.0 });
}

#[test]
fn new_change_leaves_nothing_to_redo() {
    let workspace = workspace(vec![audio(1)])
        .apply(rename(1, "Storm"))
        .apply(Command::Undo)
        .apply(Command::Edit(CueId(1), Edit::Level(-6.0)));
    assert!(workspace.redoable().is_none());
}

#[test]
fn show_mode_leaves_only_moves_of_standby_to_undo() {
    let show = workspace(vec![audio(1), audio(2)])
        .apply(rename(1, "Storm"))
        .apply(move_standby(2))
        .apply(Command::ShowMode(true))
        .apply(Command::Undo);
    assert_eq!(standby(&show), Some(1));
    let locked = show.apply(Command::Undo);
    assert_eq!(name(&locked, 1), Some("Storm"));
    assert!(locked.undoable().is_none());
}

#[test]
fn go_forgets_the_moves_of_the_standby_it_moves_on() {
    let played = workspace(vec![audio(1), audio(2), audio(3)])
        .apply(move_standby(2))
        .apply(GO);
    assert!(played.undoable().is_none());
    assert_eq!(standby(&played.apply(Command::Undo)), Some(3));
}
