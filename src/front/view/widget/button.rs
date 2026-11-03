// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Buttons that act as soon as they are pressed
//!
//! iced's own buttons send their message only when the mouse button comes
//! back up, while every other control acts on the press, so an operator
//! holding GO would wait for the cue until letting go.  These buttons keep
//! the look of iced's, but they send their message on the press.  The press
//! never reaches the iced button inside, so it cannot send the message again
//! on release, and a button held down keeps the look of one hovered.

use iced::{
    Element, Event, Length, Theme,
    advanced::{Shell, layout::Layout},
    mouse,
    widget::button::{Status, Style},
};

use super::{
    pressed,
    wrap::{Intercept, Wrap},
};
use crate::front::{Message, view::theme};

/// Button that sends its message when pressed, and stays opaque when
/// disabled
pub struct Button<'a> {
    /// iced's button, which draws this one but never sees a press
    look: iced::widget::Button<'a, Message>,
    on_press: Option<Message>,
}

/// Returns a disabled button that shows `content`, until it has a message
pub fn button<'a>(content: impl Into<Element<'a, Message>>) -> Button<'a> {
    Button {
        look: iced::widget::button(content).style(theme::button),
        on_press: None,
    }
}

impl Button<'_> {
    /// Returns the button sending `message` when pressed
    pub fn on_press(self, message: Message) -> Self {
        self.on_press_maybe(Some(message))
    }

    /// Returns the button sending `message` when pressed, or disabled
    /// without one
    pub fn on_press_maybe(self, message: Option<Message>) -> Self {
        Self {
            look: self.look.on_press_maybe(message.clone()),
            on_press: message,
        }
    }

    /// Returns the button at the width given
    pub fn width(self, width: impl Into<Length>) -> Self {
        Self {
            look: self.look.width(width),
            ..self
        }
    }

    /// Returns the button drawn by `style` in place of the theme's button
    /// colors
    pub fn style(self, style: fn(&Theme, Status) -> Style) -> Self {
        Self {
            look: self.look.style(style),
            ..self
        }
    }
}

impl<'a> From<Button<'a>> for Element<'a, Message> {
    fn from(button: Button<'a>) -> Self {
        Wrap {
            content: button.look.into(),
            intercept: OnPress(button.on_press),
        }
        .into()
    }
}

/// Sends the message of a button on the press, which the iced button
/// inside never sees
struct OnPress(Option<Message>);

impl Intercept for OnPress {
    fn before(
        &self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        shell: &mut Shell<'_, Message>,
    ) -> bool {
        let press = pressed(event);
        if press
            && !shell.is_event_captured()
            && cursor.is_over(layout.bounds())
            && let Some(message) = &self.0
        {
            shell.publish(message.clone());
            shell.capture_event();
        }
        !press
    }
}
