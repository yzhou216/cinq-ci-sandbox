// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Why cues cannot play, and the files of cues that have yet to load or
//! that could not be played

use super::Workspace;
use crate::back::{Cue, Kind, Media, Problem};

impl Workspace {
    /// Returns the reason a cue cannot play, if there is one
    pub fn problem<'a>(&self, cue: &'a Cue) -> Option<Problem<'a>> {
        match (cue.kind, &cue.media) {
            (Kind::Audio | Kind::Video, media) => match media {
                _ if cue.file_name().is_none() => Some(Problem::NoFile),
                Media::Unloaded => Some(Problem::NotLoaded),
                Media::Failed(reason) => Some(Problem::LoadFailed(reason)),
                Media::Loaded { .. } if cue.clip().is_none() => Some(Problem::EmptyClip),
                Media::Loaded { .. } => None,
            },
            (Kind::Fade | Kind::Stop, _) => match self.target(cue) {
                Some(target) if cue.can_target(target) => None,
                _ => Some(Problem::NoTarget),
            },
            (Kind::Wait | Kind::Memo, _) => None,
        }
    }

    /// Returns every cue that cannot play, with the reason
    pub fn problems(&self) -> impl Iterator<Item = (&Cue, Problem<'_>)> {
        self.cues()
            .filter_map(|cue| self.problem(cue).map(|problem| (cue, problem)))
    }

    /// Returns the audio and video cues whose file has not been loaded
    pub fn unloaded(&self) -> impl Iterator<Item = &Cue> {
        self.files_where(|media| *media == Media::Unloaded)
    }

    /// Returns the audio and video cues whose file could not be played
    pub fn failed(&self) -> impl Iterator<Item = &Cue> {
        self.files_where(|media| matches!(media, Media::Failed(_)))
    }

    /// Returns the audio and video cues that name a file and whose media
    /// passes `test`
    fn files_where(&self, test: fn(&Media) -> bool) -> impl Iterator<Item = &Cue> {
        self.cues().filter(move |cue| {
            cue.kind.plays_file() && test(&cue.media) && cue.file_name().is_some()
        })
    }
}
