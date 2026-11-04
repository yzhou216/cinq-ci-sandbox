// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Rows of an open menu, one to an option, which the mouse highlights and
//! picks

use iced::{
    Event, Length, Rectangle, Renderer, Size, Theme,
    advanced::{
        Clipboard, Renderer as _, Shell,
        layout::{self, Layout},
        mouse, renderer,
        text::Renderer as _,
        widget::{Tree, Widget},
    },
    border, overlay, padding,
};

use super::Choice;
use crate::front::{
    Message,
    view::widget::{
        dropdown::{
            PADDING,
            label::{label, line_height, start},
        },
        pointer_over, pressed,
    },
};

/// Options of a menu, one to a row
pub(super) struct Rows<'b, T> {
    pub(super) choice: Choice<'b, T>,
}

impl<T> Rows<'_, T> {
    /// Returns the row under the cursor
    fn under(
        &self,
        cursor: mouse::Cursor,
        layout: Layout<'_>,
        renderer: &Renderer,
    ) -> Option<usize> {
        let position = cursor.position_in(layout.bounds())?;
        let row = (position.y / row_height(renderer)) as usize;
        (row < self.choice.options.len()).then_some(row)
    }
}

impl<T: ToString + Clone> Widget<Message, Theme, Renderer> for Rows<'_, T> {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Shrink)
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let height = row_height(renderer) * self.choice.options.len() as f32;
        layout::Node::new(limits.resolve(Length::Fill, Length::Shrink, Size::new(0.0, height)))
    }

    /// Draws the rows in view, the highlighted one in the colors of a
    /// selection
    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let look = overlay::menu::default(theme);
        let bounds = layout.bounds();
        let height = row_height(renderer);
        let scrolled = viewport.y - bounds.y;
        let first = (scrolled / height) as usize;
        let last = ((scrolled + viewport.height) / height).ceil() as usize;
        self.choice
            .options
            .iter()
            .enumerate()
            .take(last)
            .skip(first)
            .for_each(|(row, option)| {
                let bounds = Rectangle {
                    y: bounds.y + height * row as f32,
                    height,
                    ..bounds
                };
                let highlighted = self.choice.shared.highlight.get() == Some(row);
                if highlighted {
                    renderer.fill_quad(
                        renderer::Quad {
                            bounds: bounds.shrink(padding::horizontal(look.border.width)),
                            border: border::rounded(look.border.radius),
                            ..renderer::Quad::default()
                        },
                        look.selected_background,
                    );
                }
                let color = if highlighted {
                    look.selected_text_color
                } else {
                    look.text_color
                };
                let name = label(renderer, option.to_string(), f32::INFINITY);
                renderer.fill_text(name, start(renderer, bounds), color, *viewport);
            });
    }

    fn update(
        &mut self,
        _tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let row = self.under(cursor, layout, renderer);
        let highlight = &self.choice.shared.highlight;
        match event {
            Event::Mouse(mouse::Event::CursorMoved { .. })
                if row.is_some() && row != highlight.get() =>
            {
                highlight.set(row);
                shell.request_redraw();
            }
            _ if pressed(event) && row.is_some() => self.choice.pick(row, shell),
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        pointer_over(cursor, layout.bounds())
    }
}

/// Height of a row of the menu: a line of text with the padding around it
pub(super) fn row_height(renderer: &Renderer) -> f32 {
    line_height(renderer) + PADDING.y()
}
