// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of adding, deleting, editing, and loading cues

use std::path::{Path, PathBuf};

use super::*;

#[test]
fn show_mode_blocks_edits_but_not_playback() {
    let workspace = workspace(vec![audio(1), audio(2)])
        .apply(Command::ShowMode(true))
        .apply(Command::Edit(CueId(1), Edit::Name("Renamed".to_owned())))
        .apply(moving(1, 1))
        .apply(GO);
    assert_eq!(workspace.cue(CueId(1)).unwrap().name, "Sound");
    assert_eq!(ids(&workspace), [1, 2]);
    assert_eq!(workspace.active().len(), 1);
}

#[test]
fn new_cue_before_another_targets_the_cue_it_follows() {
    let workspace =
        workspace(vec![audio(1), audio(2)]).apply(add(0, Kind::Fade, Place::Before(CueId(2))));
    assert_eq!(ids(&workspace), [1, 3, 2]);
    assert_eq!(workspace.lists()[0].cues[1].target, Some(CueId(1)));
}

#[test]
fn cue_added_after_a_list_played_to_its_end_waits_on_standby() {
    let played = workspace(vec![audio(1)]).apply(GO);
    assert_eq!(standby(&played), None);
    let id = played.next_cue_id().expect("identities remain").0;
    let above = played
        .clone()
        .apply(add(0, Kind::Audio, Place::Before(CueId(1))));
    assert_eq!(standby(&above), None);
    let below = played.apply(add(0, Kind::Audio, Place::After(CueId(1))));
    assert_eq!(standby(&below), Some(id));
}

#[test]
fn cue_added_while_a_cue_waits_on_standby_leaves_standby_alone() {
    let workspace = workspace(vec![audio(1), audio(2)]).apply(add(0, Kind::Audio, Place::End));
    assert_eq!(standby(&workspace), Some(1));
}

#[test]
fn moved_cue_stops_at_either_end_of_its_list_and_keeps_standby() {
    let shown = workspace(vec![audio(1), audio(2), audio(3)]);
    let down = shown.clone().apply(moving(1, 5));
    assert_eq!(ids(&down), [2, 3, 1]);
    assert_eq!(standby(&down), Some(1));
    assert_eq!(ids(&shown.apply(moving(3, -1))), [1, 3, 2]);
}

#[test]
fn loading_ignores_a_file_that_has_been_replaced() {
    let workspace = workspace(vec![audio(1)])
        .apply(Command::Edit(CueId(1), Edit::File("other.ogg".to_owned())))
        .apply(loaded(1, "sound.ogg", Media::Loaded { duration: 10.0 }));
    let cue = workspace.cue(CueId(1)).unwrap();
    assert_eq!(workspace.problem(cue), Some(Problem::NotLoaded));
}

#[test]
fn new_cues_are_numbered_after_the_highest_number_of_any_list() {
    let lists = vec![
        CueList::new("Main", vec![audio(1), audio(7)]),
        CueList::new("Preshow", Vec::new()),
    ];
    let workspace = Workspace::new(lists).apply(add(1, Kind::Memo, Place::End));
    assert_eq!(workspace.lists()[1].cues[0].number, "8");
}

#[test]
fn numbers_too_large_to_count_up_from_do_not_overflow() {
    let endless = Cue {
        number: "inf".to_owned(),
        ..audio(1)
    };
    let workspace = workspace(vec![endless]).apply(add(0, Kind::Memo, Place::End));
    assert_eq!(workspace.lists()[0].cues[1].number, u64::MAX.to_string());
}

#[test]
fn level_edit_leaves_a_cue_that_a_panic_is_stopping() {
    let workspace = workspace(vec![audio(1)])
        .apply(GO)
        .apply(Command::Panic)
        .apply(Command::Edit(CueId(1), Edit::Level(-3.0)))
        .apply(Command::Tick(PANIC_FADE));
    assert!(workspace.active().is_empty());
}

#[test]
fn run_whose_sound_cannot_start_ends_and_its_chain_plays_on() {
    let failed = loaded(1, "sound.ogg", Media::Failed("file not found".to_owned()));
    let workspace = workspace(vec![continuing_after(1, 2.0), audio(2)])
        .apply(GO)
        .apply(failed);
    let left: Vec<CueId> = workspace.active().iter().map(|active| active.id).collect();
    assert_eq!(left, [CueId(2)]);
    let cue = workspace.cue(CueId(1)).unwrap();
    assert_eq!(
        workspace.problem(cue),
        Some(Problem::LoadFailed("file not found"))
    );
}

#[test]
fn failure_of_a_file_that_was_replaced_leaves_the_cue_playing() {
    let replaced = loaded(1, "other.ogg", Media::Loaded { duration: 10.0 });
    let stale = loaded(1, "sound.ogg", Media::Failed("file not found".to_owned()));
    let workspace = workspace(vec![audio(1)])
        .apply(Command::Edit(CueId(1), Edit::File("other.ogg".to_owned())))
        .apply(replaced)
        .apply(GO)
        .apply(stale);
    assert!(workspace.active_of(CueId(1)).is_some());
}

#[test]
fn file_paths_leave_out_surrounding_spaces() {
    let workspace = Workspace::new(Vec::new()).directory("/show");
    assert_eq!(
        workspace.path(&playing(" sounds/storm.ogg\n")),
        Some(PathBuf::from("/show/sounds/storm.ogg"))
    );
    assert_eq!(workspace.path(&playing(" \n")), None);
}

#[test]
fn chosen_sound_file_goes_by_its_path_from_the_show_when_it_is_there() {
    let workspace = Workspace::new(Vec::new()).directory("/show");
    let name = |path: &str| workspace.file_name_of(Path::new(path));
    assert_eq!(name("/show/sounds/storm.ogg"), "sounds/storm.ogg");
    assert_eq!(name("/elsewhere/storm.ogg"), "/elsewhere/storm.ogg");
    assert_eq!(name("/showcase/storm.ogg"), "/showcase/storm.ogg");
}

#[test]
fn sound_files_are_looked_for_next_to_the_file_a_cue_plays() {
    let workspace = Workspace::new(Vec::new()).directory("/show");
    let directory = |file: &str| workspace.sound_directory(&playing(file));
    assert_eq!(directory("sounds/storm.ogg"), Path::new("/show/sounds"));
    assert_eq!(directory("/music/storm.ogg"), Path::new("/music"));
    assert_eq!(directory(""), Path::new("/show"));
}

#[test]
fn edit_that_changes_nothing_leaves_a_running_cue_playing() {
    // Text fields send their text again on keys that change nothing, such
    // as Backspace at the start of the field
    let workspace = workspace(vec![audio(1)])
        .apply(GO)
        .apply(Command::Edit(CueId(1), Edit::File("sound.ogg".to_owned())));
    let cue = workspace.cue(CueId(1)).unwrap();
    assert!(workspace.active_of(CueId(1)).is_some());
    assert_eq!(workspace.problem(cue), None);
}

#[test]
fn editing_a_running_cue_leaves_the_run_under_way() {
    let workspace = workspace(vec![audio(1)])
        .apply(GO)
        .apply(Command::Tick(3.0))
        .apply(Command::Edit(
            CueId(1),
            Edit::PreWait("10".to_owned().into()),
        ));
    let active = workspace.active_of(CueId(1)).unwrap();
    assert_eq!(active.phase(), Phase::Running { remaining: 7.0 });
}

#[test]
fn deleting_the_last_cue_on_standby_moves_standby_past_the_end() {
    let workspace = workspace(vec![audio(1), audio(2)])
        .apply(Command::Standby {
            list: 0,
            cue: Some(CueId(2)),
        })
        .apply(Command::Delete(vec![CueId(2)]));
    assert_eq!(workspace.lists()[0].standby, None);
}

#[test]
fn new_numbers_read_numbers_with_spaces_and_large_numbers_exactly() {
    let spaced = Cue {
        number: " 12 ".to_owned(),
        ..audio(1)
    };
    let large = Cue {
        number: "16777217".to_owned(),
        ..audio(2)
    };
    let add = add(0, Kind::Memo, Place::End);
    let numbers = |workspace: Workspace| workspace.lists()[0].cues.last().unwrap().number.clone();
    assert_eq!(numbers(workspace(vec![spaced]).apply(add.clone())), "13");
    assert_eq!(numbers(workspace(vec![large]).apply(add)), "16777218");
}

#[test]
fn files_that_could_not_be_played_are_listed_for_another_try() {
    let failed = loaded(1, "sound.ogg", Media::Failed("file not found".to_owned()));
    let workspace = workspace(vec![audio(1), audio(2)]).apply(failed);
    let listed: Vec<CueId> = workspace.failed().map(|cue| cue.id).collect();
    assert_eq!(listed, [CueId(1)]);
}

#[test]
fn editing_a_fade_cue_in_its_pre_wait_leaves_the_fade_it_will_make() {
    let fade = Cue {
        pre_wait: 2.0.into(),
        ..fade_out(2)
    };
    let workspace = workspace(vec![continuing(1), fade])
        .apply(GO)
        .apply(Command::Tick(1.0))
        .apply(Command::Edit(CueId(2), Edit::Level(-10.0)))
        .apply(Command::Edit(CueId(2), Edit::StopTarget(false)))
        .apply(Command::Tick(1.0));
    let fade = workspace.active_of(CueId(1)).unwrap().fade.unwrap();
    assert_eq!((fade.to, fade.then_stop), (SILENCE, true));
}

#[test]
fn editing_the_clip_of_a_running_cue_waits_for_its_next_run() {
    let workspace = workspace(vec![Cue {
        looping: true,
        ..audio(1)
    }])
    .apply(GO)
    .apply(Command::Edit(CueId(1), Edit::Loop(false)))
    .apply(Command::Tick(30.0));
    let active = workspace.active_of(CueId(1)).expect("the loop plays on");
    assert_eq!(active.phase(), Phase::Looping);
}
