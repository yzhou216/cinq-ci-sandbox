// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! What a fired audio or video cue plays once its pre-wait is over: the
//! clip of its file, with its sound, its picture, or both, and how opaque
//! its picture is

use super::{Active, Effect, Phase};
use crate::back::Clip;

impl Active {
    /// Returns whether the action is under way, paused or not, which it is
    /// once the pre-wait is over
    fn under_way(&self) -> bool {
        matches!(self.phase(), Phase::Running { .. } | Phase::Looping)
    }

    /// Returns the clip of an audio or video cue while it has a sound,
    /// paused or not, which it has once its pre-wait is over
    pub(in crate::back) fn sound(&self) -> Option<Clip> {
        self.effect.sound().filter(|_| self.under_way())
    }

    /// Returns the clip of an audio or video cue while it has yet to sound,
    /// waiting for its turn in a chain or for its pre-wait to end
    pub(in crate::back) fn pending(&self) -> Option<Clip> {
        self.effect.sound().filter(|_| !self.under_way())
    }

    /// Returns the clip of a video cue from the moment GO fires it, whose
    /// picture shows once the pre-wait is over
    pub(in crate::back) fn video(&self) -> Option<Clip> {
        match self.effect {
            Effect::Video { clip, .. } => Some(clip),
            _ => None,
        }
    }

    /// Returns whether the cue is a video cue whose picture shows, paused
    /// or not, which it does once its pre-wait is over
    pub fn shows_picture(&self) -> bool {
        self.video().is_some() && self.under_way()
    }

    /// Returns whether the cue plays the clip of an audio or video cue,
    /// with sound or without, paused or not, which it does once its
    /// pre-wait is over
    pub(in crate::back::workspace) fn playing(&self) -> bool {
        self.effect.clip().is_some() && self.under_way()
    }

    /// Returns how opaque the picture of a video cue is, from none at zero
    /// to whole at one: whole, apart from while a fade that stops the cue
    /// runs, as that of a panic does, when it falls with the progress of
    /// the fade, so that the picture fades to black along with the sound
    pub fn opacity(&self) -> f32 {
        self.fade
            .filter(|fade| fade.then_stop)
            .map_or(1.0, |fade| fade.left() as f32)
    }

    /// Returns whether the action goes on until something stops the cue,
    /// as the clip of an audio or video cue that loops does
    pub(super) fn loops(&self) -> bool {
        self.effect.clip().is_some_and(|clip| clip.looping)
    }
}
