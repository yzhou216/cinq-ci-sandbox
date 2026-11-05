// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! What a cue does, and what happens to the next cue once it has started

use serde::{Deserialize, Serialize};
use strum::{Display, VariantArray};

/// Kind of cue, named after what the cue does, which displays as its name
/// and goes by it in lower case in a show file
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Display, VariantArray, Serialize, Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// Plays a sound file
    #[default]
    Audio,
    /// Plays a video file: its picture in the stage window, with its sound
    Video,
    /// Changes the level of a playing audio or video cue over time
    Fade,
    /// Stops another cue
    Stop,
    /// Takes time and does nothing else, to pace a chain of cues
    Wait,
    /// Does nothing, as a note in the cue list
    Memo,
}

impl Kind {
    /// Returns whether the operator sets how long the action of this kind
    /// takes.  Audio and video cues take as long as the part of their file
    /// that they play, and the rest are instant.
    pub fn has_duration(self) -> bool {
        matches!(self, Kind::Fade | Kind::Wait)
    }

    /// Returns whether cues of this kind play a file: audio cues play its
    /// sound, and video cues its picture along with its sound
    pub fn plays_file(self) -> bool {
        matches!(self, Kind::Audio | Kind::Video)
    }

    /// Returns whether cues of this kind have a level: audio and video cues
    /// play at one, and fade cues fade their target to one
    pub(in crate::back) fn has_level(self) -> bool {
        self.plays_file() || self == Kind::Fade
    }
}

/// What happens to the next cue in the list once this cue has started,
/// which goes by its name in lower case in a show file
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, Display, VariantArray, Serialize, Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum Advance {
    /// Next cue waits for the operator to press GO
    #[default]
    #[strum(to_string = "wait for GO")]
    Manual,
    /// Next cue starts when this cue's post-wait has elapsed
    #[strum(to_string = "auto-continue")]
    Continue,
    /// Next cue starts when this cue's action ends
    #[strum(to_string = "auto-follow")]
    Follow,
}
