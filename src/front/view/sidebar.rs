// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Cue lists, running cues, and warnings, all visible at the same time

use iced::{
    Element, Fill,
    widget::{column, progress_bar, radio, row, rule, scrollable, text},
};

use super::{SPACING, button, format, warnings};
use crate::{
    back::{Active, Cue},
    front::{App, Message},
};

/// Thickness of the progress bars of running cues, thin enough that several
/// running cues fit in the sidebar at once
const BAR: f32 = 10.0;

/// Returns the sidebar `width` wide
pub(super) fn view(app: &App, width: f32) -> Element<'_, Message> {
    let workspace = &app.workspace;
    let lists = workspace.lists().iter().enumerate().map(|(index, list)| {
        // Count of cues in brackets, as the headings count running cues and
        // warnings
        radio(
            format!("{} ({})", list.name, list.cues.len()),
            index,
            Some(app.list),
            Message::SwitchList,
        )
        .into()
    });
    let active = workspace
        .active()
        .iter()
        .filter_map(|active| workspace.cue(active.id).map(|cue| active_cue(active, cue)));
    let device = app.audio_error.as_deref().map(|error| {
        text(format!("No audio output: {error}"))
            .style(text::danger)
            .into()
    });
    let file = app
        .file_problem
        .as_deref()
        .map(|problem| text(problem).style(text::warning).into());
    let settings = app.config.problems.iter().map(|problem| {
        text(format!("config.toml: {problem}"))
            .style(text::warning)
            .into()
    });
    let count = warnings(app);
    // Warnings take the warning color of the theme while there are any
    let warned = text(format!("Warnings ({count})")).style(if count > 0 {
        text::warning
    } else {
        text::default
    });
    let problems = workspace.problems().map(|(cue, problem)| {
        button(text(format!("{cue}: {problem}")))
            .on_press(Message::Select(cue.id))
            .width(Fill)
            .into()
    });
    scrollable(
        column![
            text("Cue lists"),
            column(lists).spacing(SPACING),
            rule::horizontal(1),
            text(format!("Active ({})", workspace.active().len())),
            column(active).spacing(SPACING),
            rule::horizontal(1),
            warned,
            column(
                device
                    .into_iter()
                    .chain(file)
                    .chain(settings)
                    .chain(problems)
            )
            .spacing(SPACING),
        ]
        .spacing(SPACING)
        .padding(SPACING)
        .width(Fill),
    )
    .spacing(0)
    .width(width)
    .height(Fill)
    .into()
}

/// Shows a running cue with its progress, and the level of an audio or
/// video cue that sounds
fn active_cue<'a>(active: &Active, cue: &Cue) -> Element<'a, Message> {
    column![
        row![
            text(cue.to_string()).width(Fill),
            button("Stop").on_press(Message::Stop(cue.id)),
        ]
        .spacing(SPACING),
        progress_bar(0.0..=1.0, active.progress()).girth(BAR),
        text(format::activity(active, cue)).style(if active.paused {
            text::warning
        } else {
            text::success
        }),
    ]
    .spacing(SPACING / 2.0)
    .into()
}
