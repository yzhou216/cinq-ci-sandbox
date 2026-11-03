// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Changes to the show, which undo takes back and redo makes again
//!
//! Every change that the operator makes to the show is recorded: a cue
//! added, moved, or deleted, a setting of a cue edited, a move of standby,
//! and kept cue lists brought back in place of those of the show.  Playback
//! is not, since what has been heard cannot be taken back.  A [`Change`]
//! records only what it touched, so taking it back leaves alone whatever
//! happened since, such as playback and the files that loaded.  Standby
//! goes back only where nothing has moved it in the meantime, and once GO
//! moves standby on, the earlier moves of that standby are forgotten, those
//! that adding a cue made among them, so that neither undo nor redo sends
//! GO back to a cue that has played.  Bringing back kept lists is the
//! exception: its undo and redo put whole lists in place, standby included.
//! Edits of one setting of one cue in a row, such as the keys typed into a
//! field, make one change, as do moves of one cue in a row.  [`undo`]
//! applies undo and redo to a workspace.

mod change;
mod undo;

pub use change::{Change, Placed, Standby, Turn};

use crate::back::items::replaced;

/// Changes that undo can take back, and the changes it took back that redo
/// can make again
#[derive(Debug, Clone, Default)]
pub(super) struct History {
    undo: Vec<Change>,
    redo: Vec<Change>,
    latest: Option<Turn>,
}

impl History {
    /// Returns the history with a change made, after which redo has nothing
    /// to make again
    fn recorded(self, change: Change) -> Self {
        let length = self.undo.len();
        let joins = self.latest == Some(Turn::Done)
            && self.undo.last().is_some_and(|last| last.takes(&change));
        let undo: Vec<Change> = if joins {
            replaced(self.undo, length - 1, |last| last.then(change))
        } else {
            self.undo.into_iter().chain([change]).collect()
        };
        // Changes in a row that end where they began leave nothing to undo
        if joins && undo.last().is_some_and(Change::changes_nothing) {
            return Self {
                undo: undo.into_iter().take(length - 1).collect(),
                redo: Vec::new(),
                latest: None,
            };
        }
        Self {
            undo,
            redo: Vec::new(),
            latest: Some(Turn::Done),
        }
    }

    /// Returns the way the latest change went, with the change that undo
    /// or redo acts on now
    fn latest(&self) -> Option<(Turn, &Change)> {
        match self.latest? {
            Turn::Undone => self.redo.last().map(|change| (Turn::Undone, change)),
            turn => self.undo.last().map(|change| (turn, change)),
        }
    }

    /// Takes the change that undo takes back next off its list and onto the
    /// list of redo
    fn undoing(self) -> (Self, Option<Change>) {
        let (undo, redo, change) = moved(self.undo, self.redo);
        let latest = change.as_ref().map_or(self.latest, |_| Some(Turn::Undone));
        (Self { undo, redo, latest }, change)
    }

    /// Takes the change that redo makes again next off its list and onto
    /// the list of undo
    fn redoing(self) -> (Self, Option<Change>) {
        let (redo, undo, change) = moved(self.redo, self.undo);
        let latest = change.as_ref().map_or(self.latest, |_| Some(Turn::Redone));
        (Self { undo, redo, latest }, change)
    }

    /// Forgets the moves of a list's standby, which GO has moved on from,
    /// including those that adding a cue made
    fn without_moves_of(self, list: usize) -> Self {
        let latest = match self.latest() {
            Some((_, change)) if change.moves_standby_of(list) => None,
            _ => self.latest,
        };
        let kept = |changes: Vec<Change>| -> Vec<Change> {
            changes
                .into_iter()
                .filter(|change| !change.moves_standby_of(list))
                .map(|change| change.keeping_standby_of(list))
                .collect()
        };
        Self {
            undo: kept(self.undo),
            redo: kept(self.redo),
            latest,
        }
    }
}

/// Moves the last change of `from` to the end of `to`, returning both lists
/// along with the change
fn moved(mut from: Vec<Change>, to: Vec<Change>) -> (Vec<Change>, Vec<Change>, Option<Change>) {
    let change = from.pop();
    let to = to.into_iter().chain(change.clone()).collect();
    (from, to, change)
}
