// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Drop-down list as an iced widget, which lays itself out, draws itself in
//! the look of iced's pick list, and opens its menu

use iced::{
    Event, Length, Point, Rectangle, Renderer, Size, Theme, Vector,
    advanced::{
        Clipboard, Renderer as _, Shell,
        layout::{self, Layout},
        mouse, overlay, renderer,
        text::{self, Renderer as _, Text, paragraph},
        widget::{
            Operation, Widget,
            tree::{self, Tree},
        },
    },
    alignment,
    keyboard::{self, Key, key::Named},
    widget::{
        pick_list::{self, Status},
        text_input,
    },
    window,
};

use super::{
    Dropdown, PADDING,
    label::{label, line_height, start},
    menu::{Menu, Shared},
    state::State,
};
use crate::front::{
    Message,
    keymap::Press,
    view::{
        Paragraph,
        widget::{pointer_over, pressed},
    },
};

impl<'a, T: ToString + PartialEq + Clone + 'a> Widget<Message, Theme, Renderer>
    for Dropdown<'a, T>
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State {
            shared: Shared::default(),
            focused: false,
            over: false,
            labels: Vec::new(),
            placeholder: paragraph::Plain::default(),
            menu: Tree::empty(),
        })
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Shrink, Length::Shrink)
    }

    /// Makes the list as wide as its widest label, as iced's pick list is,
    /// so that its width stays put whichever option is chosen
    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let state = tree.state.downcast_mut::<State>();
        let width = |plain: &mut paragraph::Plain<Paragraph>, name: &str| {
            plain.update(label(renderer, name, f32::INFINITY));
            plain.min_width()
        };
        state
            .labels
            .resize_with(self.options.len(), Default::default);
        let widest = state
            .labels
            .iter_mut()
            .zip(&self.options)
            .map(|(plain, option)| width(plain, &option.to_string()))
            .chain(
                self.placeholder
                    .as_deref()
                    .map(|placeholder| width(&mut state.placeholder, placeholder)),
            )
            .fold(0.0, f32::max);
        // Room for the arrow, which is about as wide as the text is high
        let arrow = renderer.default_size().0 + PADDING.left;
        let intrinsic = Size::new(widest + arrow, line_height(renderer));
        layout::Node::new(
            limits
                .shrink(PADDING)
                .resolve(Length::Shrink, Length::Shrink, intrinsic)
                .expand(PADDING),
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();
        let bounds = layout.bounds();
        let is_hovered = cursor.is_over(bounds);
        let status = match (state.shared.is_open(), is_hovered) {
            (true, _) => Status::Opened { is_hovered },
            (false, true) => Status::Hovered,
            (false, false) => Status::Active,
        };
        let look = pick_list::default(theme, status);
        // Border of a focused list is that of a focused text field
        let border = if state.focused {
            let focused = text_input::Status::Focused { is_hovered };
            look.border
                .color(text_input::default(theme, focused).border.color)
        } else {
            look.border
        };
        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border,
                ..renderer::Quad::default()
            },
            look.background,
        );
        // Arrow drawn by its right end as iced's pick list draws it, since
        // it never changes apart from the box, which redraws around it
        let arrow = Text {
            font: Renderer::ICON_FONT,
            align_x: text::Alignment::Right,
            align_y: alignment::Vertical::Center,
            shaping: text::Shaping::Basic,
            ..label(
                renderer,
                Renderer::ARROW_DOWN_ICON.to_string(),
                bounds.width,
            )
        };
        let end = Point::new(bounds.x + bounds.width - PADDING.right, bounds.center_y());
        renderer.fill_text(arrow, end, look.handle_color, *viewport);
        let (name, color) = match &self.selected {
            Some(option) => (option.to_string(), look.text_color),
            None => (
                self.placeholder.clone().unwrap_or_default(),
                look.placeholder_color,
            ),
        };
        let name = label(renderer, name, bounds.width - PADDING.x());
        renderer.fill_text(name, start(renderer, bounds), color, *viewport);
    }

    /// Opens the menu on a click, or on Enter while the list has the
    /// keyboard, which any click takes away, as it does from a text field.
    /// Once open, the menu takes the keys and clicks itself, apart from the
    /// rest of the batch of events in which it opens, since iced hands that
    /// out before it has the menu, and the list acts on the menu's
    /// [`Choice`] for those.
    ///
    /// [`Choice`]: super::menu::Choice
    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<State>();
        let over = cursor.is_over(layout.bounds());
        if pressed(event) && state.focused {
            state.focused = false;
            shell.request_redraw();
        }
        let choice = self.choice(&state.shared);
        match event {
            Event::Keyboard(keyboard::Event::KeyPressed {
                key,
                modified_key,
                modifiers,
                ..
            }) if state.shared.is_open() => {
                choice.press(key, Press::from_key(modified_key, *modifiers), shell);
            }
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: Key::Named(Named::Enter),
                modified_key,
                modifiers,
                ..
            }) if state.focused
                && !Press::from_key(modified_key, *modifiers)
                    .is_some_and(|press| self.keymap.acts_at_once(press)) =>
            {
                state.focused = false;
                self.open(choice, shell);
            }
            _ if pressed(event) && state.shared.is_open() => choice.click_beside(over, shell),
            _ if pressed(event) && over => self.open(choice, shell),
            Event::Window(window::Event::RedrawRequested(_)) => state.over = over,
            _ if over != state.over => shell.request_redraw(),
            _ => {}
        }
    }

    /// Lets the keys that move between fields move to the list
    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        _renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.focusable(None, layout.bounds(), tree.state.downcast_mut::<State>());
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

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        _renderer: &Renderer,
        _viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let State { shared, menu, .. } = tree.state.downcast_mut::<State>();
        if !shared.is_open() {
            return None;
        }
        let target = Rectangle::new(layout.position() + translation, layout.bounds().size());
        let menu = Menu::new(self.choice(shared), menu, target);
        Some(overlay::Element::new(Box::new(menu)))
    }
}
