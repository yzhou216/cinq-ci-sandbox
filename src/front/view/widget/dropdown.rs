// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Drop-down lists whose menus the keys move through
//!
//! iced's pick list takes no keys, so keys pressed over its open menu act
//! on the cue list behind it.  These lists look like iced's pick list but
//! open a [`menu`] of their own, in which the keys of [`Action::Next`] and
//! [`Action::Previous`] move the highlight, which the menu keeps in view,
//! and Enter picks the highlighted option.  GO and Panic act as everywhere
//! and close the menu, so that the show is in view, and the menu keeps
//! every other key until it closes.
//!
//! The keys that move between fields move to these lists as well, which
//! then show the border of a focused field, and Enter opens the menu, which
//! takes the keys from there, unless Enter is bound to GO or Panic, which
//! act as everywhere.  The list hands the keyboard to its menu as it
//! opens, so that the keys act on the cue list again once the menu closes,
//! as they do after Enter leaves a text field.
//!
//! [`Action::Next`]: crate::front::keymap::Action::Next
//! [`Action::Previous`]: crate::front::keymap::Action::Previous

mod label;
mod menu;
mod state;
mod widget;

use iced::{Element, Padding, advanced::Shell, widget::button};

use crate::front::{Message, keymap::Keymap};
use menu::{Choice, Shared};

/// Space around the text of the list and of each option of its menu, which
/// iced's buttons and pick lists leave too
const PADDING: Padding = button::DEFAULT_PADDING;

/// Drop-down list of options, one of which may be chosen
pub struct Dropdown<'a, T> {
    options: Vec<T>,
    selected: Option<T>,
    on_select: Box<dyn Fn(T) -> Message + 'a>,
    placeholder: Option<String>,
    keymap: &'a Keymap,
}

/// Returns a drop-down list of `options` with `selected` chosen, which
/// sends the message that `on_select` makes of the option picked.  The keys
/// of `keymap` move through its menu.
pub fn dropdown<'a, T>(
    options: impl Into<Vec<T>>,
    selected: Option<T>,
    on_select: impl Fn(T) -> Message + 'a,
    keymap: &'a Keymap,
) -> Dropdown<'a, T> {
    Dropdown {
        options: options.into(),
        selected,
        on_select: Box::new(on_select),
        placeholder: None,
        keymap,
    }
}

impl<T: PartialEq> Dropdown<'_, T> {
    /// Returns the list showing `placeholder` while no option is chosen
    pub fn placeholder(self, placeholder: impl Into<String>) -> Self {
        Self {
            placeholder: Some(placeholder.into()),
            ..self
        }
    }

    /// Opens the menu with the highlight on the chosen option, unless there
    /// is no option, and keeps the event that opens it
    fn open(&self, choice: Choice<'_, T>, shell: &mut Shell<'_, Message>)
    where
        T: Clone,
    {
        if !self.options.is_empty() {
            choice.open(self.chosen());
        }
        shell.capture_event();
        shell.request_redraw();
    }

    /// Returns the place of the chosen option among the options
    fn chosen(&self) -> Option<usize> {
        self.options
            .iter()
            .position(|option| Some(option) == self.selected.as_ref())
    }

    /// Returns the options together with the highlight of the menu, through
    /// which the list acts on its menu
    fn choice<'b>(&'b self, shared: &'b Shared) -> Choice<'b, T> {
        Choice::new(&self.options, &*self.on_select, self.keymap, shared)
    }
}

impl<'a, T: ToString + PartialEq + Clone + 'a> From<Dropdown<'a, T>> for Element<'a, Message> {
    fn from(dropdown: Dropdown<'a, T>) -> Self {
        Element::new(dropdown)
    }
}
