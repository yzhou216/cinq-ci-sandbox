// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Stage window, which shows the pictures of the video cues that play
//!
//! The stage window opens once the show holds a video cue, and shows black
//! while no video plays.  Closing it hides it until a video cue fires
//! again, while closing the main window quits cinq.  Each picture that the
//! video output hands on becomes an image for iced once, as it comes due,
//! and stays until a newer picture of its cue takes its place or the cue
//! stops.  The key of the fullscreen action toggles the window between
//! fullscreen and a window.

#[cfg(test)]
mod tests;

use std::collections::HashMap;

use iced::{
    Size, Task,
    widget::image,
    window::{self, Mode},
};

use crate::{
    back::{CueId, Kind, Picture, Workspace},
    front::{App, Message, window_settings},
};

/// Size that the stage window opens at, in the shape of most video
const SIZE: Size = Size::new(960.0, 540.0);

/// Stage window, with the newest picture of each video cue that runs
#[derive(Default)]
pub struct Stage {
    /// Window while it is open
    window: Option<window::Id>,
    /// Whether the operator closed the window, which keeps it closed until
    /// a video cue fires
    closed: bool,
    /// Newest picture of each video cue that runs, as iced draws it
    pictures: HashMap<CueId, image::Handle>,
}

impl Stage {
    /// Returns the stage window while it is open
    pub fn window(&self) -> Option<window::Id> {
        self.window
    }

    /// Returns whether `window` is the stage window
    pub fn shows_in(&self, window: window::Id) -> bool {
        self.window == Some(window)
    }

    /// Returns the newest picture of the video cue `cue`, if it has one
    pub fn picture(&self, cue: CueId) -> Option<&image::Handle> {
        self.pictures.get(&cue)
    }

    /// Returns the stage with `pictures`, which have come due, in place of
    /// the earlier pictures of their cues, and without the pictures of the
    /// cues that `workspace` no longer runs
    pub(in crate::front) fn with_pictures(
        self,
        workspace: &Workspace,
        pictures: Vec<(CueId, Picture)>,
    ) -> Self {
        let due = pictures.into_iter().map(|(cue, picture)| {
            let image = image::Handle::from_rgba(picture.width, picture.height, picture.pixels);
            (cue, image)
        });
        let pictures = self
            .pictures
            .into_iter()
            .chain(due)
            .filter(|(cue, _)| workspace.active_of(*cue).is_some())
            .collect();
        Self { pictures, ..self }
    }

    /// Returns the stage to open again, as a video cue that fires opens it
    /// after the operator has closed it
    pub(super) fn reopened(self) -> Self {
        Self {
            closed: false,
            ..self
        }
    }
}

impl App {
    /// Opens the stage window once the show holds a video cue, unless it
    /// is open already or the operator has closed it.  The main window then
    /// asks for the keyboard back, as far as the window manager lets it, so
    /// that keys typed for a field of the main window never act in the
    /// stage window, which has none.
    pub(in crate::front) fn staged(self) -> (Self, Task<Message>) {
        let wanted = self.stage.window.is_none()
            && !self.stage.closed
            && self.workspace.cues().any(|cue| cue.kind == Kind::Video);
        if !wanted {
            return (self, Task::none());
        }
        let (window, opened) = window::open(window_settings(SIZE));
        let main = self.main;
        let opening = opened.then(move |_| main.map_or_else(Task::none, window::gain_focus));
        let stage = Stage {
            window: Some(window),
            ..self.stage
        };
        (Self { stage, ..self }, opening)
    }

    /// Hides the stage window once it has closed, until a video cue fires,
    /// and quits once the main window has closed
    pub(super) fn closed(self, window: window::Id) -> (Self, Task<Message>) {
        if self.stage.shows_in(window) {
            let stage = Stage {
                window: None,
                closed: true,
                ..self.stage
            };
            (Self { stage, ..self }, Task::none())
        } else if self.main == Some(window) {
            (self, iced::exit())
        } else {
            (self, Task::none())
        }
    }

    /// Toggles the stage window, if it is open, between fullscreen and a
    /// window, from whichever of the two it is in now
    pub(super) fn fullscreen(&self) -> Task<Message> {
        self.stage.window.map_or_else(Task::none, |window| {
            window::mode(window).then(move |mode| {
                let toggled = match mode {
                    Mode::Fullscreen => Mode::Windowed,
                    Mode::Windowed | Mode::Hidden => Mode::Fullscreen,
                };
                window::set_mode(window, toggled)
            })
        })
    }
}
