// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Status of the show, most important first, then at the right the keys of
//! a command typed so far, what the latest opening or saving told or else
//! the latest change to the show with the key that takes it back, the key
//! that saves unsaved changes, and the keys of GO, Panic, and the help page
//!
//! The bar keeps to one line.  When it is too narrow for all of it, the
//! keys of GO, of Panic, of saving, and of the help page give way in turn,
//! each one whole, and then the status gives way from its end, so that the
//! keys typed so far and what the latest change told always show in full.

#[cfg(test)]
mod tests;

use iced::{
    Center, Color, Element, Fill, never,
    widget::{
        container, responsive, rich_text, row, span, text,
        text::{LineHeight, Wrapping},
    },
};
use itertools::Itertools;
use unicode_width::UnicodeWidthStr;

use super::{LINE, SPACING, TEXT_SIZE, answered, char_width, format, giving_way, theme, warnings};
use crate::{
    back::Turn,
    front::{
        App, Message,
        keymap::{Action, written},
    },
};

/// Actions whose keys give way in turn when the bar is too narrow, the
/// first to go first
const YIELDING: [Action; 4] = [Action::Go, Action::Panic, Action::Save, Action::Help];

pub(super) fn view(app: &App) -> Element<'_, Message> {
    // Bar is one line of text high with the gap around it, since a
    // responsive widget fills all the height it is given
    let height = LineHeight::default().to_absolute(TEXT_SIZE).0 + 2.0 * SPACING;
    container(responsive(move |size| bar(app, size.width)))
        .height(height)
        .into()
}

/// Draws the bar `width` wide, with as many hints as fit beside the status.
/// Hints grow to the left, so that the keys typed so far come and go
/// without moving the rest.
fn bar(app: &App, width: f32) -> Element<'_, Message> {
    let status = status(app);
    let room = ((width - 3.0 * SPACING) / char_width()).floor() as usize;
    let status_columns: usize = status.iter().map(|(part, _)| part.width()).sum();
    let shown = fitted(&hints(app), room.saturating_sub(status_columns));
    let status = rich_text(
        status
            .into_iter()
            .map(|(part, color)| span(part).color_maybe(color))
            .collect_vec(),
    )
    .on_link_click(never)
    .wrapping(Wrapping::None);
    row![
        container(status).width(Fill).clip(true),
        text(shown).wrapping(Wrapping::None)
    ]
    .spacing(SPACING)
    .padding(SPACING)
    .align_y(Center)
    .into()
}

/// Returns the hints joined, each one whole, without as many of those
/// that give way as it takes for the rest to fit in `room` columns, or
/// without all of them when even that is too many
fn fitted(hints: &[(Option<Action>, String)], room: usize) -> String {
    giving_way(
        &YIELDING,
        |gone| {
            hints
                .iter()
                .filter(|(action, _)| !action.is_some_and(|action| gone.contains(&action)))
                .map(|(_, hint)| hint)
                .join(LINE)
        },
        |hints| hints.width() <= room,
    )
}

/// Returns the parts of the status, each with the color that makes it
/// stand out, if it does: unsaved changes, show mode, which locks editing,
/// and any warnings stand out
fn status(app: &App) -> Vec<(String, Option<Color>)> {
    let count = warnings(app);
    let mode = if app.editable() {
        "Edit mode"
    } else {
        "Show mode"
    };
    vec![
        (app.show.name().into_owned(), None),
        (app.show.mark().to_owned(), Some(theme::PALETTE.warning)),
        (LINE.to_owned(), None),
        (
            mode.to_owned(),
            (!app.editable()).then_some(theme::PALETTE.primary),
        ),
        (LINE.to_owned(), None),
        (
            format::count(count, "warning"),
            (count > 0).then_some(theme::PALETTE.warning),
        ),
    ]
}

/// Returns the hints in the order they show, each with the action whose
/// keys it names when it can give way
fn hints(app: &App) -> Vec<(Option<Action>, String)> {
    let keymap = &app.config.keymap;
    let key =
        |action: Action, label: &str| keymap.hint(action, label).map(|hint| (Some(action), hint));
    let typed = (!app.keys.is_empty()).then(|| (None, written(&app.keys)));
    let told = app
        .told
        .clone()
        .or_else(|| notice(app))
        .map(|told| (None, told));
    let save = key(Action::Save, Action::Save.name()).filter(|_| app.show.unsaved());
    typed
        .into_iter()
        .chain(told)
        .chain(save)
        .chain(key(Action::Go, "GO"))
        .chain(key(Action::Panic, "Panic"))
        .chain(key(Action::Help, Action::Help.name()))
        .collect()
}

/// Tells the latest change to the show, with the key that takes it back,
/// or that makes it again after an undo, while that key can act
fn notice(app: &App) -> Option<String> {
    let workspace = &app.workspace;
    let (turn, change) = workspace.latest_change()?;
    let told = format::change(workspace, change);
    let (told, action, next) = match turn {
        Turn::Done => (capitalized(&told), Action::Undo, workspace.undoable()),
        Turn::Undone => (format!("Undid: {told}"), Action::Redo, workspace.redoable()),
        Turn::Redone => (format!("Redid: {told}"), Action::Undo, workspace.undoable()),
    };
    next.map(|_| answered(told, &app.config.keymap, action))
}

/// Returns the text with its first letter in upper case
fn capitalized(text: &str) -> String {
    text.chars()
        .take(1)
        .flat_map(char::to_uppercase)
        .chain(text.chars().skip(1))
        .collect()
}
