// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Menu as an iced overlay, which opens next to its list, takes the keys
//! and the clicks beside it, and scrolls the highlight into view

use iced::{
    Element, Event, Point, Rectangle, Renderer, Size, Theme, Vector,
    advanced::{
        Clipboard, Overlay, Renderer as _, Shell,
        layout::{self, Layout},
        mouse, renderer,
        widget::{
            Id, Tree, Widget,
            operation::{self, Operation},
        },
    },
    keyboard, overlay,
    widget::{Scrollable, operation::AbsoluteOffset},
};

use super::{
    Choice,
    rows::{Rows, row_height},
};
use crate::front::{Message, keymap::Press, reveal, view::widget::pressed};

/// Menu of the options of a drop-down list
pub(in crate::front::view::widget::dropdown) struct Menu<'b, T> {
    choice: Choice<'b, T>,
    /// State of the scrollable, kept in the state of the list
    tree: &'b mut Tree,
    scrollable: Scrollable<'b, Message>,
    /// Bounds of the drop-down list, which the menu opens next to
    target: Rectangle,
}

impl<'b, T: ToString + Clone + 'b> Menu<'b, T> {
    pub(in crate::front::view::widget::dropdown) fn new(
        choice: Choice<'b, T>,
        tree: &'b mut Tree,
        target: Rectangle,
    ) -> Self {
        let scrollable = Scrollable::new(Element::new(Rows { choice }));
        tree.diff(&scrollable as &dyn Widget<Message, Theme, Renderer>);
        Self {
            choice,
            tree,
            scrollable,
            target,
        }
    }
}

impl<T: ToString + Clone> Overlay<Message, Theme, Renderer> for Menu<'_, T> {
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        let below = bounds.height - self.target.y - self.target.height;
        let above = self.target.y;
        let limits = layout::Limits::new(
            Size::ZERO,
            Size::new(bounds.width - self.target.x, below.max(above)),
        )
        .width(self.target.width);
        let node = self.scrollable.layout(self.tree, renderer, &limits);
        let top = if below > above {
            self.target.y + self.target.height
        } else {
            self.target.y - node.size().height
        };
        node.move_to(Point::new(self.target.x, top))
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        let bounds = layout.bounds();
        let look = overlay::menu::default(theme);
        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border: look.border,
                shadow: look.shadow,
                ..renderer::Quad::default()
            },
            look.background,
        );
        self.scrollable
            .draw(self.tree, renderer, theme, style, layout, cursor, &bounds);
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        let shared = self.choice.shared;
        if !shared.is_open() {
            return;
        }
        match event {
            Event::Keyboard(keyboard::Event::KeyPressed {
                key,
                modified_key,
                modifiers,
                ..
            }) => self
                .choice
                .press(key, Press::from_key(modified_key, *modifiers), shell),
            _ if pressed(event) && !cursor.is_over(layout.bounds()) => {
                self.choice.click_beside(cursor.is_over(self.target), shell);
            }
            _ => {
                let bounds = layout.bounds();
                self.scrollable.update(
                    self.tree, event, layout, cursor, renderer, clipboard, shell, &bounds,
                );
            }
        }
        if shared.reveal.replace(false)
            && let Some(row) = shared.highlight.get()
        {
            let height = row_height(renderer);
            let top = height * row as f32;
            let reveal = &mut Reveal {
                top,
                bottom: top + height,
            };
            self.scrollable.operate(self.tree, layout, renderer, reveal);
            shell.request_redraw();
        }
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.scrollable
            .mouse_interaction(self.tree, layout, cursor, &layout.bounds(), renderer)
    }
}

/// Operation that scrolls the menu as little as it takes to show the span
/// from `top` to `bottom`, in pixels below the top of the first option
struct Reveal {
    top: f32,
    bottom: f32,
}

impl Operation for Reveal {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        _id: Option<&Id>,
        bounds: Rectangle,
        _content_bounds: Rectangle,
        translation: Vector,
        state: &mut dyn operation::Scrollable,
    ) {
        if let Some(offset) = reveal(self.top, self.bottom, translation.y, bounds.height) {
            state.scroll_to(AbsoluteOffset {
                x: None,
                y: Some(offset),
            });
        }
    }
}
