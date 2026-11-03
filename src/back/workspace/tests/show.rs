// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of show files, which hold the settings that each kind of cue uses
//! and where its sound file is, and of bringing back cue lists kept from
//! before as one change

use std::path::Path;

use super::*;

/// Returns a show in `/show` of a storm, an audio cue at -4 dB, and a fade
/// cue that fades it out
fn storm() -> Workspace {
    workspace(vec![
        Cue::new(CueId(1), "1", Kind::Audio, "Storm")
            .file("storm.ogg")
            .level(-4.0)
            .build(),
        Cue::new(CueId(2), "2", Kind::Fade, "Storm out")
            .target(CueId(1))
            .build(),
    ])
    .directory("/show")
}

/// Returns the cue lists of a workspace as a show file in `/show` holds
/// them
fn shown(workspace: &Workspace) -> String {
    workspace.show(Some(Path::new("/show")))
}

/// Reads a show in `/show` whose file holds one list, Main, of `cues`
fn read_list(cues: &str) -> Result<Workspace, String> {
    Workspace::from_show(&format!("[[list]]\nname = \"Main\"\n{cues}"), "/show")
}

#[test]
fn show_file_holds_the_settings_that_each_kind_of_cue_uses() {
    assert_eq!(
        shown(&storm()),
        r#"[[list]]
name = "Main"

[[list.cue]]
id = 1
number = "1"
kind = "audio"
name = "Storm"
file = "storm.ogg"
level = -4.0

[[list.cue]]
id = 2
number = "2"
kind = "fade"
name = "Storm out"
target = 1
duration = 5.0
level = -60.0
stop_target = true
"#
    );
}

#[test]
fn show_file_reads_back_as_it_was_written() {
    let busy = Workspace::new(vec![
        CueList::new(
            "Main",
            vec![
                Cue {
                    pre_wait: 0.5.into(),
                    post_wait: 2.0.into(),
                    advance: Advance::Continue,
                    notes: "GO on the house lights".to_owned(),
                    ..audio(1)
                },
                Cue::new(CueId(2), "2", Kind::Wait, "Hold")
                    .duration(1.5)
                    .build(),
                Cue::new(CueId(3), "", Kind::Stop, "End")
                    .target(CueId(1))
                    .build(),
            ],
        ),
        CueList::new("Preshow", Vec::new()),
    ])
    .directory("/show");
    let text = shown(&busy);
    let read = Workspace::from_show(&text, "/show").expect("the show reads back");
    assert_eq!(shown(&read), text);
    assert_eq!(read.lists().len(), 2);
    assert_eq!(read.next_cue_id(), Some(CueId(4)));
}

#[test]
fn settings_that_a_file_leaves_out_take_the_defaults_of_their_kind() {
    let read = read_list("[[list.cue]]\nid = 7\nkind = \"fade\"\n").expect("the show reads");
    let fade = read.cue(CueId(7)).expect("the fade is there");
    assert_eq!(fade.duration.value(), 5.0);
    assert_eq!(fade.level, SILENCE);
    assert!(fade.stop_target);
    assert_eq!(fade.pre_wait.text(), "");
}

#[test]
fn show_file_without_lists_holds_one_empty_list() {
    let read = Workspace::from_show("", "/show").expect("an empty file reads");
    assert_eq!(read.lists().len(), 1);
    assert!(read.lists()[0].cues.is_empty());
}

#[test]
fn show_file_whose_cues_share_an_identity_cannot_be_read() {
    let twice = "[[list.cue]]\nid = 1\nkind = \"memo\"\n".repeat(2);
    assert_eq!(
        read_list(&twice).err().as_deref(),
        Some("more than one cue has the id 1")
    );
}

#[test]
fn relative_sound_files_go_from_wherever_the_file_is() {
    let workspace = workspace(vec![
        Cue::new(CueId(1), "1", Kind::Audio, "Near")
            .file("sounds/near.ogg")
            .build(),
        Cue::new(CueId(2), "2", Kind::Audio, "Far")
            .file("/library/far.ogg")
            .build(),
    ])
    .directory("/show");
    let names = |show: &str, file: &str| show.contains(&format!("file = \"{file}\""));
    let elsewhere = workspace.show(Some(Path::new("/elsewhere")));
    assert!(names(&elsewhere, "../show/sounds/near.ogg"), "{elsewhere}");
    assert!(names(&elsewhere, "/library/far.ogg"), "{elsewhere}");
    let nowhere = workspace.show(None);
    assert!(names(&nowhere, "/show/sounds/near.ogg"), "{nowhere}");
}

#[test]
fn restoring_kept_lists_is_one_change_that_undo_takes_back() {
    let saved = storm();
    let kept = Workspace::new(vec![
        CueList::new("Main", vec![audio(1), audio(9)]),
        CueList::new("Preshow", Vec::new()),
    ]);
    let restored = saved.clone().apply(Command::Restore(kept.lists().to_vec()));
    assert_eq!(restored.lists().len(), 2);
    assert_eq!(restored.next_cue_id(), Some(CueId(10)));
    let undone = restored.apply(Command::Undo);
    assert_eq!(shown(&undone), shown(&saved));
    let redone = undone.apply(Command::Redo);
    assert_eq!(redone.lists().len(), 2);
}

#[test]
fn show_mode_keeps_the_lists_from_being_restored() {
    let locked = storm()
        .apply(Command::ShowMode(true))
        .apply(Command::Restore(Vec::new()));
    assert_eq!(ids(&locked), [1, 2]);
}

#[test]
fn show_always_holds_a_list() {
    assert_eq!(Workspace::default().lists().len(), 1);
    assert_eq!(Workspace::new(Vec::new()).lists().len(), 1);
    let emptied = storm().apply(Command::Restore(Vec::new()));
    assert_eq!(emptied.lists().len(), 1);
}

#[test]
fn show_file_holds_where_an_audio_cue_starts_and_ends_and_whether_it_loops() {
    let rain = workspace(vec![
        Cue::new(CueId(1), "1", Kind::Audio, "Rain")
            .file("rain.ogg")
            .start(1.5)
            .end(20.0)
            .looping(true)
            .build(),
    ])
    .directory("/show");
    let text = shown(&rain);
    assert!(text.contains("file = \"rain.ogg\"\nstart = 1.5\nend = 20.0\nloop = true\n"));
    let read = Workspace::from_show(&text, "/show").expect("the show reads back");
    assert_eq!(shown(&read), text);
    let cue = read.cue(CueId(1)).expect("the rain is there");
    assert_eq!((cue.start.value(), cue.end.time()), (1.5, Some(20.0)));
    assert!(cue.looping);
}

#[test]
fn show_file_with_a_level_that_no_cue_could_be_given_cannot_be_read() {
    let cue = |level: &str| format!("[[list.cue]]\nid = 1\nkind = \"audio\"\nlevel = {level}\n");
    assert!(read_list(&cue("60.0")).is_err());
    assert!(read_list(&cue("nan")).is_err());
    assert!(read_list(&cue("-6.0")).is_ok());
}

#[test]
fn show_whose_identities_have_run_out_adds_no_cue() {
    let full =
        read_list("[[list.cue]]\nid = 4294967295\nkind = \"memo\"\n").expect("the show reads");
    assert_eq!(full.next_cue_id(), None);
    let added = full.apply(add(0, Kind::Memo, Place::End));
    assert_eq!(ids(&added), [u32::MAX]);
}
