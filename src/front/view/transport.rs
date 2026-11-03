// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! GO button, standby display, and show-wide playback controls

use iced::{
    Element, Fill,
    widget::{checkbox, column, container, row, text, text::Wrapping},
};
use itertools::Itertools;

use super::{SPACING, button, format, theme};
use crate::front::{App, Message};

pub(super) fn view(app: &App) -> Element<'_, Message> {
    let list = app.current();
    let standby = list.standby.and_then(|id| app.workspace.cue(id));
    // Summary keeps to three lines, each cut short at the edge rather than
    // wrapped, so that the parts below stay where they are as standby moves
    let [title, notes, chain] = standby.map_or_else(
        || {
            [
                "Standby: end of list".to_owned(),
                "Notes: none".to_owned(),
                "GO fires nothing".to_owned(),
            ]
        },
        |cue| {
            let notes = if cue.notes.trim().is_empty() {
                "none".to_owned()
            } else {
                cue.notes.lines().join(" ")
            };
            [
                format!("Standby: {cue}"),
                format!("Notes: {notes}"),
                format::chain(&app.workspace, list, cue),
            ]
        },
    );
    let [title, notes, chain] =
        [title, notes, chain].map(|line| text(line).wrapping(Wrapping::None));
    let summary = column![title.size(24), notes, chain];
    let paused = app.workspace.active().iter().any(|active| active.paused);
    row![
        button(text("GO").size(40))
            .style(theme::go)
            .on_press_maybe(standby.map(|_| Message::Go)),
        container(summary).width(Fill).clip(true),
        column![
            row![
                button("Pause all").on_press_maybe(app.running().then_some(Message::PauseAll)),
                button("Resume all").on_press_maybe(paused.then_some(Message::ResumeAll)),
                button("Panic").style(theme::panic).on_press(Message::Panic),
            ]
            .spacing(SPACING),
            checkbox(app.workspace.show_mode())
                .label("Show mode")
                .on_toggle(Message::ShowMode),
        ]
        .spacing(SPACING),
    ]
    .spacing(SPACING)
    .padding(SPACING)
    .into()
}
