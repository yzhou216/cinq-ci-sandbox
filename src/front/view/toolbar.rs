// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Editing actions above the cue list
//!
//! The list of the kind of cue to add keeps the keys typed while it has
//! the keyboard, as the fields of the inspector do.

use iced::{
    Element,
    widget::{row, space},
};
use strum::VariantArray;

use super::{SPACING, button, dropdown, keep_keys};
use crate::{
    back::{Kind, Place},
    front::{App, Message},
};

pub(super) fn view(app: &App) -> Element<'_, Message> {
    let keymap = &app.config.keymap;
    let selected_here = app.selected_here().is_some();
    // Selected cue on standby already leaves nothing to put there
    let movable = selected_here && app.current().standby != app.selected;
    let tools = row![
        button("Standby selected").on_press_maybe(movable.then_some(Message::StandbySelected)),
        space::horizontal(),
        dropdown(Kind::VARIANTS, Some(app.new_kind), Message::NewKind, keymap),
        button("Add cue").on_press_maybe(app.editable().then_some(Message::Add(Place::After))),
        button("Delete cue")
            .on_press_maybe((app.editable() && selected_here).then_some(Message::Delete(1))),
    ]
    .spacing(SPACING)
    .padding(SPACING);
    keep_keys(tools, keymap)
}
