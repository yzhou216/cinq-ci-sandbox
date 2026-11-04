// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! State that iced keeps for a drop-down list from one frame to the next,
//! through which the keys that move between fields focus the list

use iced::advanced::{
    text::paragraph,
    widget::{Tree, operation::Focusable},
};

use super::menu::Shared;
use crate::front::view::Paragraph;

/// State of a drop-down list
pub(super) struct State {
    /// Whether the menu is open and where its highlight is, which the menu
    /// shares while it is open
    pub(super) shared: Shared,
    /// Whether the list has the keyboard, which it has only until its menu
    /// opens
    pub(super) focused: bool,
    /// Whether the cursor was over the list in the last frame, so that the
    /// list redraws when its look changes
    pub(super) over: bool,
    /// Labels of the options and the placeholder, which set the width
    pub(super) labels: Vec<paragraph::Plain<Paragraph>>,
    pub(super) placeholder: paragraph::Plain<Paragraph>,
    /// State of the widgets of the menu, which outlives each opening
    pub(super) menu: Tree,
}

impl Focusable for State {
    fn is_focused(&self) -> bool {
        self.focused
    }

    fn focus(&mut self) {
        self.focused = true;
    }

    fn unfocus(&mut self) {
        self.focused = false;
    }
}
