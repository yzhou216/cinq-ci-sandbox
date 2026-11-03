// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Part of a sound file that an audio cue plays

/// Part of its sound file that an audio cue plays, from `start` to `end`
/// in seconds into the file, which comes before `end`.  A clip that loops
/// plays from its start again each time it reaches its end, until
/// something stops its cue.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Clip {
    pub(in crate::back) start: f64,
    pub(in crate::back) end: f64,
    pub(in crate::back) looping: bool,
}

impl Clip {
    /// Returns how long one pass through the clip lasts
    pub(in crate::back) fn length(&self) -> f64 {
        self.end - self.start
    }

    /// Returns where in the file the sound is once `played` seconds of the
    /// clip have played, counting every pass of a clip that loops
    pub(in crate::back) fn position(&self, played: f64) -> f64 {
        let into = if self.looping {
            played % self.length()
        } else {
            played
        };
        self.start + into
    }
}
