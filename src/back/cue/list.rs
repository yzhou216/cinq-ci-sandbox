// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Cue lists, each with its own standby position

use crate::back::{
    Cue, CueId, Edit, Kind, Media, Place,
    items::{inserted, replaced_where},
};

/// Named list of cues with its own standby position
#[derive(Debug, Clone)]
pub struct CueList {
    pub name: String,
    pub cues: Vec<Cue>,
    /// Cue the next GO fires, or `None` past the end of the list
    pub standby: Option<CueId>,
}

impl CueList {
    /// Creates a list with standby on its first cue
    pub fn new(name: impl Into<String>, cues: Vec<Cue>) -> Self {
        let standby = cues.first().map(|cue| cue.id);
        Self {
            name: name.into(),
            cues,
            standby,
        }
    }

    pub fn position(&self, id: CueId) -> Option<usize> {
        self.cues.iter().position(|cue| cue.id == id)
    }

    pub(in crate::back) fn after(&self, id: CueId) -> Option<CueId> {
        self.position(id)
            .and_then(|index| self.cues.get(index + 1))
            .map(|cue| cue.id)
    }

    /// Returns the cues one GO on `start` fires, each paired with its delay
    /// after the GO.  The chain follows auto-continue and auto-follow until
    /// it reaches a cue that waits for GO.
    pub fn chain(&self, start: CueId) -> impl Iterator<Item = (&Cue, f32)> {
        let from = self.position(start).unwrap_or(self.cues.len());
        self.cues[from..].iter().scan(Some(0.0), |next, cue| {
            let delay = (*next)?;
            *next = cue.next_delay().map(|gap| delay + gap);
            Some((cue, delay))
        })
    }

    /// Moves standby to a cue, treating a cue of another list as the end
    pub(in crate::back) fn with_standby(self, standby: Option<CueId>) -> Self {
        let standby = standby.filter(|&id| self.position(id).is_some());
        Self { standby, ..self }
    }

    /// Inserts a cue with a number and a name at a place in the list.  As
    /// in QLab, a new fade or stop cue targets the cue it follows when it
    /// can act on that cue.  A cue that goes at the end of a list whose
    /// standby waits past the end goes on standby, so that GO fires the
    /// first cue of a new list, and the cue added to a list played to its
    /// end.
    pub(in crate::back) fn with_new_cue(
        self,
        id: CueId,
        number: String,
        kind: Kind,
        at: Place,
    ) -> Self {
        let index = match at {
            Place::After(cue) => self.position(cue).map(|index| index + 1),
            Place::Before(cue) => self.position(cue),
            Place::End => None,
        }
        .unwrap_or(self.cues.len());
        let cue = Cue::new(id, number, kind, Cue::new_name(kind)).build();
        let target = index
            .checked_sub(1)
            .and_then(|before| self.cues.get(before))
            .filter(|before| cue.can_target(before))
            .map(|before| before.id);
        let standby = match self.standby {
            None if index == self.cues.len() => Some(id),
            standby => standby,
        };
        Self {
            cues: inserted(self.cues, index, Cue { target, ..cue }),
            standby,
            ..self
        }
    }

    /// Removes a cue.  Standby on that cue moves on to the next cue, or
    /// past the end after the last cue, so that GO never goes back to a cue
    /// that has played.
    pub(in crate::back) fn without(self, id: CueId) -> Self {
        let standby = if self.standby == Some(id) {
            self.after(id)
        } else {
            self.standby
        };
        let cues = self.cues.into_iter().filter(|cue| cue.id != id).collect();
        Self {
            cues,
            standby,
            ..self
        }
    }

    /// Moves a cue to an index, or to the end when the list is shorter,
    /// leaving standby where it is
    pub(in crate::back) fn with_moved(self, id: CueId, to: usize) -> Self {
        let (moving, rest): (Vec<Cue>, Vec<Cue>) =
            self.cues.into_iter().partition(|cue| cue.id == id);
        moving
            .into_iter()
            .fold(Self { cues: rest, ..self }, |list, cue| {
                list.with_cue_at(to, cue)
            })
    }

    /// Inserts a cue at an index, or at the end when the list is shorter
    pub(in crate::back) fn with_cue_at(self, index: usize, cue: Cue) -> Self {
        let index = index.min(self.cues.len());
        Self {
            cues: inserted(self.cues, index, cue),
            ..self
        }
    }

    pub(in crate::back) fn edited(self, id: CueId, edit: Edit) -> Self {
        self.with_cue(id, |cue| cue.edited(edit))
    }

    pub(in crate::back) fn loaded(self, id: CueId, media: Media) -> Self {
        self.with_cue(id, |cue| Cue { media, ..cue })
    }

    fn with_cue(self, id: CueId, change: impl FnOnce(Cue) -> Cue) -> Self {
        Self {
            cues: replaced_where(self.cues, |cue| cue.id == id, change),
            ..self
        }
    }
}
