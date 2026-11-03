// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Show files, which hold the cue lists of a workspace as TOML
//!
//! A show file lists the cue lists of a show in order, each with its cues,
//! and each cue with the settings that its kind uses, leaving out those
//! that keep their defaults, apart from the level, the duration, and
//! whether a fade stops its target, which it holds for every cue whose kind
//! uses them.  Cues keep their identities, by which fade and stop cues name
//! their targets.  What playback and loading find, such as the cue on
//! standby, the cues that run, and how long each sound file plays, belongs
//! to the session rather than to the show, and the file leaves it out.

use std::path::{Path, PathBuf};

use itertools::Itertools;
use serde::{Deserialize, Serialize};

use super::Workspace;
use crate::back::{Advance, Cue, CueId, CueList, Kind, LEVELS, LOUDEST, SILENCE};

/// Show as its file holds it
#[derive(Serialize, Deserialize)]
struct ShowFile {
    #[serde(default, rename = "list")]
    lists: Vec<ListFile>,
}

/// Cue list as a show file holds it
#[derive(Serialize, Deserialize)]
struct ListFile {
    name: String,
    #[serde(default, rename = "cue", skip_serializing_if = "Vec::is_empty")]
    cues: Vec<CueFile>,
}

/// Cue as a show file holds it, without the settings that its kind does
/// not use, and without those that keep their defaults, apart from its
/// level, its duration, and whether a fade stops its target
#[derive(Serialize, Deserialize)]
struct CueFile {
    id: u32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    number: String,
    kind: Kind,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    file: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    start: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    end: Option<f32>,
    #[serde(default, rename = "loop", skip_serializing_if = "std::ops::Not::not")]
    looping: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    target: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pre_wait: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    duration: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    post_wait: Option<f32>,
    #[serde(default, skip_serializing_if = "is_manual")]
    advance: Advance,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    level: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    stop_target: Option<bool>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    notes: String,
}

impl Workspace {
    /// Reads a show from the text of its file, finding the sound files of
    /// its cues from `directory`.  Each list starts with standby on its
    /// first cue, and a file without lists holds one empty list, as every
    /// show does.  A file in which two cues share an identity cannot be
    /// read, nor can one with a level that no cue could be given, since a
    /// level typed wrong by hand, such as 60 for -60, would play at a
    /// thousand times the amplitude of its file.
    pub fn from_show(text: &str, directory: impl Into<PathBuf>) -> Result<Self, String> {
        let show: ShowFile = toml::from_str(text).map_err(|error| error.to_string())?;
        let workspace =
            Self::new(show.lists.into_iter().map(ListFile::list).collect()).directory(directory);
        let duplicate = workspace.cues().map(|cue| cue.id.0).duplicates().next();
        let unplayable = workspace
            .cues()
            .find(|cue| cue.kind.has_level() && !LEVELS.contains(&cue.level));
        match (duplicate, unplayable) {
            (Some(id), _) => Err(format!("more than one cue has the id {id}")),
            (None, Some(cue)) => Err(format!(
                "the cue with the id {} has the level {} dB, which is not between {SILENCE} and \
                 {LOUDEST} dB",
                cue.id.0, cue.level
            )),
            (None, None) => Ok(workspace),
        }
    }

    /// Returns the text of the show as a file in `directory` holds it, with
    /// the relative paths of sound files going from that directory, or as
    /// absolute paths without one
    pub fn show(&self, directory: Option<&Path>) -> String {
        let show = ShowFile {
            lists: self
                .lists
                .iter()
                .map(|list| ListFile {
                    name: list.name.clone(),
                    cues: list
                        .cues
                        .iter()
                        .map(|cue| CueFile::of(cue, self.written_file(cue, directory)))
                        .collect(),
                })
                .collect(),
        };
        toml::to_string(&show).expect("TOML holds every setting of a show")
    }

    /// Returns the sound file of `cue` as a file in `directory` names it:
    /// an absolute path as it is, and a relative one from `directory`
    /// rather than from the directory of the workspace, or as an absolute
    /// path without one
    fn written_file(&self, cue: &Cue, directory: Option<&Path>) -> String {
        let Some(name) = cue.file_name() else {
            return String::new();
        };
        if Path::new(name).is_absolute() || directory == Some(self.directory.as_path()) {
            return name.to_owned();
        }
        let absolute = self.directory.join(name);
        directory
            .and_then(|directory| pathdiff::diff_paths(&absolute, directory))
            .unwrap_or(absolute)
            .to_string_lossy()
            .into_owned()
    }
}

impl ListFile {
    fn list(self) -> CueList {
        CueList::new(self.name, self.cues.into_iter().map(CueFile::cue).collect())
    }
}

impl CueFile {
    /// Returns a cue as a file holds it, where it names its sound file as
    /// `file`
    fn of(cue: &Cue, file: String) -> Self {
        let set = |seconds: f32| Some(seconds).filter(|&seconds| seconds > 0.0);
        let audio = cue.kind == Kind::Audio;
        Self {
            id: cue.id.0,
            number: cue.number.clone(),
            kind: cue.kind,
            name: cue.name.clone(),
            file,
            start: set(cue.start.value()).filter(|_| audio),
            end: cue.end.time().filter(|_| audio),
            looping: audio && cue.looping,
            target: cue.target.map(|target| target.0),
            pre_wait: set(cue.pre_wait.value()),
            duration: cue.kind.has_duration().then(|| cue.duration.value()),
            post_wait: set(cue.post_wait.value()),
            advance: cue.advance,
            level: cue.kind.has_level().then_some(cue.level),
            stop_target: (cue.kind == Kind::Fade).then_some(cue.stop_target),
            notes: cue.notes.clone(),
        }
    }

    /// Returns the cue that the file holds, with the defaults of its kind
    /// for the settings that the file leaves out
    fn cue(self) -> Cue {
        Cue::new(CueId(self.id), self.number, self.kind, self.name)
            .file(self.file)
            .maybe_start(self.start)
            .maybe_end(self.end)
            .looping(self.looping)
            .maybe_target(self.target.map(CueId))
            .maybe_pre_wait(self.pre_wait)
            .maybe_duration(self.duration)
            .maybe_post_wait(self.post_wait)
            .advance(self.advance)
            .maybe_level(self.level)
            .maybe_stop_target(self.stop_target)
            .notes(self.notes)
            .build()
    }
}

/// Returns whether the next cue waits for GO, which a file leaves unsaid
fn is_manual(advance: &Advance) -> bool {
    *advance == Advance::Manual
}
