// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Workspace, which holds the cue lists and the cues playing from them
//!
//! Playback here is bookkeeping without side effects: fired cues count
//! time, fades move levels, and stop cues end other cues.  The audio output
//! in [`super::Audio`] turns the result into sound.  A [`Command`] asks for
//! each change, and an [`Active`] records each cue that GO has fired.  The
//! rules of playback live in [`playback`] and [`stopping`], those that
//! change the cue lists in [`editing`], undo and redo in [`history`], and
//! show files in [`show`].

mod active;
mod command;
mod editing;
mod history;
mod playback;
mod show;
mod stopping;
#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

pub use active::{Active, Phase};
pub use command::{Command, Place};
pub use history::{Change, Turn};

use crate::back::{
    Cue, CueId, CueList, Kind, Media, Problem,
    items::{replaced, replaced_where},
};
use history::History;

/// Show as a whole, with its cue lists, running cues, and editing lock
///
/// Front ends read a workspace freely but change it only through
/// [`Workspace::apply`], which keeps every rule about changes in one place.
/// A show always holds at least one cue list, which the front end shows.
#[derive(Debug, Clone)]
pub struct Workspace {
    lists: Vec<CueList>,
    active: Vec<Active>,
    /// Lock on editing during a performance, which leaves playback alone
    show_mode: bool,
    /// Identity the next added cue receives, counted past the largest
    /// identity, so that a show whose identities run out adds no cue
    /// rather than one that shares an identity
    next_id: u64,
    /// Serial number the next fired cue receives
    next_run: u32,
    /// Directory that relative file paths start from
    directory: PathBuf,
    /// Changes to the cues that undo and redo act on
    history: History,
}

impl Default for Workspace {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

/// Returns the identity after the highest that `lists` hold
fn next_id(lists: &[CueList]) -> u64 {
    lists
        .iter()
        .flat_map(|list| &list.cues)
        .map(|cue| u64::from(cue.id.0))
        .max()
        .unwrap_or(0)
        + 1
}

/// Returns `lists`, or one empty list named Main in place of none, so that
/// a show always holds a list
fn listed(lists: Vec<CueList>) -> Vec<CueList> {
    if lists.is_empty() {
        vec![CueList::new("Main", Vec::new())]
    } else {
        lists
    }
}

impl Workspace {
    /// Creates a workspace in edit mode with nothing playing, holding
    /// `lists`, or one empty list without any
    pub fn new(lists: Vec<CueList>) -> Self {
        let lists = listed(lists);
        let next_id = next_id(&lists);
        Self {
            lists,
            active: Vec::new(),
            show_mode: false,
            next_id,
            next_run: 0,
            directory: PathBuf::new(),
            history: History::default(),
        }
    }

    /// Returns the workspace with relative file paths starting from
    /// `directory`, the way QLab finds files next to its workspace file.
    /// Without one, they start from the current directory.
    pub fn directory(self, directory: impl Into<PathBuf>) -> Self {
        Self {
            directory: directory.into(),
            ..self
        }
    }

    /// Returns where the sound file that `cue` names is, or none when it
    /// names no file
    pub(in crate::back) fn path(&self, cue: &Cue) -> Option<PathBuf> {
        cue.file_name().map(|name| self.directory.join(name))
    }

    /// Returns the directory to look in for a sound file for `cue`: the
    /// directory of the file it plays now, or else the one that relative
    /// paths start from
    pub fn sound_directory(&self, cue: &Cue) -> PathBuf {
        self.path(cue)
            .as_deref()
            .and_then(Path::parent)
            .unwrap_or(&self.directory)
            .to_owned()
    }

    /// Returns the name by which a cue plays the sound file at `path`: its
    /// path from the directory that relative paths start from, when the
    /// file is in that directory, so that the show and its sounds can move
    /// together, or else the path as it is
    pub fn file_name_of(&self, path: &Path) -> String {
        path.strip_prefix(&self.directory)
            .unwrap_or(path)
            .to_string_lossy()
            .into_owned()
    }

    pub fn lists(&self) -> &[CueList] {
        &self.lists
    }

    /// Returns the cues that GO has fired and that have not finished yet
    pub fn active(&self) -> &[Active] {
        &self.active
    }

    /// Returns whether editing is locked for a performance
    pub fn show_mode(&self) -> bool {
        self.show_mode
    }

    /// Returns the identity the next added cue will receive, or none once
    /// the identities have run out
    pub fn next_cue_id(&self) -> Option<CueId> {
        u32::try_from(self.next_id).ok().map(CueId)
    }

    pub fn cues(&self) -> impl Iterator<Item = &Cue> {
        self.lists.iter().flat_map(|list| &list.cues)
    }

    pub fn cue(&self, id: CueId) -> Option<&Cue> {
        self.cues().find(|cue| cue.id == id)
    }

    /// Returns the cue that `cue` targets, as a fade or stop cue does, if
    /// the workspace holds it
    pub fn target(&self, cue: &Cue) -> Option<&Cue> {
        cue.target.and_then(|id| self.cue(id))
    }

    pub fn list_of(&self, id: CueId) -> Option<usize> {
        self.lists
            .iter()
            .position(|list| list.position(id).is_some())
    }

    pub fn active_of(&self, id: CueId) -> Option<&Active> {
        self.active.iter().find(|active| active.id == id)
    }

    /// Returns the reason a cue cannot play, if there is one
    pub fn problem<'a>(&self, cue: &'a Cue) -> Option<Problem<'a>> {
        match (cue.kind, &cue.media) {
            (Kind::Audio, _) if cue.file_name().is_none() => Some(Problem::NoFile),
            (Kind::Audio, Media::Unloaded) => Some(Problem::NotLoaded),
            (Kind::Audio, Media::Failed(reason)) => Some(Problem::LoadFailed(reason)),
            (Kind::Audio, Media::Loaded { .. }) if cue.clip().is_none() => Some(Problem::EmptyClip),
            (Kind::Fade | Kind::Stop, _) => match self.target(cue) {
                Some(target) if cue.can_target(target) => None,
                _ => Some(Problem::NoTarget),
            },
            _ => None,
        }
    }

    /// Returns every cue that cannot play, with the reason
    pub fn problems(&self) -> impl Iterator<Item = (&Cue, Problem<'_>)> {
        self.cues()
            .filter_map(|cue| self.problem(cue).map(|problem| (cue, problem)))
    }

    /// Returns the audio cues whose file has not been loaded
    pub fn unloaded(&self) -> impl Iterator<Item = &Cue> {
        self.files_where(|media| *media == Media::Unloaded)
    }

    /// Returns the audio cues whose file could not be played
    pub fn failed(&self) -> impl Iterator<Item = &Cue> {
        self.files_where(|media| matches!(media, Media::Failed(_)))
    }

    /// Returns the audio cues that name a file and whose media passes `test`
    fn files_where(&self, test: fn(&Media) -> bool) -> impl Iterator<Item = &Cue> {
        self.cues().filter(move |cue| {
            cue.kind == Kind::Audio && test(&cue.media) && cue.file_name().is_some()
        })
    }

    /// Returns the workspace with a command applied.  Commands that change
    /// cues do nothing in show mode, which only allows playback and moves
    /// of standby, and undo and redo there take back and make again only
    /// moves of standby.
    pub fn apply(self, command: Command) -> Self {
        match command {
            command if self.show_mode && command.edits() => self,
            Command::Go { list } => self.go(list),
            Command::Panic => self.panic(),
            Command::Pause(paused) => self.paused(paused),
            Command::Stop(id) => self.stopped(id),
            Command::Tick(seconds) => self.tick(seconds),
            Command::Standby { list, cue } => self.moved_standby(list, cue),
            Command::ShowMode(show_mode) => Self { show_mode, ..self },
            Command::Loaded { cue, file, media } => self.loaded(cue, &file, media),
            Command::Undo => self.undone(),
            Command::Redo => self.redone(),
            Command::Add { list, kind, at } => self.added(list, kind, at),
            Command::Delete(ids) => self.deleted(&ids),
            Command::Move { cue, by } => self.moved(cue, by),
            Command::Edit(id, edit) => self.edited(id, edit),
            Command::Restore(lists) => self.restored(lists),
        }
    }

    /// Changes the list at index `list`, or nothing when there is none
    fn with_list(self, list: usize, change: impl FnOnce(CueList) -> CueList) -> Self {
        Self {
            lists: replaced(self.lists, list, change),
            ..self
        }
    }

    /// Changes the list that holds a cue, or nothing when no list holds it
    fn with_list_of(self, id: CueId, change: impl FnOnce(CueList) -> CueList) -> Self {
        match self.list_of(id) {
            Some(list) => self.with_list(list, change),
            None => self,
        }
    }

    /// Changes a cue that GO has fired, or nothing when it is not running
    fn with_active(self, id: CueId, change: impl FnOnce(Active) -> Active) -> Self {
        Self {
            active: replaced_where(self.active, |active| active.id == id, change),
            ..self
        }
    }
}
