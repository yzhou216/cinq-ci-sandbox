// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Cues that GO has fired, kept as bookkeeping without side effects, with
//! what they do once their pre-wait is over and the fades they make, which
//! [`effect`] describes, and what audio and video cues play, which
//! [`playing`] tells
//!
//! Time here is counted in seconds as `f64`, since a clock that adds a
//! tick forty times a second for an hour would drift by seconds in `f32`.

mod effect;
mod playing;

use crate::back::{Cue, CueId, Kind};
pub(in crate::back) use effect::{Effect, Fade};

/// Cue that GO has fired and that has not finished yet
///
/// A fired cue keeps its timing and its effect, such as the clip of an
/// audio or video cue, as its cue had them when GO fired it, so that an
/// edit changes the next run of a cue and not the one under way.  Only a
/// running audio or video cue follows an edit at once: it takes a new
/// level, and it stops when it gets a new file.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Active {
    pub id: CueId,
    /// Serial number of this firing, which tells a cue that GO fired again
    /// apart from one that kept running
    pub(in crate::back) run: u32,
    /// Serial number of the first cue that the same GO fired, which every
    /// cue of one chain shares
    pub(super) chain: u32,
    /// Seconds since GO, negative while the cue waits its turn in a chain
    pub(super) elapsed: f64,
    pub paused: bool,
    /// Level in dB an audio or video cue plays at
    pub level: f32,
    /// Fade under way on the level
    pub fade: Option<Fade>,
    /// Seconds of pre-wait
    pub(super) pre_wait: f64,
    /// Seconds that the action takes after the pre-wait, or that one pass
    /// of a clip that loops takes
    pub(super) length: f64,
    pub(super) effect: Effect,
    /// Whether the action has begun, which happens once the pre-wait is over
    pub(super) started: bool,
}

/// Where an active cue is in its lifetime.  An audio or video cue whose
/// clip loops stays [`Phase::Looping`] until something stops it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Phase {
    Scheduled { starts_in: f64 },
    PreWait { remaining: f64 },
    Running { remaining: f64 },
    Looping,
}

impl Active {
    /// Fires a cue `delay` seconds after GO, as run `run` of the chain that
    /// starts with run `chain`
    pub(super) fn new(cue: &Cue, run: u32, chain: u32, delay: f64) -> Self {
        let length = f64::from(cue.action());
        let effect = match (cue.kind, cue.target) {
            (Kind::Audio, _) => cue.clip().map_or(Effect::Nothing, Effect::Sound),
            (Kind::Video, _) => cue.clip().map_or(Effect::Nothing, |clip| Effect::Video {
                clip,
                sound: cue.sounds(),
            }),
            (Kind::Fade, Some(target)) => Effect::Fade {
                target,
                fade: Fade {
                    by: Some(cue.id),
                    to: cue.level,
                    remaining: length,
                    length,
                    then_stop: cue.stop_target,
                },
            },
            (Kind::Stop, Some(target)) => Effect::Stop(target),
            _ => Effect::Nothing,
        };
        Self {
            id: cue.id,
            run,
            chain,
            elapsed: -delay,
            paused: false,
            level: cue.level,
            fade: None,
            pre_wait: cue.pre_wait.value().into(),
            length,
            effect,
            started: false,
        }
    }

    pub fn phase(&self) -> Phase {
        match self.elapsed {
            elapsed if elapsed < 0.0 => Phase::Scheduled {
                starts_in: -elapsed,
            },
            elapsed if elapsed < self.pre_wait => Phase::PreWait {
                remaining: self.pre_wait - elapsed,
            },
            _ if self.loops() => Phase::Looping,
            elapsed => Phase::Running {
                remaining: (self.pre_wait + self.length - elapsed).max(0.0),
            },
        }
    }

    /// Returns how much of the action has played, from zero to one, which
    /// for a clip that loops is how much of the pass under way has played
    pub fn progress(&self) -> f32 {
        let progress = match self.phase() {
            Phase::Scheduled { .. } | Phase::PreWait { .. } => 0.0,
            _ if self.length <= 0.0 => 1.0,
            Phase::Running { remaining } => 1.0 - remaining / self.length,
            Phase::Looping => self.played() % self.length / self.length,
        };
        progress as f32
    }

    /// Returns how many seconds of the action have passed, which tells
    /// where in its clip the sound and the picture of the cue should be
    pub(in crate::back) fn played(&self) -> f64 {
        (self.elapsed - self.pre_wait).max(0.0)
    }

    /// Returns whether the cue still waits for an earlier cue of its chain
    pub(super) fn waiting(&self) -> bool {
        self.elapsed < 0.0
    }

    /// Returns whether the action is due to begin, which it is once the
    /// pre-wait is over
    pub(super) fn due(&self) -> bool {
        !self.started && self.elapsed >= self.pre_wait
    }

    /// Returns the cue with its action begun if it is due to begin
    pub(super) fn begun(self) -> Self {
        Self {
            started: self.started || self.due(),
            ..self
        }
    }

    /// Returns whether a fade that stops the cue has ended
    pub(super) fn faded_out(&self) -> bool {
        self.fade
            .is_some_and(|fade| fade.then_stop && fade.remaining <= 0.0)
    }

    /// Returns whether the action is over, which that of a cue that loops
    /// never is
    pub(super) fn finished(&self) -> bool {
        !self.loops() && self.elapsed >= self.pre_wait + self.length
    }

    /// Returns the cue `seconds` later, or unchanged while it is paused
    pub(super) fn advanced(self, seconds: f64) -> Self {
        if self.paused {
            self
        } else {
            Self {
                elapsed: self.elapsed + seconds,
                ..self
            }
            .faded(seconds)
        }
    }

    /// Moves the level `seconds` along its fade.  Each step covers the share
    /// of the remaining change that the step is of the remaining time, so
    /// the level moves in a straight line in dB.  A fade that leaves the cue
    /// running is dropped once it ends.
    pub(super) fn faded(self, seconds: f64) -> Self {
        match self.fade {
            Some(fade) if fade.remaining > seconds => Self {
                level: self.level + (fade.to - self.level) * (seconds / fade.remaining) as f32,
                fade: Some(Fade {
                    remaining: fade.remaining - seconds,
                    ..fade
                }),
                ..self
            },
            Some(fade) => Self {
                level: fade.to,
                fade: fade.then_stop.then_some(Fade {
                    remaining: 0.0,
                    ..fade
                }),
                ..self
            },
            None => self,
        }
    }
}
