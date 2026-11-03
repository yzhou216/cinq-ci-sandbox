// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Rules of stopping: a stopped cue, the rest of its chain, and the fades
//! it was making

use super::Workspace;
use crate::back::{Active, CueId};

impl Workspace {
    /// Stops a cue.  When the next cue of its chain has yet to start, the
    /// rest of the chain goes too, since the stopped cue would have started
    /// it.
    pub(super) fn stopped(self, id: CueId) -> Self {
        let rest = self
            .active_of(id)
            .filter(|stopped| {
                self.active.iter().any(|next| {
                    next.chain == stopped.chain && next.run == stopped.run + 1 && next.waiting()
                })
            })
            .map(|stopped| (stopped.chain, stopped.run));
        let active = self
            .active
            .into_iter()
            .filter(|active| {
                !rest.is_some_and(|(chain, run)| active.chain == chain && active.run > run)
            })
            .collect();
        Self { active, ..self }.skipped(id)
    }

    /// Ends the run of a cue and leaves the rest of its chain to play, as GO
    /// skips a cue that cannot play.  Fades the cue was making stop where
    /// they are.
    pub(super) fn skipped(self, id: CueId) -> Self {
        let active = self
            .active
            .into_iter()
            .filter(|active| active.id != id)
            .map(|active| Active {
                fade: active.fade.filter(|fade| fade.by != Some(id)),
                ..active
            })
            .collect();
        Self { active, ..self }
    }
}
