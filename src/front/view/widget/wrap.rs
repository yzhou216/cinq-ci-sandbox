// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Widget that lays out and draws its content unchanged, around which an
//! [`Intercept`] handles events before and after the content sees them,
//! draws under the content, and tells operations about it

use iced::{
    Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector,
    advanced::{
        Clipboard, Shell,
        layout::{self, Layout},
        mouse, overlay, renderer,
        widget::{
            Operation, Widget,
            tree::{self, Tree},
        },
    },
};

use crate::front::Message;

/// What a [`Wrap`] does with an event around its content
pub trait Intercept {
    /// Handles an event before the content sees it, and returns whether
    /// the content sees it at all
    fn before(
        &self,
        _event: &Event,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _shell: &mut Shell<'_, Message>,
    ) -> bool {
        true
    }

    /// Handles an event after the content has seen it
    fn after(
        &self,
        _content: &mut Element<'_, Message>,
        _tree: &mut Tree,
        _event: &Event,
        _layout: Layout<'_>,
        _renderer: &Renderer,
        _shell: &mut Shell<'_, Message>,
    ) {
    }

    /// Draws under the content, which draws itself over what this draws
    fn under(&self, _layout: Layout<'_>, _renderer: &mut Renderer) {}

    /// Tells an operation about the content, before the content tells it
    /// about itself
    fn operate(&self, _layout: Layout<'_>, _operation: &mut dyn Operation) {}
}

/// Content with an [`Intercept`] around its events
pub struct Wrap<'a, I> {
    pub content: Element<'a, Message>,
    pub intercept: I,
}

impl<'a, I: Intercept + 'a> From<Wrap<'a, I>> for Element<'a, Message> {
    fn from(wrap: Wrap<'a, I>) -> Self {
        Element::new(wrap)
    }
}

impl<I: Intercept> Widget<Message, Theme, Renderer> for Wrap<'_, I> {
    fn tag(&self) -> tree::Tag {
        self.content.as_widget().tag()
    }

    fn state(&self) -> tree::State {
        self.content.as_widget().state()
    }

    fn children(&self) -> Vec<Tree> {
        self.content.as_widget().children()
    }

    fn diff(&self, tree: &mut Tree) {
        self.content.as_widget().diff(tree);
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content.as_widget_mut().layout(tree, renderer, limits)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.intercept.under(layout, renderer);
        self.content
            .as_widget()
            .draw(tree, renderer, theme, style, layout, cursor, viewport);
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.intercept.operate(layout, operation);
        self.content
            .as_widget_mut()
            .operate(tree, layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        if self.intercept.before(event, layout, cursor, shell) {
            self.content.as_widget_mut().update(
                tree, event, layout, cursor, renderer, clipboard, shell, viewport,
            );
            self.intercept
                .after(&mut self.content, tree, event, layout, renderer, shell);
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content
            .as_widget()
            .mouse_interaction(tree, layout, cursor, viewport, renderer)
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content
            .as_widget_mut()
            .overlay(tree, layout, renderer, viewport, translation)
    }
}
