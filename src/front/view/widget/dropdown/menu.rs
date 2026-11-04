// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Menu of a drop-down list, which opens below the list or above it,
//! wherever there is more room, and scrolls when the options do not fit
//!
//! The options sit in iced's scrollable, so the menu scrolls and shows its
//! scroll bar as iced's own menu does.  The mouse highlights the option
//! under it and picks the option it presses, and the keys move the
//! highlight, which the menu scrolls into view.  A click beside the menu
//! closes it and acts as it would with the menu closed, apart from a click
//! on the list, which only closes the menu.
//!
//! iced hands each batch of events first to the menu that is open when the
//! batch begins, then to the widgets under it, so a menu that closes
//! partway through a batch leaves the rest of the batch to them.

mod overlay;
mod rows;
#[cfg(test)]
mod tests;

use std::cell::Cell;

use iced::{
    advanced::Shell,
    keyboard::{Key, key::Named},
};

use crate::front::{
    Message,
    keymap::{Action, Keymap, Press},
    step,
};
pub(super) use overlay::Menu;

/// Whether a menu is open and where its highlight is, which the drop-down
/// list keeps for the menu
#[derive(Default)]
pub(super) struct Shared {
    open: Cell<bool>,
    /// Option under the highlight, which the mouse moves as well as the keys
    highlight: Cell<Option<usize>>,
    /// Whether the menu has yet to scroll the highlight into view
    reveal: Cell<bool>,
}

impl Shared {
    pub(super) fn is_open(&self) -> bool {
        self.open.get()
    }
}

/// Options of a drop-down list with the highlight of its menu, through
/// which the list and the menu act on the menu
pub(super) struct Choice<'b, T> {
    options: &'b [T],
    on_select: &'b dyn Fn(T) -> Message,
    keymap: &'b Keymap,
    shared: &'b Shared,
}

impl<T> Clone for Choice<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Choice<'_, T> {}

impl<'b, T> Choice<'b, T> {
    pub(super) fn new(
        options: &'b [T],
        on_select: &'b dyn Fn(T) -> Message,
        keymap: &'b Keymap,
        shared: &'b Shared,
    ) -> Self {
        Self {
            options,
            on_select,
            keymap,
            shared,
        }
    }
}

impl<T: Clone> Choice<'_, T> {
    /// Opens the menu with the highlight on `row`, which it scrolls into
    /// view
    pub(super) fn open(&self, row: Option<usize>) {
        self.shared.open.set(true);
        self.shared.highlight.set(row);
        self.shared.reveal.set(true);
    }

    /// Carries out what a key does while the menu is open
    pub(super) fn press(&self, key: &Key, press: Option<Press>, shell: &mut Shell<'_, Message>) {
        match reply(self.keymap, key, press) {
            Reply::Step(offset) => {
                let row = step(self.shared.highlight.get(), offset, self.options.len());
                self.shared.highlight.set(row);
                self.shared.reveal.set(true);
                shell.capture_event();
            }
            Reply::Pick => self.pick(self.shared.highlight.get(), shell),
            Reply::Close => self.shared.open.set(false),
            Reply::Keep => shell.capture_event(),
        }
        shell.request_redraw();
    }

    /// Closes the menu on a click beside it, which goes on to act on what it
    /// lands on, unless it lands on the list, whose menu it would open again
    pub(super) fn click_beside(&self, on_list: bool, shell: &mut Shell<'_, Message>) {
        self.shared.open.set(false);
        if on_list {
            shell.capture_event();
        }
        shell.request_redraw();
    }

    /// Sends the message of the option in `row`, if there is one, and
    /// closes the menu
    fn pick(&self, row: Option<usize>, shell: &mut Shell<'_, Message>) {
        if let Some(option) = row.and_then(|row| self.options.get(row)) {
            shell.publish((self.on_select)(option.clone()));
        }
        self.shared.open.set(false);
        shell.capture_event();
    }
}

/// What a key does while the menu is open
#[derive(Debug, PartialEq)]
enum Reply {
    /// Moves the highlight by a number of options, downwards when positive
    Step(isize),
    /// Picks the highlighted option, which closes the menu
    Pick,
    /// Lets the key act and closes the menu, as the keys of GO and Panic do
    Close,
    /// Keeps the key from acting on anything behind the menu
    Keep,
}

/// Decides what a key does while the menu is open, given the press it
/// makes, when the key bindings can name it
fn reply(keymap: &Keymap, key: &Key, press: Option<Press>) -> Reply {
    match (press.and_then(|press| keymap.alone(press)), key) {
        (Some(Action::Next), _) => Reply::Step(1),
        (Some(Action::Previous), _) => Reply::Step(-1),
        (Some(action), _) if action.acts_at_once() => Reply::Close,
        (_, Key::Named(Named::Enter)) => Reply::Pick,
        _ => Reply::Keep,
    }
}
