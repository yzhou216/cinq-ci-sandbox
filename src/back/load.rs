// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Loading the files that audio and video cues name, which reads them
//!
//! [`load_files`] finds how long each file plays, or why it cannot be
//! played, without decoding it: the sound output reads sound files, and the
//! video output reads video files, along with their sound tracks, which the
//! sound output then reads in turn.  Reading can block on a slow drive, so
//! the interface runs it off its own thread.

#[cfg(test)]
mod tests;

use std::{collections::HashMap, path::PathBuf};

use itertools::Itertools;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use super::{Command, Cue, CueId, Kind, Media, Workspace, audio, video};

/// File of an audio or video cue to load: the file as the cue names it,
/// where it is, and the kind of cue that plays it
#[derive(Debug)]
pub struct Load {
    cue: CueId,
    file: String,
    path: PathBuf,
    kind: Kind,
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
                    kind: cue.kind,
                })
            })
            .collect()
    }
}

/// Reads the files of `loads`, and returns the commands that record what
/// reading found.  Each distinct file is read once for each kind of cue
/// that plays it, and the files are read in parallel, which shortens
/// opening a large show whose files are not cached yet.  Reading can block
/// on a slow drive.
pub fn load_files(loads: Vec<Load>) -> Vec<Command> {
    // Collected first, so that rayon reads no file or a single one on this
    // thread instead of waking its pool
    let files: Vec<(PathBuf, Kind)> = loads
        .iter()
        .map(|load| (load.path.clone(), load.kind))
        .unique()
        .collect();
    let found: HashMap<&(PathBuf, Kind), Media> =
        files.par_iter().map(|file| (file, probe(file))).collect();
    loads
        .into_iter()
        .map(|load| Command::Loaded {
            media: found[&(load.path, load.kind)].clone(),
            cue: load.cue,
            file: load.file,
        })
        .collect()
}

/// Reads how long a file plays for the kind of cue that plays it, or why
/// it cannot be played.  The sound track of a video file has to play as
/// well, since the cue plays it along with the picture, while a video file
/// without one plays its picture alone.
fn probe((path, kind): &(PathBuf, Kind)) -> Media {
    match kind {
        Kind::Video => match video::probe(path) {
            Media::Loaded {
                duration,
                sound: true,
            } => match audio::probe(path) {
                Media::Failed(reason) => Media::Failed(format!("its sound cannot play: {reason}")),
                _ => Media::Loaded {
                    duration,
                    sound: true,
                },
            },
            media => media,
        },
        _ => audio::probe(path),
    }
}
