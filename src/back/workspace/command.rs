// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Requests that change a workspace

use crate::back::{CueId, CueList, Edit, Kind, Media};

/// Request to change the show
///
/// The interface sends these through [`super::Workspace::apply`].  Other
/// front ends, such as an OSC server, could send the same commands.
#[derive(Debug, Clone)]
pub enum Command {
    /// Fires the standby cue of a list along with its chain
    Go { list: usize },
    /// Fades out every playing cue and stops the rest, or stops everything
    /// at once while an earlier panic is still fading
    Panic,
    /// Pauses or resumes every running cue
    Pause(bool),
    /// Stops one running cue
    Stop(CueId),
    /// Advances the playback clock by a number of seconds
    Tick(f64),
    /// Moves the standby position of a list
    Standby { list: usize, cue: Option<CueId> },
    /// Locks editing for a performance, or unlocks it
    ShowMode(bool),
    /// Adds a cue of a kind at a place in a list
    Add { list: usize, kind: Kind, at: Place },
    /// Removes cues, stopping those that are running, as one change
    Delete(Vec<CueId>),
    /// Moves a cue within its list by a number of places, downwards when
    /// positive, as far as the ends of the list allow
    Move { cue: CueId, by: isize },
    /// Changes one setting of a cue
    Edit(CueId, Edit),
    /// Puts cue lists kept from before in place of those of the show, as
    /// one change, as bringing back unsaved changes does
    Restore(Vec<CueList>),
    /// Takes back the last change to the cues
    Undo,
    /// Makes again the change that undo took back last
    Redo,
    /// Records what loading the file of a cue found
    Loaded {
        cue: CueId,
        file: String,
        media: Media,
    },
}

impl Command {
    /// Returns whether the command edits the show, as show mode forbids:
    /// adding, deleting, moving, or editing cues, or putting kept cue lists
    /// in place of those of the show
    pub fn edits(&self) -> bool {
        matches!(
            self,
            Command::Add { .. }
                | Command::Delete(_)
                | Command::Move { .. }
                | Command::Edit(..)
                | Command::Restore(_)
        )
    }
}

/// Where in its list a new cue goes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place {
    After(CueId),
    Before(CueId),
    /// End of the list, which is also where a cue goes whose neighbour is
    /// not in the list
    End,
}
