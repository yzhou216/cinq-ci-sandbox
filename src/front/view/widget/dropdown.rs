// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Drop-down lists whose menus the keys move through
//!
//! iced's pick list takes no keys, so keys pressed over its open menu act
//! on the cue list behind it.  These lists look like iced's pick list but
//! open a [`menu`] of their own, in which the keys of [`Action::Next`] and
//! [`Action::Previous`] move the highlight, which the menu keeps in view,
//! and Enter picks the highlighted option.  GO and Panic act as everywhere
//! and close the menu, so that the show is in view, and the menu keeps
//! every other key until it closes.
//!
//! The keys that move between fields move to these lists as well, which
//! then show the border of a focused field, and Enter opens the menu, which
//! takes the keys from there, unless Enter is bound to GO or Panic, which
//! act as everywhere.  The list hands the keyboard to its menu as it
//! opens, so that the keys act on the cue list again once the menu closes,
//! as they do after Enter leaves a text field.
//!
//! [`Action::Next`]: crate::front::keymap::Action::Next
//! [`Action::Previous`]: crate::front::keymap::Action::Previous

mod menu;

use iced::{
    Element, Event, Length, Padding, Point, Rectangle, Renderer, Size, Theme, Vector,
    advanced::{
        Clipboard, Renderer as _, Shell,
        layout::{self, Layout},
        mouse, overlay, renderer,
        text::{self, Renderer as _, Text, paragraph},
        widget::{
            Operation, Widget,
            operation::Focusable,
            tree::{self, Tree},
        },
    },
    alignment,
    keyboard::{self, Key, key::Named},
    widget::{
        button,
        pick_list::{self, Status},
        text_input,
    },
    window,
};

use super::{pointer_over, pressed};
use crate::front::{
    Message,
    keymap::{Keymap, Press},
    view::Paragraph,
};
use menu::{Choice, Menu, Shared};

/// Space around the text of the list and of each option of its menu, which
/// iced's buttons and pick lists leave too
const PADDING: Padding = button::DEFAULT_PADDING;

/// Drop-down list of options, one of which may be chosen
pub struct Dropdown<'a, T> {
    options: Vec<T>,
    selected: Option<T>,
    on_select: Box<dyn Fn(T) -> Message + 'a>,
    placeholder: Option<String>,
    keymap: &'a Keymap,
}

/// Returns a drop-down list of `options` with `selected` chosen, which
/// sends the message that `on_select` makes of the option picked.  The keys
/// of `keymap` move through its menu.
pub fn dropdown<'a, T>(
    options: impl Into<Vec<T>>,
    selected: Option<T>,
    on_select: impl Fn(T) -> Message + 'a,
    keymap: &'a Keymap,
) -> Dropdown<'a, T> {
    Dropdown {
        options: options.into(),
        selected,
        on_select: Box::new(on_select),
        placeholder: None,
        keymap,
    }
}

impl<T: PartialEq> Dropdown<'_, T> {
    /// Returns the list showing `placeholder` while no option is chosen
    pub fn placeholder(self, placeholder: impl Into<String>) -> Self {
        Self {
            placeholder: Some(placeholder.into()),
            ..self
        }
    }

    /// Opens the menu with the highlight on the chosen option, unless there
    /// is no option, and keeps the event that opens it
    fn open(&self, choice: Choice<'_, T>, shell: &mut Shell<'_, Message>)
    where
        T: Clone,
    {
        if !self.options.is_empty() {
            choice.open(self.chosen());
        }
        shell.capture_event();
        shell.request_redraw();
    }

    /// Returns the place of the chosen option among the options
    fn chosen(&self) -> Option<usize> {
        self.options
            .iter()
            .position(|option| Some(option) == self.selected.as_ref())
    }

    /// Returns the options together with the highlight of the menu, through
    /// which the list acts on its menu
    fn choice<'b>(&'b self, shared: &'b Shared) -> Choice<'b, T> {
        Choice::new(&self.options, &*self.on_select, self.keymap, shared)
    }
}

impl<'a, T: ToString + PartialEq + Clone + 'a> From<Dropdown<'a, T>> for Element<'a, Message> {
    fn from(dropdown: Dropdown<'a, T>) -> Self {
        Element::new(dropdown)
    }
}

/// State of a drop-down list
struct State {
    /// Whether the menu is open and where its highlight is, which the menu
    /// shares while it is open
    shared: Shared,
    /// Whether the list has the keyboard, which it has only until its menu
    /// opens
    focused: bool,
    /// Whether the cursor was over the list in the last frame, so that the
    /// list redraws when its look changes
    over: bool,
    /// Labels of the options and the placeholder, which set the width
    labels: Vec<paragraph::Plain<Paragraph>>,
    placeholder: paragraph::Plain<Paragraph>,
    /// State of the widgets of the menu, which outlives each opening
    menu: Tree,
}

impl Focusable for State {
    fn is_focused(&self) -> bool {
        self.focused
    }

    fn focus(&mut self) {
        self.focused = true;
    }

    fn unfocus(&mut self) {
        self.focused = false;
    }
}

/// Returns the height of a line of text in the list and its menu
fn line_height(renderer: &Renderer) -> f32 {
    f32::from(text::LineHeight::default().to_absolute(renderer.default_size()))
}

/// Returns a label on one line, in the font and size of the list and its
/// menu, to draw from its top left corner at [`start`]
fn label<C>(renderer: &Renderer, content: C, width: f32) -> Text<C> {
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
fn start(renderer: &Renderer, bounds: Rectangle) -> Point {
    Point::new(
        bounds.x + PADDING.left,
        bounds.center_y() - line_height(renderer) / 2.0,
    )
}

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
