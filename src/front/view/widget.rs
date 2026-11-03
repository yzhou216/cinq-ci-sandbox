// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Widgets that iced lacks
//!
//! [`button`](mod@button) acts on the press rather than the release,
//! [`dropdown`](mod@dropdown) lets keys move through its menu,
//! [`keep_keys`](mod@keep_keys) keeps the keys typed into its text fields
//! from reaching the key bindings, and [`marked_row`](mod@marked_row) marks
//! the selected row of a table.  All but the drop-down work around an iced
//! widget with [`wrap`], while the drop-down draws itself in the look of
//! iced's pick list.

mod button;
mod dropdown;
mod keep_keys;
mod marked_row;
mod wrap;

use iced::{Event, Rectangle, mouse, touch};

pub use button::button;
pub use dropdown::dropdown;
pub use keep_keys::keep_keys;
pub use marked_row::marked_row;

/// Returns whether an event presses on the window, with the left mouse
/// button or a finger
fn pressed(event: &Event) -> bool {
    matches!(
        event,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerPressed { .. })
    )
}

/// Returns the cursor of something to click while the cursor is over
/// `bounds`, and the default cursor elsewhere
fn pointer_over(cursor: mouse::Cursor, bounds: Rectangle) -> mouse::Interaction {
    if cursor.is_over(bounds) {
        mouse::Interaction::Pointer
    } else {
        mouse::Interaction::default()
    }
}
