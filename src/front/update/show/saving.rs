// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Saving the show in its file, or in another that the operator chooses

use std::path::PathBuf;

use iced::Task;

use super::{EXTENSION, Saved, Show, chooser, file::write, with_extension};
use crate::front::{App, Message, update::on_own_thread};

impl App {
    /// Asks where to save the show, and saves it there
    pub(in crate::front::update) fn choose_file(&self) -> Task<Message> {
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
    pub(in crate::front::update) fn save(&self) -> Task<Message> {
        match self.show.file.clone() {
            Some(file) => self.save_to(file),
            None => self.choose_file(),
        }
    }

    /// Starts saving the show in `file`, with the relative paths of the
    /// files of its cues going from the directory of the file
    pub(in crate::front::update) fn save_to(&self, file: PathBuf) -> Task<Message> {
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
    pub(in crate::front::update) fn saved(self, saved: Saved) -> Self {
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
}
