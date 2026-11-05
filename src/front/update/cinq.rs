// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Interface together with the parts that have side effects: the sound and
//! video output, the keeper of unsaved changes, and the windows

use std::{
    path::{Path, PathBuf},
    time::Instant,
};

use iced::{Element, Subscription, Task, window};

use super::{Keeper, show};
use crate::{
    back::{Audio, Video},
    front::{App, Cinq, MAIN_SIZE, Message, keymap, view, window_settings},
};

impl Cinq {
    /// Opens the main window and the audio device, reads the key bindings
    /// from `config`, and starts opening the show in `show`, or a new show
    /// without one, which the window opens without waiting for
    pub(in crate::front) fn new(
        config: Option<&Path>,
        show: Option<PathBuf>,
    ) -> (Self, Task<Message>) {
        let audio = Audio::open();
        let unsaved_directory = show::unsaved_directory();
        let (main, opened) = window::open(window_settings(MAIN_SIZE));
        let cinq = Self {
            app: App {
                main: Some(main),
                audio_error: audio.error().map(str::to_owned),
                config: keymap::config::load(config),
                unsaved_directory: unsaved_directory.clone(),
                ..App::default()
            },
            audio,
            video: Video::default(),
            keeper: Keeper::new(unsaved_directory.clone()),
        };
        let opening = show::open(show, unsaved_directory);
        (cinq, Task::batch([opened.discard(), opening]))
    }

    /// Applies one message, lets the sound and video output catch up with
    /// the result, keeps the unsaved changes of the show, starts loading
    /// any file that the message named, and opens the stage window once the
    /// show holds a video cue.  This is the only place where state is
    /// replaced, and the sound and video output and the keeper of unsaved
    /// changes are the only parts it calls that have side effects;
    /// everything else builds new values.  A clock that the message starts
    /// counts from now, so that its first tick counts only the time since.
    /// Taking the state out of `self` instead of cloning it keeps updates
    /// free of copies.
    pub(in crate::front) fn update(&mut self, message: Message) -> Task<Message> {
        let (app, task) = std::mem::take(&mut self.app).next(message);
        let (workspace, pictures) = self.video.sync(self.audio.sync(app.workspace));
        let app = App {
            stage: app.stage.with_pictures(&workspace, pictures),
            file_problem: self.keeper.keep(&app.show).or(app.file_problem),
            workspace,
            ..app
        };
        let app = App {
            last_tick: app.last_tick.or_else(|| app.running().then(Instant::now)),
            ..app
        };
        let (app, load) = app.load();
        let (app, stage) = app.staged();
        self.app = app;
        Task::batch([task, load, stage])
    }

    /// Draws the window `window`: the stage window, or else the main window
    pub(in crate::front) fn view(&self, window: window::Id) -> Element<'_, Message> {
        if self.app.stage.shows_in(window) {
            view::stage::view(&self.app)
        } else {
            view::view(&self.app)
        }
    }

    /// Returns the title of the window `window`: the name of the show and
    /// the name of the program, along with the mark of unsaved changes in
    /// the main window, and the word stage in the stage window
    pub(in crate::front) fn title(&self, window: window::Id) -> String {
        let show = &self.app.show;
        if self.app.stage.shows_in(window) {
            format!("{} stage - cinq", show.name())
        } else {
            format!("{}{} - cinq", show.name(), show.mark())
        }
    }

    pub(in crate::front) fn subscription(&self) -> Subscription<Message> {
        self.app.subscription()
    }
}
