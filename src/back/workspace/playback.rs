// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Rules of playback: GO, the clock, fades, pauses, and panics

use super::{
    Workspace,
    active::{Effect, Fade},
};
use crate::back::{Active, Cue, CueId, CueList, SILENCE};

/// Length in seconds of the fade-out that a panic starts
pub(super) const PANIC_FADE: f64 = 1.0;

impl Workspace {
    /// Returns the cues that the next GO of `list` fires, each with its
    /// delay after the GO: the chain from standby, without the cues that
    /// cannot play
    pub fn next_go<'a>(&'a self, list: &'a CueList) -> impl Iterator<Item = (&'a Cue, f32)> {
        list.standby
            .into_iter()
            .flat_map(move |standby| list.chain(standby))
            .filter(move |(cue, _)| self.problem(cue).is_none())
    }

    /// Fires the standby cue of `list` together with its chain, then moves
    /// standby to the first cue after the chain.  Cues in the chain that
    /// cannot play are skipped, and cues that are already running restart.
    pub(super) fn go(self, list: usize) -> Self {
        let Some((fired, standby)) = self.lists.get(list).and_then(|cues| {
            let end = cues.chain(cues.standby?).last()?.0.id;
            let fired: Vec<Active> = self
                .next_go(cues)
                .zip(self.next_run..)
                .map(|((cue, delay), run)| Active::new(cue, run, self.next_run, delay.into()))
                .collect();
            Some((fired, cues.after(end)))
        }) else {
            return self;
        };
        let next_run = fired.last().map_or(self.next_run, |active| active.run + 1);
        let workspace = self.with_standby(list, standby).played_from(list);
        let active = workspace
            .active
            .into_iter()
            .filter(|running| fired.iter().all(|cue| cue.id != running.id))
            .chain(fired.iter().copied())
            .collect();
        Self {
            active,
            next_run,
            ..workspace
        }
        .tick(0.0)
    }

    /// Advances every running cue, begins the actions that are due, and
    /// drops the cues that have finished
    pub(super) fn tick(self, seconds: f64) -> Self {
        let active = self
            .active
            .into_iter()
            .map(|active| active.advanced(seconds))
            .collect();
        Self { active, ..self }.started().without_finished()
    }

    /// Fades out every playing audio cue and stops everything else.  A
    /// panic while an earlier one is still fading stops everything at once.
    pub(super) fn panic(self) -> Self {
        let fading = self
            .active
            .iter()
            .any(|active| active.fade.is_some_and(|fade| fade.by.is_none()));
        let fade = Fade {
            by: None,
            to: SILENCE,
            remaining: PANIC_FADE,
            then_stop: true,
        };
        let active = if fading {
            Vec::new()
        } else {
            self.active
                .iter()
                .filter(|active| !active.paused && active.sounding())
                .map(|active| Active {
                    fade: Some(fade),
                    ..*active
                })
                .collect()
        };
        Self { active, ..self }
    }

    pub(super) fn paused(self, paused: bool) -> Self {
        let active = self
            .active
            .into_iter()
            .map(|active| Active { paused, ..active })
            .collect();
        Self { active, ..self }
    }

    /// Begins the action of every cue whose pre-wait is over.  Fade cues
    /// start fading their target and stop cues stop theirs.  Audio cues need
    /// nothing here, because making sound is the job of the audio output.
    fn started(self) -> Self {
        let due: Vec<CueId> = self
            .active
            .iter()
            .filter(|active| active.due())
            .map(|active| active.id)
            .collect();
        let active = self.active.into_iter().map(Active::begun).collect();
        due.into_iter()
            .fold(Self { active, ..self }, Self::with_effect_of)
    }

    /// Carries out the effect of a fade or stop cue on its target, unless
    /// an effect earlier in the same tick has stopped the cue.  A fade acts
    /// only on a target that is sounding, takes over from a fade already on
    /// it only where [`Fade::yields_to`] allows, and starts as far along as
    /// the tick has passed the end of the pre-wait.
    fn with_effect_of(self, id: CueId) -> Self {
        let Some(fired) = self.active_of(id).copied() else {
            return self;
        };
        match fired.effect {
            Effect::Fade { target, fade } => self.with_active(target, |active| {
                if active.sounding() && active.fade.is_none_or(|current| current.yields_to(&fade)) {
                    Active {
                        fade: Some(fade),
                        ..active
                    }
                    .faded(fired.played())
                } else {
                    active
                }
            }),
            Effect::Stop(target) => self.stopped(target),
            Effect::Sound(_) | Effect::Nothing => self,
        }
    }

    /// Drops the cues that have finished.  A cue that a fade has faded out
    /// stops as a stopped cue does, which also cancels the rest of its
    /// chain that it has yet to start.
    fn without_finished(self) -> Self {
        let faded_out: Vec<CueId> = self
            .active
            .iter()
            .filter(|active| active.faded_out())
            .map(|active| active.id)
            .collect();
        let workspace = faded_out.into_iter().fold(self, Self::stopped);
        let active = workspace
            .active
            .into_iter()
            .filter(|active| !active.finished())
            .collect();
        Self {
            active,
            ..workspace
        }
    }
}
