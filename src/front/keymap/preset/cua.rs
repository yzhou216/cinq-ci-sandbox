// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Keys of IBM's Common User Access, which most desktop programs follow:
//! the arrows, Home, and End move, Control with a letter edits, and F1
//! helps.  Space and Escape keep GO and Panic, as in QLab.

use crate::front::keymap::Action;

pub(super) fn keys(action: Action) -> &'static [&'static str] {
    match action {
        Action::Go => &["Space"],
        Action::Panic => &["Escape"],
        Action::Next => &["ArrowDown"],
        Action::Previous => &["ArrowUp"],
        Action::NextList => &["Control+PageDown"],
        Action::PreviousList => &["Control+PageUp"],
        Action::First => &["Home"],
        Action::Last => &["End"],
        Action::NextTab => &["Control+Tab"],
        Action::PreviousTab => &["Control+Shift+Tab"],
        Action::Standby => &["Enter"],
        Action::Edit => &["F2"],
        Action::NextField => &["Tab"],
        Action::PreviousField => &["Shift+Tab"],
        Action::AddBelow => &["Control+n"],
        Action::AddAbove => &["Control+N"],
        Action::MoveDown => &["Alt+ArrowDown"],
        Action::MoveUp => &["Alt+ArrowUp"],
        Action::Delete => &["Delete"],
        Action::Undo => &["Control+z"],
        Action::Redo => &["Control+y", "Control+Z"],
        Action::Open => &["Control+o"],
        Action::Save => &["Control+s"],
        Action::SaveAs => &["Control+S"],
        Action::Help => &["F1"],
        Action::CloseHelp => &["Control+w"],
    }
}
