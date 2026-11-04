// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Opening a show, which puts it on screen along with the unsaved changes
//! kept for it

use std::path::Path;

use iced::Task;

use super::{Read, Show, chooser, open};
use crate::{
    back::{Command, Workspace},
    front::{App, Message, keymap::Action, view::answered},
};

impl App {
    /// Asks which show to open, starting in the directory of the show on
    /// screen, and starts reading the show chosen
    pub(in crate::front::update) fn choose_show(&self) -> Task<Message> {
        let chooser = chooser(self.show.directory()).set_title("Open show");
        let unsaved_directory = self.unsaved_directory.clone();
        Task::future(chooser.pick_file())
            .and_then(move |chosen| open(Some(chosen.path().to_owned()), unsaved_directory.clone()))
    }

    /// Puts the show that `read` found on screen in place of the one
    /// there, and brings back the unsaved changes kept for it as one
    /// change.  The show that was on screen keeps its own unsaved changes,
    /// which the keeper keeps as they happen.  A show opened again while it
    /// is on screen with unsaved changes brings back the changes on screen,
    /// since its kept file may still wait for the newest of them.
    pub(in crate::front::update) fn opened(self, read: Read) -> Self {
        let name = read.file.as_deref().map_or_else(
            || "a new show".to_owned(),
            |file| file.display().to_string(),
        );
        let from_show = |text: &str| Workspace::from_show(text, &read.directory);
        let workspace = read.text.and_then(|text| match text {
            Some(text) => from_show(&text),
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
            from_show(&self.show.text).map(Some)
        } else {
            read.kept
                .and_then(|kept| kept.as_deref().map(from_show).transpose())
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
}
