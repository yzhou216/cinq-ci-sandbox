// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Keys in the manner of vi: letters move and act, some commands take two
//! keys, and the arrows and Control with `n` and `p` move as well

use crate::front::keymap::Action;

pub(super) fn keys(action: Action) -> &'static [&'static str] {
    match action {
        Action::Go => &["Space"],
        Action::Panic => &["Escape"],
        Action::Next => &["j", "ArrowDown", "Control+n"],
        Action::Previous => &["k", "ArrowUp", "Control+p"],
        Action::NextList => &["l", "ArrowRight"],
        Action::PreviousList => &["h", "ArrowLeft"],
        Action::First => &["g g"],
        Action::Last => &["G"],
        Action::NextTab => &["g t"],
        Action::PreviousTab => &["g T"],
        Action::Standby => &["Enter"],
        Action::Edit => &["i"],
        Action::NextField => &["Tab"],
        Action::PreviousField => &["Shift+Tab"],
        Action::AddBelow => &["o"],
        Action::AddAbove => &["O"],
        Action::MoveDown => &["Alt+j", "Alt+ArrowDown"],
        Action::MoveUp => &["Alt+k", "Alt+ArrowUp"],
        Action::Delete => &["d d"],
        Action::Undo => &["u"],
        Action::Redo => &["Control+r"],
        Action::Open => &["Control+o"],
        Action::Save => &["Control+s"],
        Action::SaveAs => &["Control+S"],
        Action::Help => &["?"],
        Action::CloseHelp => &["q"],
    }
}
