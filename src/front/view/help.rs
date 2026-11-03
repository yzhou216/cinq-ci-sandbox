// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Help page, which lists every key binding over the rest of the window
//!
//! The page links to the list of key values and to the configuration file,
//! each of which opens in the program that the desktop opens it with, apart
//! from the configuration file on Apple's systems, which opens in the
//! default text editor.  The link makes a configuration file that is not
//! there yet.  The page ends with the notice of the license of cinq.  The
//! keys of the next and previous cue scroll it, and those of the first and
//! last cue go to its top and its bottom.

use std::ffi::OsString;

use iced::{
    Element, Fill,
    widget::{
        center, column, container, mouse_area, opaque, rich_text, scrollable, span, table, text,
        text::{IntoFragment, Span},
    },
};
use strum::VariantArray;

use super::{HELP, SPACING, button, theme};
use crate::front::{
    App, Message,
    keymap::{Action, config::Source},
};

/// Widest that the page grows, so that its lines stay easy to read
const WIDTH: f32 = 960.0;

/// Space above and below the text of each row of the table
const ROW_GAP: f32 = 2.0;

/// List of the key values of the W3C UI Events specification
const KEY_VALUES: &str = "https://www.w3.org/TR/uievents-key/";

/// Licenses of the Free Software Foundation, to which the notice of the
/// license of cinq points
const LICENSES: &str = "https://www.gnu.org/licenses/";

/// Notice of the license of cinq, as the README gives it, whose last
/// paragraph goes on with a link to [`LICENSES`]
const NOTICE: [&str; 3] = [
    "cinq is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or any later version.",
    "This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU General Public License for more details.",
    "You should have received a copy of the GNU General Public License along with this program.  If not, see ",
];

pub(super) fn view(app: &App) -> Element<'_, Message> {
    let keymap = &app.config.keymap;
    let bindings = table::table(
        [
            table::column(text("Keys"), |action: Action| text(keymap.listed(action))),
            table::column(text("Action"), |action: Action| text(action.description())).width(Fill),
            table::column(text("Name in config.toml"), |action: Action| {
                text(action.name())
            }),
        ],
        Action::VARIANTS.iter().copied(),
    )
    // Rows closer than those of the cue list, so that the page fits the
    // window without scrolling
    .padding_y(ROW_GAP);
    let key_values = paragraph(
        [
            span(
                "The manual page cinq(5) describes config.toml, where keys go by their W3C UI Events key values, listed at ",
            ),
            link(KEY_VALUES, OsString::from(KEY_VALUES)),
            span(", apart from the space bar, which is Space."),
        ],
        Message::OpenLink,
    );
    let source = match &app.config.source {
        Source::File(file) => paragraph(
            [
                span("These keys come from "),
                link(file.display().to_string(), file.clone()),
                span(", and change as it does."),
            ],
            Message::EditConfig,
        ),
        Source::Missing(file) => paragraph(
            [
                span("These are the default keys, which a file at "),
                link(file.display().to_string(), file.clone()),
                span(" can change.  Following the link makes it."),
            ],
            Message::EditConfig,
        ),
        Source::NoDirectory => text("These are the default keys.").into(),
    };
    let example = keymap
        .key(Action::Next)
        .map_or_else(String::new, |keys| format!(", as in 3 {keys}"));
    let [free, warranty, copy] = NOTICE;
    let notice = [
        text(free).into(),
        text(warranty).into(),
        paragraph(
            [
                span(copy),
                link(LICENSES, OsString::from(LICENSES)),
                span("."),
            ],
            Message::OpenLink,
        ),
    ];
    let page = column![
        text(format!("Keys of the {} keymap", keymap.name())).size(24),
        bindings,
        text(format!("A count typed before a key repeats a move{example}, deletes as many cues, and picks the row for the first and last cue.")),
        text("The keys of GO and Panic act at once, even halfway through another command."),
        source,
        key_values,
    ]
    .extend(app.link_problem.as_deref().map(|why| text(why).into()))
    .extend(notice)
    .push(button("Close").on_press(Message::CloseHelp))
    .spacing(SPACING);
    let page = container(scrollable(page).id(HELP).spacing(0))
        .style(container::bordered_box)
        .padding(SPACING)
        .max_width(WIDTH);
    // Clicks beside the page close it, and the page keeps its own clicks
    opaque(mouse_area(center(opaque(page))).on_press(Message::CloseHelp))
}

/// Returns a paragraph of `spans`, whose links `open` turns into the
/// message that opens what they name
fn paragraph<'a, Link: Clone + 'static>(
    spans: impl AsRef<[Span<'a, Link>]> + 'a,
    open: impl Fn(Link) -> Message + 'a,
) -> Element<'a, Message> {
    rich_text(spans).on_link_click(open).into()
}

/// Returns `label` as a link to `target`, in the color of links and
/// underlined, as links usually are
fn link<'a, Link>(label: impl IntoFragment<'a>, target: Link) -> Span<'a, Link> {
    span(label).link(target).color(theme::LINK).underline(true)
}
