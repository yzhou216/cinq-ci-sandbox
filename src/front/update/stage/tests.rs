// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the stage window: when it opens, what closing it and closing
//! the main window do, and the pictures it keeps

use iced::window;

use super::Stage;
use crate::{
    back::{Command, Cue, CueId, CueList, Kind, Media, Picture, Workspace},
    front::{App, Message, view},
};

/// Returns the state of the interface with a show of `kind` cues, whose
/// files have loaded as ten seconds of picture without sound
fn show(kind: Kind) -> App {
    let cue = Cue::new(CueId(1), "1", kind, "Cue")
        .file("movie.mp4")
        .build();
    let loaded = Command::Loaded {
        cue: CueId(1),
        file: "movie.mp4".to_owned(),
        media: Media::Loaded {
            duration: 10.0,
            sound: false,
        },
    };
    let workspace = Workspace::new(vec![CueList::new("Main", vec![cue])]).apply(loaded);
    App {
        workspace,
        main: Some(window::Id::unique()),
        ..App::default()
    }
}

/// Returns a picture of one black pixel
fn picture() -> Picture {
    Picture {
        width: 1,
        height: 1,
        pixels: vec![0, 0, 0, 255],
    }
}

#[test]
fn stage_window_opens_once_the_show_holds_a_video_cue() {
    let (audio, opening) = show(Kind::Audio).staged();
    assert_eq!((audio.stage.window(), opening.units()), (None, 0));
    let (video, opening) = show(Kind::Video).staged();
    assert!(video.stage.window().is_some());
    assert_eq!(opening.units(), 1);
    let (again, opening) = video.staged();
    assert!(again.stage.window().is_some());
    assert_eq!(opening.units(), 0);
}

#[test]
fn closed_stage_window_stays_closed_until_a_video_cue_fires() {
    let (open, _) = show(Kind::Video).staged();
    let stage = open.stage.window().expect("the stage window opens");
    let (closed, quitting) = open.closed(stage);
    assert_eq!(quitting.units(), 0);
    let (closed, opening) = closed.staged();
    assert_eq!((closed.stage.window(), opening.units()), (None, 0));
    let (gone, _) = closed.handled(Message::Go);
    let (reopened, opening) = gone.staged();
    assert!(reopened.stage.window().is_some());
    assert_eq!(opening.units(), 1);
}

#[test]
fn closing_the_main_window_quits() {
    let app = show(Kind::Video);
    let main = app.main.expect("the main window opens");
    let (_, quitting) = app.closed(main);
    assert_eq!(quitting.units(), 1);
}

#[test]
fn stage_keeps_the_newest_picture_of_each_video_cue_until_it_stops() {
    let (playing, _) = show(Kind::Video).handled(Message::Go);
    let stage = Stage::default().with_pictures(&playing.workspace, vec![(CueId(1), picture())]);
    assert!(stage.picture(CueId(1)).is_some());
    let kept = stage.with_pictures(&playing.workspace, Vec::new());
    assert!(kept.picture(CueId(1)).is_some());
    let (stopped, _) = playing.handled(Message::Stop(CueId(1)));
    let gone = kept.with_pictures(&stopped.workspace, Vec::new());
    assert!(gone.picture(CueId(1)).is_none());
}

#[test]
fn stage_draws_with_and_without_pictures() {
    let (playing, _) = show(Kind::Video).handled(Message::Go);
    let _ = view::stage::view(&playing);
    let stage = Stage::default().with_pictures(&playing.workspace, vec![(CueId(1), picture())]);
    let _ = view::stage::view(&App { stage, ..playing });
}
