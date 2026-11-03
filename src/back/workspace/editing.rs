// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Changes to the cue lists: standby, loading, and the edits that show
//! mode locks, which [`super::history`] records for undo

use itertools::Itertools;

use super::{
    Workspace,
    history::{Change, Placed, Standby},
};
use crate::back::{Active, CueId, CueList, Edit, Kind, Media, Place};

/// Returns the next whole number above every numeric cue number, which
/// keeps new cue numbers apart across all lists as QLab does.  Numbers too
/// large to count up from leave the next number at the largest there is.
fn next_number<'a>(numbers: impl Iterator<Item = &'a str>) -> String {
    let highest = numbers
        .filter_map(|number| number.trim().parse::<f64>().ok())
        .fold(0.0_f64, f64::max);
    (highest.floor() as u64).saturating_add(1).to_string()
}

impl Workspace {
    /// Puts `standby` on standby in a list without recording the move, as
    /// undo and redo move it
    pub(super) fn with_standby(self, list: usize, standby: Option<CueId>) -> Self {
        self.with_list(list, |cues| cues.with_standby(standby))
    }

    /// Moves standby as the operator asks, recording the move unless
    /// standby is there already
    pub(super) fn moved_standby(self, list: usize, cue: Option<CueId>) -> Self {
        let before = self.lists.get(list).map(|cues| cues.standby);
        let workspace = self.with_standby(list, cue);
        let after = workspace.lists.get(list).map(|cues| cues.standby);
        match before.zip(after) {
            Some((before, after)) if before != after => {
                workspace.recorded(Change::Standby(Standby {
                    list,
                    before,
                    after,
                }))
            }
            _ => workspace,
        }
    }

    /// Adds a cue with the next identity, numbered after the highest cue of
    /// any list, and records it with the standby of its list before and
    /// after, which the new cue can take.  Once the identities have run
    /// out, no cue is added.
    pub(super) fn with_new_cue(self, list: usize, kind: Kind, at: Place) -> Self {
        let Some(id) = self.next_cue_id() else {
            return self;
        };
        let number = next_number(self.cues().map(|cue| cue.number.as_str()));
        let before = self.lists.get(list).and_then(|cues| cues.standby);
        let workspace = Self {
            next_id: self.next_id + 1,
            ..self.with_list(list, |cues| cues.with_new_cue(id, number, kind, at))
        };
        match workspace.place_of(id) {
            Some(placed) => {
                let standby = Standby {
                    list,
                    before,
                    after: workspace.lists[list].standby,
                };
                workspace.recorded(Change::Added {
                    placed: Box::new(placed),
                    standby,
                })
            }
            None => workspace,
        }
    }

    /// Deletes cues as one change, with the places they had and where
    /// standby was before and after
    pub(super) fn without(self, ids: &[CueId]) -> Self {
        // Cues in the order of their places, which is the order undo puts
        // them back in
        let cues: Vec<Placed> = ids
            .iter()
            .unique()
            .filter_map(|&id| self.place_of(id))
            .sorted_by_key(|placed| (placed.list, placed.index))
            .collect();
        // Standby of each list that loses a cue, before it does
        let before: Vec<(usize, Option<CueId>)> = cues
            .iter()
            .map(|placed| placed.list)
            .dedup()
            .map(|list| (list, self.lists[list].standby))
            .collect();
        let workspace = self.removed(ids);
        let standby = before
            .into_iter()
            .map(|(list, before)| Standby {
                list,
                before,
                after: workspace.lists[list].standby,
            })
            .collect();
        if cues.is_empty() {
            workspace
        } else {
            workspace.recorded(Change::Deleted { cues, standby })
        }
    }

    /// Moves a cue within its list by `by` places, downwards when positive,
    /// as far as the ends of the list allow, and records the move unless
    /// the cue stays where it was
    pub(super) fn moved(self, id: CueId, by: isize) -> Self {
        let place = self
            .lists
            .iter()
            .find_map(|cues| cues.position(id).map(|from| (from, cues.cues.len() - 1)));
        let Some((from, last)) = place else {
            return self;
        };
        let to = from.saturating_add_signed(by).min(last);
        if to == from {
            return self;
        }
        self.with_cue_moved(id, to)
            .recorded(Change::Moved { cue: id, from, to })
    }

    /// Moves a cue to the place `to` in its list without recording the
    /// move, as undo and redo move it
    pub(super) fn with_cue_moved(self, id: CueId, to: usize) -> Self {
        self.with_list_of(id, |cues| cues.with_moved(id, to))
    }

    /// Edits a cue and records the edit, unless it leaves the setting as it
    /// was, when it changes nothing at all.  Text fields send their text
    /// again on keys that change nothing, such as Backspace at the start of
    /// a field, and such an edit must not stop a running cue.
    pub(super) fn edited(self, id: CueId, edit: Edit) -> Self {
        let undo = self
            .cue(id)
            .map(|cue| cue.reverting(&edit))
            .filter(|undo| *undo != edit);
        match undo {
            Some(undo) => self.changed(id, edit.clone()).recorded(Change::Edited {
                cue: id,
                undo,
                redo: edit,
            }),
            None => self,
        }
    }

    /// Puts cue lists kept from before in place of those of the show, and
    /// records the lists before and after as one change
    pub(super) fn restored(self, lists: Vec<CueList>) -> Self {
        let before = self.lists.clone();
        self.with_lists(lists.clone()).recorded(Change::Restored {
            before,
            after: lists,
        })
    }

    /// Puts `lists` in place of the cue lists without recording a change,
    /// as undo and redo do.  The cues that have gone stop, and new cues get
    /// identities that none of the lists holds.
    pub(super) fn with_lists(self, lists: Vec<CueList>) -> Self {
        let lists = super::listed(lists);
        let next_id = super::next_id(&lists).max(self.next_id);
        let held = |id: CueId| lists.iter().any(|cues| cues.position(id).is_some());
        let gone: Vec<CueId> = self
            .active
            .iter()
            .map(|active| active.id)
            .filter(|&id| !held(id))
            .collect();
        gone.into_iter().fold(
            Self {
                lists,
                next_id,
                ..self
            },
            Self::stopped,
        )
    }

    /// Removes cues from their lists and stops those that are running,
    /// without recording a change, as undo and redo remove them
    pub(super) fn removed(self, ids: &[CueId]) -> Self {
        ids.iter().fold(self, |workspace, &id| {
            workspace
                .with_list_of(id, |cues| cues.without(id))
                .stopped(id)
        })
    }

    /// Edits a cue without recording the edit, as undo and redo edit it.
    /// A running audio cue stops when it gets a new file, and it takes a
    /// new level at once, which ends a fade on it.  A cue that a panic or a
    /// fade is stopping keeps fading out, so that no edit brings it back.
    pub(super) fn changed(self, id: CueId, edit: Edit) -> Self {
        let audio = self.cue(id).is_some_and(|cue| cue.kind == Kind::Audio);
        let workspace = match edit {
            Edit::File(_) | Edit::Sound { .. } => self.stopped(id),
            Edit::Level(level) if audio => self.with_active(id, |active| match active.fade {
                Some(fade) if fade.then_stop => active,
                _ => Active {
                    level,
                    fade: None,
                    ..active
                },
            }),
            _ => self,
        };
        workspace.with_list_of(id, |cues| cues.edited(id, edit))
    }

    /// Records what loading the file of a cue found, unless the cue has
    /// been given another file since loading began.  A run whose sound
    /// could not start then ends, and the rest of its chain plays on, as GO
    /// skips a cue that cannot play.
    pub(super) fn loaded(self, id: CueId, file: &str, media: Media) -> Self {
        if self.cue(id).is_none_or(|cue| cue.file != file) {
            return self;
        }
        let workspace = match media {
            Media::Failed(_) => self.skipped(id),
            _ => self,
        };
        workspace.with_list_of(id, |cues| cues.loaded(id, media))
    }

    /// Returns a cue with its place in the lists
    fn place_of(&self, id: CueId) -> Option<Placed> {
        self.lists.iter().enumerate().find_map(|(list, cues)| {
            cues.position(id).map(|index| Placed {
                list,
                index,
                cue: cues.cues[index].clone(),
            })
        })
    }
}
