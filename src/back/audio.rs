// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Sound output, where the back end has side effects
//!
//! [`Audio::sync`] runs after every change to the workspace and makes the
//! sound match it: it starts the audio and video cues that the workspace
//! has sounding, follows their level and pause state, and stops the sounds
//! whose cue has stopped.  The workspace stays the only record of what
//! should be heard, so a sound that cannot start goes back to it as a file
//! that cannot be played.  Ahead of that, a thread of its own makes ready
//! the sounds that runs may start next, so that GO waits on no file.  The
//! sound of a video file plays as that of a sound file does, from the
//! file's sound track.

mod preload;
#[cfg(test)]
mod tests;
mod voice;

use std::{collections::HashMap, io, path::Path};

use itertools::Itertools;
use kira::{
    AudioManager, AudioManagerSettings, DefaultBackend,
    sound::{FromFileError, streaming::StreamingSoundData},
};

use super::{Active, Clip, Command, Cue, CueId, Media, Problem, Workspace};
use preload::{Ask, Preloader, Wanted};
use voice::Voice;

/// Endings of the names of the sound files that the sound output plays,
/// which the features of kira in `Cargo.toml` decide
pub const SOUND_FILES: [&str; 5] = ["flac", "m4a", "mp3", "ogg", "wav"];

/// Sound output for a workspace
pub struct Audio {
    /// Audio device, or the reason it could not be opened
    output: Result<AudioManager, String>,
    /// Sounds of the audio and video cues that are sounding
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

    /// Starts, follows, and stops sounds, so that each sounding audio or
    /// video cue plays its clip at the level and pause state of its run,
    /// and nothing else plays.  Returns the commands that record the sounds
    /// that could not start.
    fn play(&mut self, workspace: &Workspace) -> Vec<Command> {
        let Ok(manager) = &mut self.output else {
            return Vec::new();
        };
        let preloader = &mut self.preloader;
        let sounding: HashMap<CueId, (&Cue, &Active, Clip)> = workspace
            .active()
            .iter()
            .filter_map(|active| {
                let cue = workspace.cue(active.id)?;
                Some((cue.id, (cue, active, active.sound()?)))
            })
            .collect();
        // Voices that play on, and those whose cue has stopped or which GO
        // has fired again
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
            .map(|(id, (cue, active, clip))| {
                // Voice that plays already, or else the sound made ready
                // ahead if there is one, or else the file opened now
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
                voice
                    .map(|voice| (id, voice.followed(active)))
                    .map_err(|reason| Command::Loaded {
                        cue: id,
                        file: cue.file.clone(),
                        media: Media::Failed(reason),
                    })
            })
            .partition_result();
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
        .filter(|(cue, _)| cue.sounds())
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

/// Reads how long the sound of a file plays, or why it cannot be played.
/// This opens the file and reads its format without decoding the sound.
pub(super) fn probe(file: &Path) -> Media {
    match StreamingSoundData::from_file(file) {
        Ok(data) => Media::Loaded {
            duration: data.duration().as_secs_f32(),
            sound: true,
        },
        Err(error) => Media::Failed(reason(error)),
    }
}

/// Describes why the sound of a file cannot be played
fn reason(error: FromFileError) -> String {
    match error {
        FromFileError::IoError(error) if error.kind() == io::ErrorKind::NotFound => {
            "file not found".to_owned()
        }
        error => error.to_string(),
    }
}
