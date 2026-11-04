// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Labels of the list and its menu, each on one line in the default font
//! and size, drawn from its top left corner

use iced::{
    Point, Rectangle, Renderer, Size,
    advanced::text::{self, Renderer as _, Text},
    alignment,
};

use super::PADDING;

/// Returns the height of a line of text in the list and its menu
pub(super) fn line_height(renderer: &Renderer) -> f32 {
    f32::from(text::LineHeight::default().to_absolute(renderer.default_size()))
}

/// Returns a label on one line, in the font and size of the list and its
/// menu, to draw from its top left corner at [`start`]
pub(super) fn label<C>(renderer: &Renderer, content: C, width: f32) -> Text<C> {
    Text {
        content,
        bounds: Size::new(width, line_height(renderer)),
        size: renderer.default_size(),
        line_height: text::LineHeight::default(),
        font: renderer.default_font(),
        align_x: text::Alignment::Default,
        align_y: alignment::Vertical::Top,
        shaping: text::Shaping::default(),
        wrapping: text::Wrapping::None,
    }
}

/// Returns the top left corner of a label inside `bounds`, past the left
/// padding and centred on the height.  Labels go by their top left corner
/// rather than by their middle, as iced's own pick list draws them, since
/// iced's software renderer redraws a changed text only below and to the
/// right of the point it is drawn at.
pub(super) fn start(renderer: &Renderer, bounds: Rectangle) -> Point {
    Point::new(
        bounds.x + PADDING.left,
        bounds.center_y() - line_height(renderer) / 2.0,
    )
}
