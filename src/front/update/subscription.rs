// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Subscriptions, which turn keys, clicks, the clock, the frames of the
//! stage window, windows that close, and changes of the window size and of
//! the configuration file into messages

use std::time::{Duration, Instant};

use iced::{Event, Subscription, event, keyboard, mouse, time, window};
use itertools::chain;

use super::keys;
use crate::{
    back::TICK,
    front::{App, Message, keymap},
};

/// Interval at which the files that could not be played are tried again
const RECHECK: Duration = Duration::from_secs(2);

impl App {
    /// Listens for keys and clicks in either window, changes of the window
    /// size, windows that close, and changes of the configuration, for
    /// clock ticks while any cue is running, for each frame of the stage
    /// window while its picture moves, so that each frame shows the picture
    /// due then, and for the time to try again the files that could not be
    /// played
    pub(super) fn subscription(&self) -> Subscription<Message> {
        let ticks = self.running().then(|| time::every(TICK).map(Message::Tick));
        let frames = self
            .stage
            .window()
            .filter(|_| self.moving_pictures())
            .map(|stage| event::listen_raw(redrawn).with(stage).filter_map(frame));
        let rechecks = (!self.rechecking && self.workspace.failed().next().is_some())
            .then(|| time::every(RECHECK).map(|_| Message::Recheck));
        let always = [
            keyboard::listen().filter_map(keys::key_press),
            event::listen_with(clicked),
            window::resize_events().map(|_| Message::Resized),
            window::close_events().map(Message::Closed),
        ];
        let reconfigured = self.config.source.file().map(|file| {
            Subscription::run_with(file.to_owned(), |file| keymap::config::watch(file.clone()))
                .map(Message::Reconfigured)
        });
        Subscription::batch(chain!(always, ticks, frames, rechecks, reconfigured))
    }
}

/// Reports a press of any mouse button, captured by a widget or not, so
/// that a click drops a command typed halfway
fn clicked(event: Event, _status: event::Status, _window: window::Id) -> Option<Message> {
    matches!(event, Event::Mouse(mouse::Event::ButtonPressed(_))).then_some(Message::Clicked)
}

/// Reports a redraw of any window, with the time it began
fn redrawn(
    event: Event,
    _status: event::Status,
    window: window::Id,
) -> Option<(window::Id, Instant)> {
    match event {
        Event::Window(window::Event::RedrawRequested(at)) => Some((window, at)),
        _ => None,
    }
}

/// Turns a redraw of the stage window `stage` into a tick of the clock
fn frame((stage, (window, at)): (window::Id, (window::Id, Instant))) -> Option<Message> {
    (window == stage).then_some(Message::Tick(at))
}
