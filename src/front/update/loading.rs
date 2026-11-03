// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Loading the files of audio cues, and trying again those that could not
//! be played
//!
//! Reading a file can block for a long time when its drive is slow or has
//! gone, so the files are read on a thread of their own.  One load and one
//! recheck run at a time, so that a read that hangs piles up no reads
//! behind it, and a recheck that hangs never holds back a file just named.

use iced::Task;

use super::on_own_thread;
use crate::{
    back::{Command, Load, load_files},
    front::{App, Message},
};

impl App {
    /// Starts loading the files that audio cues name and have not loaded,
    /// unless a load is under way
    pub(super) fn load(self) -> (Self, Task<Message>) {
        let loads = Load::all(&self.workspace, self.workspace.unloaded());
        if self.loading || loads.is_empty() {
            return (self, Task::none());
        }
        Self {
            loading: true,
            ..self
        }
        .with_task(|app| read(loads, app.show.id(), Message::Loaded))
    }

    /// Records what loading the files of the show of identity `show` found,
    /// unless another show has taken its place since
    pub(super) fn loaded(self, show: u64, found: Vec<Command>) -> Self {
        Self {
            loading: false,
            ..self.with_found(show, found)
        }
    }

    /// Starts reading again the files that could not be played, unless a
    /// recheck is under way
    pub(super) fn recheck(self) -> (Self, Task<Message>) {
        let loads = Load::all(&self.workspace, self.workspace.failed());
        if self.rechecking || loads.is_empty() {
            return (self, Task::none());
        }
        Self {
            rechecking: true,
            ..self
        }
        .with_task(|app| read(loads, app.show.id(), Message::Rechecked))
    }

    /// Records what reading again the files of the show of identity `show`
    /// found, unless another show has taken its place since
    pub(super) fn rechecked(self, show: u64, found: Vec<Command>) -> Self {
        Self {
            rechecking: false,
            ..self.with_found(show, found)
        }
    }

    /// Records what reading the files of the show of identity `show`
    /// found, unless another show has taken its place since
    fn with_found(self, show: u64, found: Vec<Command>) -> Self {
        if show == self.show.id() {
            self.send_all(found)
        } else {
            self
        }
    }
}

/// Reads the files of `loads` on a thread of its own, and hands what it
/// found to `done`, along with the identity of the show they belong to
fn read(loads: Vec<Load>, show: u64, done: fn(u64, Vec<Command>) -> Message) -> Task<Message> {
    on_own_thread(move || load_files(loads)).map(move |found| done(show, found))
}
