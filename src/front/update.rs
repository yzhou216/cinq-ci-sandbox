// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Update cycle, which applies each message to the interface state
//!
//! Each concern of the update has a module of its own: [`navigation`]
//! moves the selection, switches lists, and fires GO, [`editing`] adds,
//! moves, and deletes cues, chooses their sound files, and undoes and redoes
//! changes, [`keys`] reads keys and moves the keyboard focus, [`scroll`]
//! keeps the selected cue in view whenever the selection moves, [`loading`]
//! loads the files of audio cues and tries again those that could not be
//! played, and [`show`](mod@show) opens and saves shows and keeps their
//! unsaved changes.  The key bindings follow the configuration file as it
//! changes, through [`keymap::config::watch`](fn@keymap::config::watch).

mod editing;
mod keys;
mod loading;
mod navigation;
mod scroll;
mod show;
#[cfg(test)]
mod tests;

use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    thread,
    time::Duration,
};

use iced::{
    Event, Subscription, Task, event, futures::channel::oneshot, keyboard, mouse, time, window,
};

use super::{App, Cinq, Message, keymap, view};
use crate::back::{Audio, Command, Edit, TICK, parse_level};
pub(super) use show::{Keeper, Read, Saved, Show};

/// Interval at which the files that could not be played are tried again
const RECHECK: Duration = Duration::from_secs(2);

impl Cinq {
    /// Opens the audio device, reads the key bindings from `config`, and
    /// starts opening the show in `show`, or a new show without one, which
    /// the window opens without waiting for
    pub(super) fn new(config: Option<&Path>, show: Option<PathBuf>) -> (Self, Task<Message>) {
        let audio = Audio::open();
        let unsaved_directory = show::unsaved_directory();
        let app = App {
            audio_error: audio.error().map(str::to_owned),
            config: keymap::config::load(config),
            unsaved_directory: unsaved_directory.clone(),
            ..App::default()
        };
        let cinq = Self {
            app,
            audio,
            keeper: Keeper::new(unsaved_directory.clone()),
        };
        (cinq, show::open(show, unsaved_directory))
    }

    /// Applies one message, lets the sound output catch up with the result,
    /// keeps the unsaved changes of the show, and starts loading any file
    /// that the message named.  This is the only place where state is
    /// replaced, and the sound output and the keeper of unsaved changes are
    /// the only parts it calls that have side effects; everything else
    /// builds new values.  Taking the state out of `self` instead of cloning
    /// it keeps updates free of copies.
    pub(super) fn update(&mut self, message: Message) -> Task<Message> {
        let (app, task) = std::mem::take(&mut self.app).next(message);
        let workspace = self.audio.sync(app.workspace);
        let problem = self.keeper.keep(&app.show);
        let app = App {
            workspace,
            file_problem: problem.or(app.file_problem),
            ..app
        };
        let (app, load) = app.load();
        self.app = app;
        Task::batch([task, load])
    }

    pub(super) fn view(&self) -> iced::Element<'_, Message> {
        view::view(&self.app)
    }

    /// Returns the title of the window: the name of the show, marked while
    /// it has unsaved changes, and the name of the program
    pub(super) fn title(&self) -> String {
        let show = &self.app.show;
        format!("{}{} - cinq", show.name(), show.mark())
    }

    pub(super) fn subscription(&self) -> Subscription<Message> {
        self.app.subscription()
    }
}

impl App {
    /// Listens for keys, clicks, changes of the window size, and changes of
    /// the configuration, for clock ticks while any cue is running, and for
    /// the time to try again the files that could not be played
    fn subscription(&self) -> Subscription<Message> {
        let ticks = self.running().then(|| time::every(TICK).map(Message::Tick));
        let rechecks = (!self.rechecking && self.workspace.failed().next().is_some())
            .then(|| time::every(RECHECK).map(|_| Message::Recheck));
        let always = [
            keyboard::listen().filter_map(keys::key_press),
            event::listen_with(clicked),
            window::resize_events().map(|_| Message::Resized),
        ];
        let reconfigured = self.config.source.file().map(|file| {
            Subscription::run_with(file.to_owned(), |file| keymap::config::watch(file.clone()))
                .map(Message::Reconfigured)
        });
        Subscription::batch(
            always
                .into_iter()
                .chain(ticks)
                .chain(rechecks)
                .chain(reconfigured),
        )
    }

    /// Returns the state without the last failure to open a link, along
    /// with the task that runs `open` on a thread of its own, since an
    /// opener can take a moment to tell that it failed
    fn opening(
        self,
        open: impl FnOnce() -> Result<(), String> + Send + 'static,
    ) -> (Self, Task<Message>) {
        let task = on_own_thread(open).map(Message::LinkOpened);
        let app = Self {
            link_problem: None,
            ..self
        };
        (app, task)
    }

    /// Returns the state along with the task that `task` starts from it
    fn with_task(self, task: impl FnOnce(&Self) -> Task<Message>) -> (Self, Task<Message>) {
        let task = task(&self);
        (self, task)
    }

    /// Applies one message to the state, along with the widget operations
    /// it asks for
    fn handled(self, message: Message) -> (Self, Task<Message>) {
        let app = match message {
            Message::Typed { press, repeat } => return self.typed(press, repeat),
            Message::Focus(focus) => return self.focused(focus),
            // Selection scrolls into view as it or the rows around it move
            Message::Select(id) => return self.with_selection(id).revealed(),
            Message::Step(offset) => return self.stepped(offset).revealed(),
            Message::Row(row) => return self.at_row(row).revealed(),
            Message::SwitchList(list) => return self.switched_to(list).revealed(),
            Message::StepList(offset) => return self.stepped_list(offset).revealed(),
            Message::Add(place) => return self.with_new_cue(place).revealed(),
            Message::Move(by) => return self.with_selected_moved(by).revealed(),
            Message::Delete(count) => return self.without_selected(count).revealed(),
            Message::Undo => return self.undone().revealed(),
            Message::Redo => return self.redone().revealed(),
            Message::Resized => return self.revealed(),
            Message::Recheck => return self.recheck(),
            Message::Open => return self.with_task(Self::choose_show),
            Message::Opened(read) => return self.opened(*read).revealed(),
            Message::Save => return self.with_task(Self::save),
            Message::SaveAs => return self.with_task(Self::choose_file),
            Message::SaveTo(file) => return self.with_task(|app| app.save_to(file)),
            Message::ChooseSound(id) => return self.with_task(|app| app.choose_sound(id)),
            Message::OpenLink(target) => return self.opening(move || open_link(&target)),
            Message::EditConfig(file) => return self.opening(move || edit_config(&file)),
            // GO and Panic close the help page, so that the show is in view
            // as it runs or stops, and a selection that GO moves scrolls
            // into view, while the list stays put otherwise
            Message::Go => {
                let selected = self.selected;
                let gone = self.gone();
                return if gone.selected == selected {
                    (gone, Task::none())
                } else {
                    gone.revealed()
                };
            }
            Message::Panic => Self {
                help: false,
                ..self.send(Command::Panic)
            },
            Message::PauseAll => self.send(Command::Pause(true)),
            Message::ResumeAll => self.send(Command::Pause(false)),
            Message::Stop(id) => self.send(Command::Stop(id)),
            Message::Tick(now) => {
                let elapsed = self.last_tick.map_or(TICK, |last| now - last);
                Self {
                    last_tick: Some(now),
                    ..self.send(Command::Tick(elapsed.as_secs_f64()))
                }
            }
            // Nothing without a selected cue, as the toolbar's button does
            Message::StandbySelected => {
                let standby = self.selected_here().map(|cue| Command::Standby {
                    list: self.list,
                    cue: Some(cue),
                });
                self.send_all(standby)
            }
            Message::ShowMode(show_mode) => self.send(Command::ShowMode(show_mode)),
            Message::Loaded(show, found) => self.loaded(show, found),
            Message::Rechecked(show, found) => self.rechecked(show, found),
            Message::Saved(saved) => self.saved(saved),
            // Keys typed halfway went by the bindings before, so the update
            // drops them, as it does for most messages
            Message::Reconfigured(config) => Self { config, ..self },
            Message::LinkOpened(result) => Self {
                link_problem: result.err(),
                ..self
            },
            Message::Clicked => self,
            Message::Edit(id, edit) => self.send(Command::Edit(id, edit)),
            Message::LevelTyped(cue, text) => {
                let edit = parse_level(&text).map(|level| Command::Edit(cue, Edit::Level(level)));
                Self {
                    typed_level: Some((cue, text)),
                    ..self
                }
                .send_all(edit)
            }
            Message::SoundChosen(id, path) => {
                let file = self.workspace.file_name_of(&path);
                let edit = self
                    .workspace
                    .cue(id)
                    .map(|cue| Command::Edit(id, cue.playing(file)));
                self.send_all(edit)
            }
            Message::ToggleHelp => Self {
                help: !self.help,
                ..self
            },
            Message::CloseHelp => Self {
                help: false,
                ..self
            },
            Message::Tab(tab) => Self { tab, ..self },
            Message::StepTab(offset) => Self {
                tab: self.tab.stepped(offset),
                ..self
            },
            Message::NewKind(new_kind) => Self { new_kind, ..self },
        };
        (app, Task::none())
    }

    /// Hands a command to the back end, and brings the show as its file
    /// would hold it up to date after a command that can change the cues.
    /// The list on screen stays within the lists, whose number undo and
    /// bringing back unsaved changes can change.
    fn send(self, command: Command) -> Self {
        let changes = command.edits() || matches!(command, Command::Undo | Command::Redo);
        let workspace = self.workspace.apply(command);
        let app = Self {
            list: self.list.min(workspace.lists().len().saturating_sub(1)),
            workspace,
            ..self
        };
        if changes { app.with_text() } else { app }
    }

    /// Hands each of `commands` to the back end in turn, as [`App::send`]
    /// does
    fn send_all(self, commands: impl IntoIterator<Item = Command>) -> Self {
        commands.into_iter().fold(self, Self::send)
    }

    /// Returns the state after one message, with the widget operations it
    /// asks for.  Anything the operator does other than typing drops a
    /// command typed halfway, so that a stray `d` cannot wait through a
    /// click and an edit to delete a cue.  A level typed so far gives way to
    /// the level of its cue as well, apart from a click, which can land in
    /// its own field.  The clock stops with the last running cue, so that
    /// time spent idle or paused never reaches the next tick, and a link
    /// that failed to open is forgotten once the help page that holds it
    /// closes.
    fn next(self, message: Message) -> (Self, Task<Message>) {
        let passive = matches!(
            message,
            Message::Tick(_)
                | Message::Loaded(..)
                | Message::Recheck
                | Message::Rechecked(..)
                | Message::LinkOpened(_)
                | Message::Saved(_)
                | Message::Resized
        );
        let typing = passive || matches!(message, Message::Typed { .. });
        let clicked = matches!(message, Message::Clicked);
        let app = Self {
            keys: if typing { self.keys } else { Vec::new() },
            typed_level: self.typed_level.filter(|_| passive || clicked),
            ..self
        };
        let (app, task) = app.handled(message);
        let app = Self {
            last_tick: app.last_tick.filter(|_| app.running()),
            link_problem: app.link_problem.filter(|_| app.help),
            ..app
        };
        (app, task)
    }
}

/// Reports a press of any mouse button, captured by a widget or not, so
/// that a click drops a command typed halfway
fn clicked(event: Event, _status: event::Status, _window: window::Id) -> Option<Message> {
    matches!(event, Event::Mouse(mouse::Event::ButtonPressed(_))).then_some(Message::Clicked)
}

/// Runs blocking `work` on a thread of its own once iced runs the task: off
/// the interface thread, where GO and Panic must never wait, and off the
/// workers of iced's executor, which keep the clock ticking.  The task gives
/// what the work returns, or nothing when the program closes first.
fn on_own_thread<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> Task<T> {
    Task::future(async move {
        let (sender, receiver) = oneshot::channel();
        thread::spawn(move || {
            // Receiver has gone only when the program is closing
            let _ = sender.send(work());
        });
        receiver.await.ok()
    })
    .and_then(Task::done)
}

/// Opens a web page or a file in the program that the desktop opens it
/// with: through xdg-open on desktops that follow the XDG specifications,
/// and through the system's own opener elsewhere.  Returns why it failed,
/// if it did.
fn open_link(target: &OsStr) -> Result<(), String> {
    open::that_detached(target)
        .map_err(|error| format!("Cannot open {}: {error}", target.to_string_lossy()))
}

/// Opens the configuration file `file` as [`open_link`] does, after making
/// it when it is not there yet, except on Darwin, which opens it in the
/// default text editor, since few programs there say they open TOML.
/// Returns why it failed, if it did.
fn edit_config(file: &Path) -> Result<(), String> {
    keymap::config::make(file)
        .map_err(|error| format!("Cannot make {}: {error}", file.display()))?;
    if cfg!(target_os = "macos") {
        open_in_text_editor(file)
    } else {
        open_link(file.as_os_str())
    }
}

/// Opens `file` in the default text editor through `open -t`, and waits
/// for `open`, which tells whether there is one.  Returns why it failed, if
/// it did.
fn open_in_text_editor(file: &Path) -> Result<(), String> {
    let cannot = |why: String| format!("Cannot open {}: {why}", file.display());
    let status = std::process::Command::new("/usr/bin/open")
        .args(["-t", "--"])
        .arg(file)
        .status()
        .map_err(|error| cannot(error.to_string()))?;
    if status.success() {
        Ok(())
    } else {
        Err(cannot(format!("open -t failed with {status}")))
    }
}
