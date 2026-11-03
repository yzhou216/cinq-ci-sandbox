// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of loading the files of audio cues, and of making their sounds
//! ready ahead, which need no audio device

use std::{
    fs,
    path::{Path, PathBuf},
    thread,
    time::Duration,
};

use tempfile::TempDir;

use super::{
    Load, load_files,
    preload::{Ask, Preloaded, Preloader, Wanted},
    upcoming,
};
use crate::back::{Advance, Clip, Command, Cue, CueId, CueList, Kind, Media, Workspace};

/// Returns an audio cue that plays `sound.wav`
fn audio(id: u32) -> Cue {
    Cue::new(CueId(id), id.to_string(), Kind::Audio, "Sound")
        .file("sound.wav")
        .build()
}

/// Returns a workspace in `/show` with one list of `cues`, every file of
/// which has loaded as ten seconds of sound
fn loaded(cues: Vec<Cue>) -> Workspace {
    let workspace = Workspace::new(vec![CueList::new("Main", cues)]).directory("/show");
    let loads: Vec<Command> = workspace
        .unloaded()
        .map(|cue| Command::Loaded {
            cue: cue.id,
            file: cue.file.clone(),
            media: Media::Loaded { duration: 10.0 },
        })
        .collect();
    loads.into_iter().fold(workspace, Workspace::apply)
}

/// Returns the cues of the sounds that runs may start next, in order
fn upcoming_cues(workspace: &Workspace) -> Vec<u32> {
    upcoming(workspace)
        .into_iter()
        .map(|ask| ask.cue.0)
        .collect()
}

/// Returns the ask for the sound of cue 1 that `wanted` describes, at no
/// change of level
fn ask(wanted: &Wanted) -> Ask {
    Ask {
        cue: CueId(1),
        wanted: wanted.clone(),
        level: 0.0,
    }
}

/// Returns a tenth of a second of silence as a WAV file: mono, in 16 bits,
/// at 8000 frames a second
fn silence() -> Vec<u8> {
    let data: u32 = 1600;
    [
        b"RIFF".as_slice(),
        &(36 + data).to_le_bytes(),
        b"WAVE",
        b"fmt ",
        &16_u32.to_le_bytes(),
        &1_u16.to_le_bytes(),
        &1_u16.to_le_bytes(),
        &8000_u32.to_le_bytes(),
        &16000_u32.to_le_bytes(),
        &2_u16.to_le_bytes(),
        &16_u16.to_le_bytes(),
        b"data",
        &data.to_le_bytes(),
        &vec![0; 1600],
    ]
    .concat()
}

/// Returns the whole of a tenth of a second of sound in `path` as wanted
fn whole(path: &Path) -> Wanted {
    Wanted {
        path: path.to_owned(),
        clip: Clip {
            start: 0.0,
            end: 0.1,
            looping: false,
        },
    }
}

/// Returns a temporary directory that holds a tenth of a second of silence,
/// with the whole of that sound as wanted
fn silent_file() -> (TempDir, Wanted) {
    let directory = TempDir::new().expect("a temporary directory can be made");
    let file = directory.path().join("silence.wav");
    fs::write(&file, silence()).expect("the file can be written");
    let wanted = whole(&file);
    (directory, wanted)
}

/// Returns what `preloader` made for `cue`, once its thread has made it,
/// waiting five seconds at most
fn made(
    preloader: &mut Preloader,
    cue: CueId,
    wanted: &Wanted,
) -> Option<Result<Preloaded, String>> {
    (0..500).find_map(|_| {
        preloader.collect();
        let made = preloader.take(cue, wanted);
        if made.is_none() {
            thread::sleep(Duration::from_millis(10));
        }
        made
    })
}

#[test]
fn every_cue_that_names_a_missing_file_learns_that_it_is_not_found() {
    let named = |id| {
        Cue::new(CueId(id), id.to_string(), Kind::Audio, "Sound")
            .file("gone.ogg")
            .build()
    };
    let workspace =
        Workspace::new(vec![CueList::new("Main", vec![named(1), named(2)])]).directory("/nowhere");
    let found = load_files(Load::all(&workspace, workspace.unloaded()));
    let not_found = |id| {
        found.iter().any(|command| {
            matches!(command, Command::Loaded { cue, media: Media::Failed(reason), .. }
                if *cue == CueId(id) && reason == "file not found")
        })
    };
    assert!(not_found(1) && not_found(2), "{found:?}");
}

#[test]
fn sounds_made_ready_are_those_of_the_next_go_and_of_runs_yet_to_sound() {
    let continuing = Cue {
        advance: Advance::Continue,
        post_wait: 2.0.into(),
        ..audio(1)
    };
    let workspace = loaded(vec![continuing, audio(2), audio(3)]);
    assert_eq!(upcoming_cues(&workspace), [1, 2]);
    let fired = workspace.apply(Command::Go { list: 0 });
    assert_eq!(upcoming_cues(&fired), [2, 3]);
    let wanted = upcoming(&fired).remove(0).wanted;
    assert_eq!(wanted.path, PathBuf::from("/show/sound.wav"));
}

#[test]
fn sound_wanted_is_made_ready_on_a_thread_of_its_own() {
    let (_directory, wanted) = silent_file();
    let mut preloader = Preloader::new();
    preloader.want(vec![ask(&wanted)]);
    assert!(matches!(
        made(&mut preloader, CueId(1), &wanted),
        Some(Ok(_))
    ));
}

#[test]
fn sound_that_cannot_be_made_ready_tells_why() {
    let wanted = whole(Path::new("/nowhere/gone.wav"));
    let mut preloader = Preloader::new();
    preloader.want(vec![ask(&wanted)]);
    let made = made(&mut preloader, CueId(1), &wanted);
    assert!(matches!(&made, Some(Err(reason)) if reason == "file not found"));
}

#[test]
fn sound_no_longer_wanted_is_let_go() {
    let (_directory, wanted) = silent_file();
    let mut preloader = Preloader::new();
    preloader.want(vec![ask(&wanted)]);
    preloader.want(Vec::new());
    thread::sleep(Duration::from_millis(200));
    preloader.collect();
    assert!(preloader.take(CueId(1), &wanted).is_none());
}
