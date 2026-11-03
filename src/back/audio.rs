// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Sound output and the loading of sound files, where the back end has
//! side effects
//!
//! [`Audio::sync`] runs after every change to the workspace and makes the
//! sound match it: it starts the audio cues that the workspace has
//! sounding, follows their level and pause state, and stops the sounds
//! whose cue has stopped.  The workspace stays the only record of what
//! should be heard, so a sound that cannot start goes back to it as a file
//! that cannot be played.  Ahead of that, a thread of its own makes ready
//! the sounds that runs may start next, so that GO waits on no file.
//! [`load_files`] reads the files that audio cues name, which can block, so
//! the interface runs it off its own thread.

mod preload;
#[cfg(test)]
mod tests;
mod voice;

use std::{
    collections::HashMap,
    io,
    path::{Path, PathBuf},
};

use itertools::{Either, Itertools};
use kira::{
    AudioManager, AudioManagerSettings, DefaultBackend,
    sound::{FromFileError, streaming::StreamingSoundData},
};
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use super::{Active, Clip, Command, Cue, CueId, Media, Problem, Workspace};
use preload::{Ask, Preloader, Wanted};
use voice::Voice;

/// Endings of the names of the sound files that the sound output plays,
/// which the features of kira in `Cargo.toml` decide
pub const SOUND_FILES: [&str; 4] = ["flac", "mp3", "ogg", "wav"];

/// Sound output for a workspace
pub struct Audio {
    /// Audio device, or the reason it could not be opened
    output: Result<AudioManager, String>,
    /// Sounds of the audio cues that are sounding
    voices: HashMap<CueId, Voice>,
    /// Sounds that runs may start next, made ready ahead
    preloader: Preloader,
}

impl Audio {
    /// Opens the default audio device.  Without one, the workspace still
    /// runs and files still load, but nothing is heard.
    pub fn open() -> Self {
        Self {
            output: AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())
                .map_err(|error| error.to_string()),
            voices: HashMap::new(),
            preloader: Preloader::new(),
        }
    }

    /// Returns why the audio device could not be opened, if it could not
    pub fn error(&self) -> Option<&str> {
        self.output.as_ref().err().map(String::as_str)
    }

    /// Makes the sound match the workspace, and returns the workspace with
    /// the files whose sound could not start.  The sounds that runs may
    /// start next are made ready once those that start now have taken
    /// theirs, unless there is no audio device to hear them on.
    pub fn sync(&mut self, workspace: Workspace) -> Workspace {
        self.preloader.collect();
        let failures = self.play(&workspace);
        if self.output.is_ok() {
            self.preloader.want(upcoming(&workspace));
        }
        failures.into_iter().fold(workspace, Workspace::apply)
    }

    /// Starts, follows, and stops sounds, so that each sounding audio cue
    /// plays its clip at the level and pause state of its run, and nothing
    /// else plays.  Returns the commands that record the sounds that could
    /// not start.
    fn play(&mut self, workspace: &Workspace) -> Vec<Command> {
        let Ok(manager) = &mut self.output else {
            return Vec::new();
        };
        let preloader = &mut self.preloader;
        let sounding: HashMap<CueId, (&Cue, &Active, Clip)> = workspace
            .active()
            .iter()
            .filter_map(|active| {
                let clip = active.sound()?;
                workspace
                    .cue(active.id)
                    .map(|cue| (cue.id, (cue, active, clip)))
            })
            .collect();
        // Sounds whose cue has stopped, or which GO has fired again
        let (mut kept, ended): (HashMap<_, _>, HashMap<_, _>) = std::mem::take(&mut self.voices)
            .into_iter()
            .partition(|(id, voice)| {
                sounding
                    .get(id)
                    .is_some_and(|(_, active, _)| active.run == voice.run)
            });
        ended.into_values().for_each(Voice::stop);
        let (voices, failures) = sounding
            .into_iter()
            .partition_map(|(id, (cue, active, clip))| {
                // Sound made ready ahead if there is one, or else the file
                // opened now
                let voice = kept.remove(&id).map_or_else(
                    || {
                        let path = workspace
                            .path(cue)
                            .ok_or_else(|| Problem::NoFile.to_string())?;
                        let wanted = Wanted { path, clip };
                        match preloader.take(id, &wanted) {
                            Some(Ok(preloaded)) => Voice::start(manager, preloaded, active),
                            // Sound that could not be made ready, whose
                            // file may have come back since
                            _ => Voice::open(manager, &wanted.path, active, clip),
                        }
                    },
                    Ok,
                );
                match voice {
                    Ok(voice) => Either::Left((id, voice.followed(active))),
                    Err(reason) => Either::Right(Command::Loaded {
                        cue: id,
                        file: cue.file.clone(),
                        media: Media::Failed(reason),
                    }),
                }
            });
        self.voices = voices;
        failures
    }
}

/// Returns the sounds that runs may start next, each of a cue and with the
/// level to make it ready at: those of the runs that GO has fired that have
/// yet to sound, then those of the cues that the next GO of each list
/// fires, apart from the cues that GO skips
fn upcoming(workspace: &Workspace) -> Vec<Ask> {
    let fired = workspace.active().iter().filter_map(|active| {
        let clip = active.pending()?;
        Some((workspace.cue(active.id)?, clip, active.level))
    });
    let next = workspace
        .lists()
        .iter()
        .flat_map(|list| workspace.next_go(list))
        .filter_map(|(cue, _)| Some((cue, cue.clip()?, cue.level)));
    fired
        .chain(next)
        .filter_map(|(cue, clip, level)| {
            let path = workspace.path(cue)?;
            Some(Ask {
                cue: cue.id,
                wanted: Wanted { path, clip },
                level,
            })
        })
        .unique_by(|ask| ask.cue)
        .collect()
}

/// File of an audio cue to load: the file as the cue names it, and where
/// it is
#[derive(Debug)]
pub struct Load {
    cue: CueId,
    file: String,
    path: PathBuf,
}

impl Load {
    /// Returns what loading the files that `cues` name takes, finding each
    /// file as `workspace` does
    pub fn all<'a>(workspace: &Workspace, cues: impl IntoIterator<Item = &'a Cue>) -> Vec<Self> {
        cues.into_iter()
            .filter_map(|cue| {
                Some(Self {
                    cue: cue.id,
                    file: cue.file.clone(),
                    path: workspace.path(cue)?,
                })
            })
            .collect()
    }
}

/// Reads the files of `loads`, and returns the commands that record what
/// reading found.  Each distinct file is read once, and the files are read
/// in parallel, which shortens opening a large show whose files are not
/// cached yet.  Reading can block on a slow drive.
pub fn load_files(loads: Vec<Load>) -> Vec<Command> {
    // Collected first, so that rayon reads no file or a single one on this
    // thread instead of waking its pool
    let paths: Vec<PathBuf> = loads
        .iter()
        .map(|load| &load.path)
        .unique()
        .cloned()
        .collect();
    let found: HashMap<PathBuf, Media> = paths
        .into_par_iter()
        .map(|path| {
            let media = probe(&path);
            (path, media)
        })
        .collect();
    loads
        .into_iter()
        .map(|load| Command::Loaded {
            media: found[&load.path].clone(),
            cue: load.cue,
            file: load.file,
        })
        .collect()
}

/// Reads how long a sound file plays, or why it cannot be played.  This
/// opens the file and reads its format without decoding the sound.
fn probe(file: &Path) -> Media {
    match StreamingSoundData::from_file(file) {
        Ok(data) => Media::Loaded {
            duration: data.duration().as_secs_f32(),
        },
        Err(error) => Media::Failed(reason(&error)),
    }
}

/// Describes why a sound file cannot be played
fn reason(error: &FromFileError) -> String {
    match error {
        FromFileError::IoError(error) if error.kind() == io::ErrorKind::NotFound => {
            "file not found".to_owned()
        }
        error => error.to_string(),
    }
}
