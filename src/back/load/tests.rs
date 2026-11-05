// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of loading the files of audio and video cues, which read each file
//! as the kind of its cue plays it

use std::path::Path;

use super::{Load, load_files};
use crate::back::{
    Command, Cue, CueId, CueList, Kind, Media, Workspace, video::tests::movie::movie,
};

/// Returns a cue of `kind` that plays `file`
fn playing(id: u32, kind: Kind, file: &str) -> Cue {
    Cue::new(CueId(id), id.to_string(), kind, "Cue")
        .file(file)
        .build()
}

/// Returns what loading the files of `cues` in `directory` finds, by cue
fn found(cues: Vec<Cue>, directory: &Path) -> Vec<(u32, Media)> {
    let workspace = Workspace::new(vec![CueList::new("Main", cues)]).directory(directory);
    load_files(Load::all(&workspace, workspace.unloaded()))
        .into_iter()
        .filter_map(|command| match command {
            Command::Loaded { cue, media, .. } => Some((cue.0, media)),
            _ => None,
        })
        .collect()
}

#[test]
fn every_cue_that_names_a_missing_file_learns_that_it_is_not_found() {
    let cues = vec![
        playing(1, Kind::Audio, "sound.wav"),
        playing(2, Kind::Audio, "sound.wav"),
        playing(3, Kind::Video, "movie.mp4"),
    ];
    let not_found = Media::Failed("file not found".to_owned());
    let expected: Vec<(u32, Media)> = (1..=3).map(|cue| (cue, not_found.clone())).collect();
    assert_eq!(found(cues, Path::new("/nowhere")), expected);
}

#[test]
fn video_file_loads_as_a_picture_for_a_video_cue_and_as_no_sound_for_an_audio_cue() {
    let (directory, _) = movie(15, 5);
    let cues = vec![
        playing(1, Kind::Video, "movie.mp4"),
        playing(2, Kind::Audio, "movie.mp4"),
    ];
    let found = found(cues, directory.path());
    assert_eq!(
        found[0],
        (
            1,
            Media::Loaded {
                duration: 0.5,
                sound: false
            }
        )
    );
    assert!(matches!(found[1], (2, Media::Failed(_))), "{found:?}");
}
