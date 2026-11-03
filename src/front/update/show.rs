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
//! saved, so that nothing has to ask first.

#[cfg(test)]
mod tests;

use std::{
    borrow::Cow,
    env, fs,
    io::{self, Write},
    iter,
    path::{Component, Path, PathBuf},
    sync::mpsc,
    thread,
};

use iced::Task;
use itertools::{Either, Itertools};

use super::on_own_thread;
use crate::{
    back::{Command, Workspace},
    front::{App, Message, keymap::Action, read, view::answered},
};

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
    /// Asks which show to open, starting in the directory of the show on
    /// screen, and starts reading the show chosen
    pub(super) fn choose_show(&self) -> Task<Message> {
        let chooser = chooser(self.show.directory()).set_title("Open show");
        let unsaved_directory = self.unsaved_directory.clone();
        Task::future(chooser.pick_file())
            .and_then(move |chosen| open(Some(chosen.path().to_owned()), unsaved_directory.clone()))
    }

    /// Asks where to save the show, and saves it there
    pub(super) fn choose_file(&self) -> Task<Message> {
        let chooser = chooser(self.show.directory())
            .set_title("Save show")
            .set_file_name(match &self.show.file {
                Some(_) => self.show.name().into_owned(),
                None => format!("untitled.{EXTENSION}"),
            });
        Task::future(chooser.save_file())
            .and_then(|chosen| Task::done(Message::SaveTo(with_extension(chosen.path()))))
    }

    /// Saves the show in its file, or asks where to save a show without one
    pub(super) fn save(&self) -> Task<Message> {
        match self.show.file.clone() {
            Some(file) => self.save_to(file),
            None => self.choose_file(),
        }
    }

    /// Starts saving the show in `file`, with the relative paths of its
    /// sound files going from the directory of the file
    pub(super) fn save_to(&self, file: PathBuf) -> Task<Message> {
        let show = self.show.id;
        let text = self.workspace.show(file.parent());
        on_own_thread(move || Saved {
            result: write(&file, &text, true).map_err(|error| error.to_string()),
            show,
            file,
            text,
        })
        .map(Message::Saved)
    }

    /// Records what saving the show found.  The show then goes by the file
    /// it went to, and any change made while it was being saved stays
    /// unsaved.  A save that ends after another show has opened leaves that
    /// show alone.
    pub(super) fn saved(self, saved: Saved) -> Self {
        match saved.result {
            Ok(()) if saved.show != self.show.id => self,
            Ok(()) => {
                let show = Show {
                    text: self.workspace.show(saved.file.parent()),
                    file: Some(saved.file),
                    saved: saved.text,
                    version: self.show.version + 1,
                    ..self.show
                };
                Self {
                    told: Some(format!("Saved {}", show.name())),
                    file_problem: None,
                    show,
                    ..self
                }
            }
            Err(problem) => Self {
                file_problem: Some(format!("Cannot save {}: {problem}", saved.file.display())),
                ..self
            },
        }
    }

    /// Puts the show that `read` found on screen in place of the one
    /// there, and brings back the unsaved changes kept for it as one
    /// change.  The show that was on screen keeps its own unsaved changes,
    /// which the keeper keeps as they happen.  A show opened again while it
    /// is on screen with unsaved changes brings back the changes on screen,
    /// since its kept file may still wait for the newest of them.
    pub(super) fn opened(self, read: Read) -> Self {
        let name = read.file.as_deref().map_or_else(
            || "a new show".to_owned(),
            |file| file.display().to_string(),
        );
        let workspace = read.text.and_then(|text| match text {
            Some(text) => Workspace::from_show(&text, &read.directory),
            None => Ok(Workspace::default().directory(&read.directory)),
        });
        let workspace = match workspace {
            Ok(workspace) => workspace,
            Err(problem) => {
                return Self {
                    file_problem: Some(format!("Cannot open {name}: {problem}")),
                    ..self
                };
            }
        };
        let saved = workspace.show(read.file.as_deref().and_then(Path::parent));
        let again = read.file == self.show.file;
        let kept = if self.show.unsaved() && again {
            Ok(Some(self.show.text.clone()))
        } else {
            read.kept
        };
        // Show that goes keeps its unsaved changes, which opening it again
        // brings back, as starting cinq without a show does for a new show
        let told = (self.show.unsaved() && !again).then(|| {
            let kept = format!("Kept the unsaved changes to {}", self.show.name());
            match self.show.file {
                Some(_) => answered(kept, &self.config.keymap, Action::Open),
                None => kept,
            }
        });
        let opened = Self {
            workspace,
            show: Show {
                id: self.show.id + 1,
                file: read.file,
                saved: saved.clone(),
                text: saved,
                version: self.show.version + 1,
            },
            file_problem: None,
            ..self
        };
        let kept = kept.and_then(|kept| {
            kept.map(|kept| Workspace::from_show(&kept, &read.directory))
                .transpose()
        });
        let opened = match kept {
            Ok(Some(kept)) if kept.show(opened.show.directory()) != opened.show.saved => {
                opened.send(Command::Restore(kept.lists().to_vec()))
            }
            Ok(_) => opened,
            Err(problem) => Self {
                file_problem: Some(format!(
                    "Cannot bring back the unsaved changes to {name}: {problem}"
                )),
                ..opened
            },
        };
        // Selection and message once the kept changes are back, since
        // bringing them back replaces the cues and clears what was told
        Self { told, ..opened }.switched_to(0)
    }

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
    /// Thread that makes the changes to the kept files
    writer: Writer,
}

impl Keeper {
    /// Returns a keeper of unsaved changes in `unsaved_directory`
    pub fn new(unsaved_directory: Option<PathBuf>) -> Self {
        Self {
            unsaved_directory,
            seen: None,
            kept: None,
            writer: Writer::spawn(),
        }
    }

    /// Keeps the unsaved changes of `show` in its kept file, or removes the
    /// file once the show has none, as well as the kept file that the show
    /// had before it moved to another file.  The kept file of another show
    /// stays, for that show to bring back.  Returns why keeping failed, if
    /// it did.  A change that fails on the thread is told at a later call,
    /// once the thread has tried it.
    pub fn keep(&mut self, show: &Show) -> Option<String> {
        let failed = self.writer.failed();
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
            self.writer.send(Change::Remove(gone));
        }
        if let Some(wanted) = &wanted {
            self.writer
                .send(Change::Write(wanted.clone(), show.text.clone()));
        }
        self.kept = wanted.map(|wanted| (show.id, wanted));
        failed.or_else(|| {
            (self.unsaved_directory.is_none() && show.unsaved())
                .then(|| "Cannot keep the unsaved changes: there is no state directory".to_owned())
        })
    }
}

/// Thread of a keeper, which makes the changes to the kept files, with the
/// ways to and from it
#[derive(Debug)]
struct Writer {
    /// Way to the thread, which closes when the writer goes, so that the
    /// thread ends once it has made every change sent to it
    jobs: Option<mpsc::Sender<Job>>,
    /// Why the changes that failed did, in the order that they failed
    failures: mpsc::Receiver<String>,
    /// Thread, which the writer waits for when it goes
    thread: Option<thread::JoinHandle<()>>,
}

/// Work for the thread of a keeper
#[derive(Debug)]
enum Job {
    Change(Change),
    /// Answers once every change sent before it is made
    #[cfg_attr(not(test), expect(dead_code))]
    Settle(mpsc::Sender<()>),
}

/// Change to a kept file
#[derive(Debug)]
enum Change {
    Write(PathBuf, String),
    Remove(PathBuf),
}

impl Writer {
    /// Starts the thread, which runs until the writer goes
    fn spawn() -> Self {
        let (jobs, waiting) = mpsc::channel();
        let (failed, failures) = mpsc::channel();
        Self {
            jobs: Some(jobs),
            failures,
            thread: Some(thread::spawn(move || work(&waiting, &failed))),
        }
    }

    /// Sends `change` to the thread, which lives as long as the way to it,
    /// so that sending cannot fail
    fn send(&self, change: Change) {
        if let Some(jobs) = &self.jobs {
            let _ = jobs.send(Job::Change(change));
        }
    }

    /// Returns why the latest change that failed since the last call did,
    /// if one did
    fn failed(&self) -> Option<String> {
        self.failures.try_iter().last()
    }

    /// Waits for the thread to make every change sent to it so far
    #[cfg(test)]
    fn settled(&self) {
        let (done, settled) = mpsc::channel();
        if let Some(jobs) = &self.jobs {
            let _ = jobs.send(Job::Settle(done));
        }
        let _ = settled.recv();
    }
}

impl Drop for Writer {
    /// Waits for the thread to make every change sent to it, so that
    /// closing the window loses none
    fn drop(&mut self) {
        self.jobs = None;
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
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

/// Returns the file in `unsaved_directory` that keeps the unsaved changes
/// of the show in `file`, or of a new show without one: by the path of the
/// show's file under `shows`, in which `..` takes away the directory before
/// it, or as `untitled.cinq.toml` for a new show
fn kept_file(unsaved_directory: &Path, file: Option<&Path>) -> PathBuf {
    match file {
        Some(file) => unsaved_directory.join("shows").join(file.components().fold(
            PathBuf::new(),
            |path, component| match component {
                Component::Normal(name) => path.join(name),
                Component::ParentDir => path.parent().map_or_else(PathBuf::new, Path::to_owned),
                _ => path,
            },
        )),
        None => unsaved_directory.join(format!("untitled.{EXTENSION}")),
    }
}

/// Writes `text` to a file next to `file`, which then takes its place, so
/// that a failure halfway leaves the file as it was.  The file keeps its
/// permissions, and a link to it stays a link, since the text goes to the
/// file that the link names.  A new file gets the permissions of any file
/// that the program creates.  Waiting for the disk, as saving a show does,
/// makes the text outlast a power cut as well.
fn write(file: &Path, text: &str, wait_for_disk: bool) -> io::Result<()> {
    let file = fs::canonicalize(file).unwrap_or_else(|_| file.to_owned());
    let directory = file.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(directory)?;
    let mut temporary = created_in(directory)?;
    if let Ok(metadata) = fs::metadata(&file) {
        temporary
            .as_file()
            .set_permissions(metadata.permissions())?;
    }
    temporary.write_all(text.as_bytes())?;
    if wait_for_disk {
        temporary.as_file().sync_all()?;
    }
    temporary
        .persist(file)
        .map(drop)
        .map_err(|error| error.error)
}

/// Returns a new, empty file in `directory` that asks for the permissions
/// any created file asks for, so that the umask narrows them as it does for
/// every other file, where tempfile on its own makes the file private
#[cfg(unix)]
fn created_in(directory: &Path) -> io::Result<tempfile::NamedTempFile> {
    use std::os::unix::fs::PermissionsExt;
    tempfile::Builder::new()
        .permissions(fs::Permissions::from_mode(0o666))
        .tempfile_in(directory)
}

/// Returns a new, empty file in `directory`
#[cfg(not(unix))]
fn created_in(directory: &Path) -> io::Result<tempfile::NamedTempFile> {
    tempfile::NamedTempFile::new_in(directory)
}

/// Removes a file, which is no failure when it is gone already
fn remove(file: &Path) -> io::Result<()> {
    match fs::remove_file(file) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}
