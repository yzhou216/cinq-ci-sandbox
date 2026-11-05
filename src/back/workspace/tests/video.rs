// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of video cues: their pictures fade to black with the fades that
//! stop them, fade cues act on them, and show files hold them as they hold
//! audio cues

use std::path::Path;

use super::*;

/// Returns a video cue that plays `movie.mp4`
fn video(id: u32) -> Cue {
    Cue::new(CueId(id), id.to_string(), Kind::Video, "Movie")
        .file("movie.mp4")
        .build()
}

/// Returns how opaque the picture of cue `id` is
fn opacity(workspace: &Workspace, id: u32) -> f32 {
    workspace
        .active_of(CueId(id))
        .expect("the cue runs")
        .opacity()
}

#[test]
fn picture_fades_to_black_with_a_fade_that_stops_its_cue() {
    let continuing = Cue {
        advance: Advance::Continue,
        ..video(1)
    };
    let workspace = workspace(vec![continuing, fade_out(2)])
        .apply(GO)
        .apply(Command::Tick(1.0));
    assert!(workspace.active_of(CueId(1)).unwrap().shows_picture());
    assert!((opacity(&workspace, 1) - 0.75).abs() < 1e-6);
    assert!(workspace.apply(Command::Tick(3.0)).active().is_empty());
}

#[test]
fn picture_stays_whole_under_a_fade_that_leaves_its_cue_playing() {
    let under = Cue::new(CueId(2), "2", Kind::Fade, "Under")
        .target(CueId(1))
        .level(-18.0)
        .stop_target(false)
        .build();
    let continuing = Cue {
        advance: Advance::Continue,
        ..video(1)
    };
    let workspace = workspace(vec![continuing, under])
        .apply(GO)
        .apply(Command::Tick(2.0));
    assert_eq!(opacity(&workspace, 1), 1.0);
}

#[test]
fn panic_fades_the_picture_out_with_the_sound() {
    let panicked = workspace(vec![video(1)])
        .apply(GO)
        .apply(Command::Panic)
        .apply(Command::Tick(PANIC_FADE / 4.0));
    assert!((opacity(&panicked, 1) - 0.75).abs() < 1e-6);
    let level = level(&panicked, 1).expect("the cue runs");
    assert!((level - SILENCE / 4.0).abs() < 1e-4);
}

#[test]
fn video_cue_without_sound_still_fades_out_and_stops() {
    let silent = loaded(
        1,
        "movie.mp4",
        Media::Loaded {
            duration: 10.0,
            sound: false,
        },
    );
    let continuing = Cue {
        advance: Advance::Continue,
        ..video(1)
    };
    let workspace = workspace(vec![continuing, fade_out(2)])
        .apply(silent)
        .apply(GO);
    assert_eq!(workspace.active_of(CueId(1)).unwrap().sound(), None);
    let halfway = workspace.apply(Command::Tick(2.0));
    assert!((opacity(&halfway, 1) - 0.5).abs() < 1e-6);
    assert!(halfway.apply(Command::Tick(2.0)).active().is_empty());
}

#[test]
fn new_fade_cue_targets_the_video_cue_it_follows() {
    let workspace = workspace(vec![video(1)]).apply(add(0, Kind::Fade, Place::After(CueId(1))));
    assert_eq!(workspace.lists()[0].cues[1].target, Some(CueId(1)));
}

#[test]
fn show_file_holds_a_video_cue_as_it_holds_an_audio_cue() {
    let looping = Cue {
        start: 1.5.into(),
        looping: true,
        ..video(1)
    };
    let workspace = workspace(vec![looping]).directory("/show");
    let text = workspace.show(Some(Path::new("/show")));
    let cue = "kind = \"video\"\nname = \"Movie\"\nfile = \"movie.mp4\"\nstart = 1.5\nloop = true\nlevel = 0.0\n";
    assert!(text.contains(cue), "{text}");
    let read = Workspace::from_show(&text, "/show").expect("the show reads back");
    assert_eq!(read.show(Some(Path::new("/show"))), text);
}
