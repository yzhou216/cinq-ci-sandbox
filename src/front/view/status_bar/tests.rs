// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the hints that fit in the status bar

use super::fitted;
use crate::front::keymap::Action;

/// Returns the hints after a change, with unsaved changes
fn hints() -> Vec<(Option<Action>, String)> {
    [
        (None, "Added cue 1 | u: undo"),
        (Some(Action::Save), "Control+s: save"),
        (Some(Action::Go), "Space: GO"),
        (Some(Action::Panic), "Escape: Panic"),
        (Some(Action::Help), "?: help"),
    ]
    .into_iter()
    .map(|(action, hint)| (action, hint.to_owned()))
    .collect()
}

#[test]
fn hints_that_fit_show_in_full() {
    assert_eq!(
        fitted(&hints(), 77),
        "Added cue 1 | u: undo | Control+s: save | Space: GO | Escape: Panic | ?: help"
    );
}

#[test]
fn keys_give_way_in_turn_while_what_the_change_told_stays() {
    assert_eq!(
        fitted(&hints(), 76),
        "Added cue 1 | u: undo | Control+s: save | Escape: Panic | ?: help"
    );
    assert_eq!(fitted(&hints(), 31), "Added cue 1 | u: undo | ?: help");
    assert_eq!(fitted(&hints(), 5), "Added cue 1 | u: undo");
}
