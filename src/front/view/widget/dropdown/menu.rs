// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Menu of a drop-down list, which opens below the list or above it,
//! wherever there is more room, and scrolls when the options do not fit
//!
//! The options sit in iced's scrollable, so the menu scrolls and shows its
//! scroll bar as iced's own menu does.  The mouse highlights the option
//! under it and picks the option it presses, and the keys move the
//! highlight, which the menu scrolls into view.  A click beside the menu
//! closes it and acts as it would with the menu closed, apart from a click
//! on the list, which only closes the menu.
//!
//! iced hands each batch of events first to the menu that is open when the
//! batch begins, then to the widgets under it, so a menu that closes
//! partway through a batch leaves the rest of the batch to them.

#[cfg(test)]
mod tests;

use std::cell::Cell;

use iced::{
    Element, Event, Length, Point, Rectangle, Renderer, Size, Theme, Vector,
    advanced::{
        Clipboard, Overlay, Renderer as _, Shell,
        layout::{self, Layout},
        mouse, renderer,
        text::Renderer as _,
        widget::{
            Id, Tree, Widget,
            operation::{self, Operation},
        },
    },
    border,
    keyboard::{self, Key, key::Named},
    overlay, padding,
    widget::{Scrollable, operation::AbsoluteOffset},
};

use super::{PADDING, label, line_height, start};
use crate::front::{
    Message,
    keymap::{Action, Keymap, Press},
    reveal, step,
    view::widget::{pointer_over, pressed},
};

/// Whether a menu is open and where its highlight is, which the drop-down
/// list keeps for the menu
#[derive(Default)]
pub(super) struct Shared {
    open: Cell<bool>,
    /// Option under the highlight, which the mouse moves as well as the keys
    highlight: Cell<Option<usize>>,
    /// Whether the menu has yet to scroll the highlight into view
    reveal: Cell<bool>,
}

impl Shared {
    pub(super) fn is_open(&self) -> bool {
        self.open.get()
    }
}

/// Options of a drop-down list with the highlight of its menu, through
/// which the list and the menu act on the menu
pub(super) struct Choice<'b, T> {
    options: &'b [T],
    on_select: &'b dyn Fn(T) -> Message,
    keymap: &'b Keymap,
    shared: &'b Shared,
}

impl<T> Clone for Choice<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Choice<'_, T> {}

impl<'b, T> Choice<'b, T> {
    pub(super) fn new(
        options: &'b [T],
        on_select: &'b dyn Fn(T) -> Message,
        keymap: &'b Keymap,
        shared: &'b Shared,
    ) -> Self {
        Self {
            options,
            on_select,
            keymap,
            shared,
        }
    }
}

impl<T: Clone> Choice<'_, T> {
    /// Opens the menu with the highlight on `row`, which it scrolls into
    /// view
    pub(super) fn open(&self, row: Option<usize>) {
        self.shared.open.set(true);
        self.shared.highlight.set(row);
        self.shared.reveal.set(true);
    }

    /// Carries out what a key does while the menu is open
    pub(super) fn press(&self, key: &Key, press: Option<Press>, shell: &mut Shell<'_, Message>) {
        match reply(self.keymap, key, press) {
            Reply::Step(offset) => {
                let row = step(self.shared.highlight.get(), offset, self.options.len());
                self.shared.highlight.set(row);
                self.shared.reveal.set(true);
                shell.capture_event();
            }
            Reply::Pick => self.pick(self.shared.highlight.get(), shell),
            Reply::Close => self.shared.open.set(false),
            Reply::Keep => shell.capture_event(),
        }
        shell.request_redraw();
    }

    /// Closes the menu on a click beside it, which goes on to act on what it
    /// lands on, unless it lands on the list, whose menu it would open again
    pub(super) fn click_beside(&self, on_list: bool, shell: &mut Shell<'_, Message>) {
        self.shared.open.set(false);
        if on_list {
            shell.capture_event();
        }
        shell.request_redraw();
    }

    /// Sends the message of the option in `row`, if there is one, and
    /// closes the menu
    fn pick(&self, row: Option<usize>, shell: &mut Shell<'_, Message>) {
        if let Some(option) = row.and_then(|row| self.options.get(row)) {
            shell.publish((self.on_select)(option.clone()));
        }
        self.shared.open.set(false);
        shell.capture_event();
    }
}

/// What a key does while the menu is open
#[derive(Debug, PartialEq)]
enum Reply {
    /// Moves the highlight by a number of options, downwards when positive
    Step(isize),
    /// Picks the highlighted option, which closes the menu
    Pick,
    /// Lets the key act and closes the menu, as the keys of GO and Panic do
    Close,
    /// Keeps the key from acting on anything behind the menu
    Keep,
}

/// Decides what a key does while the menu is open, given the press it
/// makes, when the key bindings can name it
fn reply(keymap: &Keymap, key: &Key, press: Option<Press>) -> Reply {
    match (press.and_then(|press| keymap.alone(press)), key) {
        (Some(Action::Next), _) => Reply::Step(1),
        (Some(Action::Previous), _) => Reply::Step(-1),
        (Some(action), _) if action.acts_at_once() => Reply::Close,
        (_, Key::Named(Named::Enter)) => Reply::Pick,
        _ => Reply::Keep,
    }
}

/// Menu of the options of a drop-down list
pub(super) struct Menu<'b, T> {
    choice: Choice<'b, T>,
    /// State of the scrollable, kept in the state of the list
    tree: &'b mut Tree,
    scrollable: Scrollable<'b, Message>,
    /// Bounds of the drop-down list, which the menu opens next to
    target: Rectangle,
}

impl<'b, T: ToString + Clone + 'b> Menu<'b, T> {
    pub(super) fn new(choice: Choice<'b, T>, tree: &'b mut Tree, target: Rectangle) -> Self {
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
            let mut reveal = Reveal {
                top,
                bottom: top + height,
            };
            self.scrollable
                .operate(self.tree, layout, renderer, &mut reveal);
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

/// Options of a menu, one to a row
struct Rows<'b, T> {
    choice: Choice<'b, T>,
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
fn row_height(renderer: &Renderer) -> f32 {
    line_height(renderer) + PADDING.y()
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
