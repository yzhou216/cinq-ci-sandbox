// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Keys that can follow those typed so far, which a box lists while a
//! command waits for more, as which-key does in Emacs and as Helix does

use iced::{
    Element, Fill, never,
    widget::{column, container, rich_text, span},
};
use unicode_width::UnicodeWidthStr;

use super::{SPACING, theme};
use crate::front::{App, Message, keymap::written};

/// Returns a box in the bottom right corner that lists each key that can
/// come next with what it does, while a command waits for more and the
/// help page is closed
pub(super) fn view(app: &App) -> Option<Element<'_, Message>> {
    if app.help {
        return None;
    }
    let next: Vec<(String, &str)> = app
        .config
        .keymap
        .continuations(&app.keys)
        .map(|(keys, action)| (written(keys), action.description()))
        .collect();
    let width = next.iter().map(|(keys, _)| keys.width()).max()?;
    // Monospace font lines the descriptions up after the padded keys, which
    // are padded by the columns they take, as a wide character takes two
    let lines = next.into_iter().map(|(keys, description)| {
        let padding = " ".repeat(width - keys.width());
        rich_text([
            span(format!("{keys}{padding}")).color(theme::PALETTE.primary),
            span(format!("  {description}")),
        ])
        .on_link_click(never)
        .into()
    });
    let list = container(column(lines))
        .style(container::bordered_box)
        .padding(SPACING);
    Some(
        container(list)
            .align_right(Fill)
            .align_bottom(Fill)
            .padding(SPACING)
            .into(),
    )
}
