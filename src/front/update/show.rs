// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Shows that the operator opens and saves, and the unsaved changes that
//! cinq keeps for them
//!
//! A show lives in a file of its own, which the operator names on the
//! command line or picks in the desktop's file chooser.  Opening reads the
//! file and saving writes it, both on a thread of their own, as any reading
//! of files is.  Changes that are not saved never go away unasked: as they
//! happen, the [`Keeper`] keeps them in a file in the state directory of
//! cinq, which the next opening of the show reads, so that neither closing
//! the window, nor opening another show, nor a crash loses them.  The kept
//! changes come back as one change, which undo takes back to the show as
//! saved, so that nothing has to ask first.  [`opening`] puts a show on
//! screen, [`saving`] writes it to its file, [`keeper`] keeps its unsaved
//! changes, and [`file`](mod@file) writes the files that hold both.

mod file;
mod keeper;
mod opening;
mod saving;
#[cfg(test)]
mod tests;

use std::{
    borrow::Cow,
    env,
    path::{Path, PathBuf},
};

use iced::Task;

use super::on_own_thread;
use crate::front::{App, Message, read};
use file::kept_file;
pub use keeper::Keeper;

/// Ending of the name of a show file, which is TOML
const EXTENSION: &str = "cinq.toml";

/// Show on screen, with the file that holds it
#[derive(Debug, Clone, Default)]
pub struct Show {
    /// Serial number of the opening that brought the show, which tells a
    /// show that moved to another file apart from another show
    id: u64,
    /// File that holds the show, or none for a show not saved yet
    file: Option<PathBuf>,
    /// Show as its file holds it
    saved: String,
    /// Show as its file would hold it now
    text: String,
    /// Count of the changes to the show or to its file
    version: u64,
}

impl Show {
    /// Returns whether the show holds changes that its file does not
    pub fn unsaved(&self) -> bool {
        self.text != self.saved
    }

    /// Returns the mark that follows the name of the show while it has
    /// unsaved changes, or nothing
    pub fn mark(&self) -> &'static str {
        if self.unsaved() { " (unsaved)" } else { "" }
    }

    /// Returns the name that the show goes by: the name of its file, or
    /// `untitled` while it has none
    pub fn name(&self) -> Cow<'_, str> {
        self.file
            .as_deref()
            .and_then(Path::file_name)
            .map_or(Cow::Borrowed("untitled"), |name| name.to_string_lossy())
    }

    /// Returns the serial number of the opening that brought the show
    pub(super) fn id(&self) -> u64 {
        self.id
    }

    /// Returns the directory of the file of the show, which the relative
    /// paths of its sound files go from, or none while it has no file
    fn directory(&self) -> Option<&Path> {
        self.file.as_deref().and_then(Path::parent)
    }
}

/// What reading a show found, along with the unsaved changes kept for it
#[derive(Debug, Clone)]
pub struct Read {
    /// File of the show, or none for a new show without one
    file: Option<PathBuf>,
    /// Directory that the relative paths of its sound files go from
    directory: PathBuf,
    /// Text of the file, or none when there is no such file yet, or why it
    /// cannot be read
    text: Result<Option<String>, String>,
    /// Unsaved changes kept for the show, if there are any, or why they
    /// cannot be read
    kept: Result<Option<String>, String>,
}

impl Read {
    /// Reads the show in `file`, or a new show without one, along with the
    /// unsaved changes kept for it in `unsaved_directory`.  A new show finds
    /// its sound files from the directory that cinq started in.
    fn new(file: Option<PathBuf>, unsaved_directory: Option<&Path>) -> Self {
        let directory = file
            .as_deref()
            .and_then(Path::parent)
            .map_or_else(|| env::current_dir().unwrap_or_default(), Path::to_owned);
        let text = file.as_deref().map_or(Ok(None), read);
        let kept = unsaved_directory.map_or(Ok(None), |directory| {
            read(&kept_file(directory, file.as_deref()))
        });
        Self {
            file,
            directory,
            text,
            kept,
        }
    }
}

/// What saving the show found
#[derive(Debug, Clone)]
pub struct Saved {
    /// Identity of the show that was saved, so that a show opened since is
    /// left alone
    show: u64,
    /// File that the show went to
    file: PathBuf,
    /// Show as the file holds it now
    text: String,
    /// Why saving failed, if it did
    result: Result<(), String>,
}

/// Returns the directory that keeps unsaved changes: `cinq/unsaved` in the
/// state directory, or in the directory of local data on platforms without
/// one
pub(super) fn unsaved_directory() -> Option<PathBuf> {
    dirs::state_dir()
        .or_else(dirs::data_local_dir)
        .map(|directory| directory.join("cinq").join("unsaved"))
}

/// Starts reading the show in `file`, or a new show without one, along with
/// the unsaved changes kept for it in `unsaved_directory`
pub(super) fn open(file: Option<PathBuf>, unsaved_directory: Option<PathBuf>) -> Task<Message> {
    on_own_thread(move || Read::new(file, unsaved_directory.as_deref()))
        .map(|read| Message::Opened(Box::new(read)))
}

impl App {
    /// Returns the state with the show as its file would hold it brought up
    /// to date, as after a change to the cues
    pub(super) fn with_text(self) -> Self {
        let text = self.workspace.show(self.show.directory());
        if text == self.show.text {
            return self;
        }
        Self {
            show: Show {
                text,
                version: self.show.version + 1,
                ..self.show
            },
            told: None,
            ..self
        }
    }
}

/// Returns the desktop's file chooser for shows, which starts in
/// `directory`, or wherever the desktop starts without one
fn chooser(directory: Option<&Path>) -> rfd::AsyncFileDialog {
    let chooser = rfd::AsyncFileDialog::new().add_filter("Shows", &[EXTENSION]);
    match directory {
        Some(directory) => chooser.set_directory(directory),
        None => chooser,
    }
}

/// Returns `file` with the ending of a show file, unless it ends in
/// `.toml` already
fn with_extension(file: &Path) -> PathBuf {
    if file
        .extension()
        .is_some_and(|extension| extension == "toml")
    {
        file.to_owned()
    } else {
        file.with_added_extension(EXTENSION)
    }
}
