// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the video output following the runs of video cues: the
//! pictures it hands on, those it makes ready ahead, and the files whose
//! picture cannot play

use std::{
    fs,
    path::Path,
    thread,
    time::{Duration, Instant},
};

use super::{
    PATIENCE,
    movie::{movie, picture_number},
};
use crate::back::{
    Command, Cue, CueId, CueList, Kind, Load, Media, Picture, Video, Workspace, load_files,
};

/// Returns a workspace in the directory of the video at `path`, with one
/// list of one video cue that plays it, whose file has loaded
fn showing(path: &Path) -> Workspace {
    let cue = Cue::new(CueId(1), "1", Kind::Video, "Movie")
        .file("movie.mp4")
        .build();
    let directory = path.parent().expect("the video is in a directory");
    let workspace = Workspace::new(vec![CueList::new("Main", vec![cue])]).directory(directory);
    let loads = load_files(Load::all(&workspace, workspace.unloaded()));
    loads.into_iter().fold(workspace, Workspace::apply)
}

/// Syncs `video` with `workspace` until a picture comes due or the
/// workspace no longer runs the cue, and returns the workspace, along with
/// the picture
fn synced(video: &mut Video, workspace: Workspace) -> (Workspace, Option<(CueId, Picture)>) {
    synced_since(video, workspace, Instant::now())
}

/// Goes on with [`synced`], which started at `started`
fn synced_since(
    video: &mut Video,
    workspace: Workspace,
    started: Instant,
) -> (Workspace, Option<(CueId, Picture)>) {
    let (workspace, pictures) = video.sync(workspace);
    match pictures.into_iter().next() {
        None if started.elapsed() < PATIENCE && !workspace.active().is_empty() => {
            thread::sleep(Duration::from_millis(5));
            synced_since(video, workspace, started)
        }
        picture => (workspace, picture),
    }
}

#[test]
fn video_output_shows_the_pictures_of_a_run_and_lets_its_thread_go_once_it_stops() {
    let (_directory, path) = movie(15, 5);
    let mut video = Video::default();
    let playing = showing(&path).apply(Command::Go { list: 0 });
    let (playing, picture) = synced(&mut video, playing);
    let (cue, picture) = picture.expect("the first picture comes due");
    assert_eq!((cue, picture_number(&picture.pixels)), (CueId(1), 0));
    assert_eq!(video.runs.len(), 1);
    let (_, pictures) = video.sync(playing.apply(Command::Stop(CueId(1))));
    assert!(pictures.is_empty());
    assert!(video.runs.is_empty());
}

#[test]
fn picture_that_the_next_go_starts_is_made_ready_ahead_for_its_run_to_take() {
    let (_directory, path) = movie(15, 5);
    let mut video = Video::default();
    let (waiting, _) = video.sync(showing(&path));
    assert_eq!((video.ready.len(), video.runs.len()), (1, 0));
    let (playing, _) = video.sync(waiting.apply(Command::Go { list: 0 }));
    assert_eq!((video.ready.len(), video.runs.len()), (0, 1));
    // List played to its end leaves nothing to make ready
    let (_, picture) = synced(&mut video, playing);
    assert!(picture.is_some());
    assert!(video.ready.is_empty());
}

#[test]
fn run_whose_video_file_has_gone_since_it_loaded_ends_as_a_file_that_cannot_play() {
    let (_directory, path) = movie(15, 5);
    let loaded = showing(&path);
    fs::remove_file(&path).expect("the video can be removed");
    let mut video = Video::default();
    let (ended, _) = synced(&mut video, loaded.apply(Command::Go { list: 0 }));
    let cue = ended.cue(CueId(1)).expect("the cue is there");
    assert_eq!(cue.media, Media::Failed("file not found".to_owned()));
    assert!(ended.active().is_empty());
}

#[test]
fn video_file_without_sound_plays_its_picture_alone() {
    let (_directory, path) = movie(15, 5);
    let playing = showing(&path).apply(Command::Go { list: 0 });
    let active = playing.active_of(CueId(1)).expect("the cue runs");
    assert!(active.shows_picture());
    assert_eq!(active.sound(), None);
    assert_eq!(playing.problems().count(), 0);
}

#[test]
fn video_cue_whose_file_cannot_load_cannot_play() {
    let workspace = showing(Path::new("/nowhere/movie.mp4"));
    let cue = workspace.cue(CueId(1)).expect("the cue is there");
    assert_eq!(cue.media, Media::Failed("file not found".to_owned()));
    assert!(workspace.apply(Command::Go { list: 0 }).active().is_empty());
}
