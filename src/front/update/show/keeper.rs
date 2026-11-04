// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Keeper of unsaved changes, and the thread of its own that writes them to
//! the kept files of their shows

use std::{
    iter,
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
};

use itertools::{Either, Itertools};

use super::{
    Show,
    file::{kept_file, remove, write},
};

/// Keeper of unsaved changes, which writes them to the kept file of their
/// show as they happen, and removes the file once they are gone
///
/// The keeper decides at once what to write and what to remove, and leaves
/// the writing to a thread of its own, so that a slow disk never holds up
/// GO, Panic, or the clock.  The thread makes the changes in the order that
/// they came, makes only the newest change to a file while several wait,
/// and makes every change sent to it before the keeper goes, as the keeper
/// does when the window closes.
#[derive(Debug)]
pub struct Keeper {
    /// Directory that keeps unsaved changes, if the platform has one
    unsaved_directory: Option<PathBuf>,
    /// Identity and version of the show last seen
    seen: Option<(u64, u64)>,
    /// Kept file written last, with the identity of its show
    kept: Option<(u64, PathBuf)>,
    /// Way to the thread that makes the changes to the kept files, which
    /// closes when the keeper goes, so that the thread ends
    jobs: mpsc::Sender<Job>,
    /// Why the changes that failed did, in the order that they failed
    failures: mpsc::Receiver<String>,
}

impl Keeper {
    /// Returns a keeper of unsaved changes in `unsaved_directory`, and
    /// starts its thread, which runs until the keeper goes
    pub fn new(unsaved_directory: Option<PathBuf>) -> Self {
        let (jobs, waiting) = mpsc::channel();
        let (failed, failures) = mpsc::channel();
        thread::spawn(move || work(&waiting, &failed));
        Self {
            unsaved_directory,
            seen: None,
            kept: None,
            jobs,
            failures,
        }
    }

    /// Keeps the unsaved changes of `show` in its kept file, or removes the
    /// file once the show has none, as well as the kept file that the show
    /// had before it moved to another file.  The kept file of another show
    /// stays, for that show to bring back.  Returns why keeping failed, if
    /// it did.  A change that fails on the thread is told at a later call,
    /// once the thread has tried it.
    pub fn keep(&mut self, show: &Show) -> Option<String> {
        let failed = self.failures.try_iter().last();
        if self.seen == Some((show.id, show.version)) {
            return failed;
        }
        self.seen = Some((show.id, show.version));
        let wanted = self
            .unsaved_directory
            .as_deref()
            .filter(|_| show.unsaved())
            .map(|directory| kept_file(directory, show.file.as_deref()));
        let gone = self
            .kept
            .take()
            .filter(|(id, kept)| *id == show.id && wanted.as_ref() != Some(kept))
            .map(|(_, kept)| kept);
        if let Some(gone) = gone {
            self.send(Change::Remove(gone));
        }
        if let Some(wanted) = &wanted {
            self.send(Change::Write(wanted.clone(), show.text.clone()));
        }
        self.kept = wanted.map(|wanted| (show.id, wanted));
        failed.or_else(|| {
            (self.unsaved_directory.is_none() && show.unsaved())
                .then(|| "Cannot keep the unsaved changes: there is no state directory".to_owned())
        })
    }

    /// Sends `change` to the thread, which lives as long as the way to it,
    /// so that sending cannot fail
    fn send(&self, change: Change) {
        let _ = self.jobs.send(Job::Change(change));
    }

    /// Waits for the thread to make every change sent to it so far
    pub(super) fn settled(&self) {
        let (done, settled) = mpsc::channel();
        let _ = self.jobs.send(Job::Settle(done));
        let _ = settled.recv();
    }
}

impl Drop for Keeper {
    /// Waits for the thread to make every change sent to it, so that
    /// closing the window loses none
    fn drop(&mut self) {
        self.settled();
    }
}

/// Work for the thread of a keeper
#[derive(Debug)]
enum Job {
    Change(Change),
    /// Answers once every change sent before it is made
    Settle(mpsc::Sender<()>),
}

/// Change to a kept file
#[derive(Debug)]
enum Change {
    Write(PathBuf, String),
    Remove(PathBuf),
}

impl Change {
    fn file(&self) -> &Path {
        match self {
            Change::Write(file, _) | Change::Remove(file) => file,
        }
    }

    /// Makes the change, and returns why it failed, if it did, naming the
    /// show by its kept file, which goes by the name of the show's file.  A
    /// write does not wait for the disk, since a kept file only has to
    /// outlast a crash of the program.
    fn made(&self) -> Result<(), String> {
        match self {
            Change::Write(file, text) => write(file, text, false),
            Change::Remove(file) => remove(file),
        }
        .map_err(|error| {
            let name = self.file().file_name().unwrap_or_default().display();
            format!("Cannot keep the unsaved changes to {name}: {error}")
        })
    }
}

/// Makes the changes that `jobs` brings, and tells `failed` why those that
/// fail do.  Each turn takes every job that waits, and makes only the
/// newest change to each file, in the order of those changes.
fn work(jobs: &mpsc::Receiver<Job>, failed: &mpsc::Sender<String>) {
    jobs.iter().for_each(|first| {
        let (changes, settles): (Vec<Change>, Vec<mpsc::Sender<()>>) = iter::once(first)
            .chain(jobs.try_iter())
            .partition_map(|job| match job {
                Job::Change(change) => Either::Left(change),
                Job::Settle(done) => Either::Right(done),
            });
        let newest: Vec<Change> = changes
            .into_iter()
            .rev()
            .unique_by(|change| change.file().to_owned())
            .collect();
        newest
            .into_iter()
            .rev()
            .filter_map(|change| change.made().err())
            .for_each(|failure| {
                let _ = failed.send(failure);
            });
        settles.into_iter().for_each(|done| {
            let _ = done.send(());
        });
    });
}
