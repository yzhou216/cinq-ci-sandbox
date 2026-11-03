// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! What fired cues do once their pre-wait is over, and the fades they make

use crate::back::{Clip, CueId};

/// Change to the level of an audio cue that a fade cue or a panic makes
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fade {
    /// Fade cue making the change, or `None` for a panic
    pub(in crate::back) by: Option<CueId>,
    /// Level in dB the fade ends at
    pub(in crate::back) to: f32,
    /// Seconds until the fade ends
    pub(in crate::back) remaining: f64,
    /// Whether the faded cue stops once the fade ends
    pub(in crate::back) then_stop: bool,
}

impl Fade {
    /// Returns whether a newer fade may take over from this one.  Nothing
    /// takes over from a panic, and only a fade that stops the cue takes
    /// over from another that stops it, so that no fade brings back a cue
    /// on its way out.
    pub(in crate::back) fn yields_to(&self, newer: &Fade) -> bool {
        self.by.is_some() && (newer.then_stop || !self.then_stop)
    }
}

/// What a fired cue does once its pre-wait is over
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::back) enum Effect {
    /// Plays a clip of the sound file of an audio cue
    Sound(Clip),
    /// Makes a fade on the level of a cue
    Fade { target: CueId, fade: Fade },
    /// Stops a cue
    Stop(CueId),
    /// Takes time and does nothing else, as wait and memo cues do
    Nothing,
}
