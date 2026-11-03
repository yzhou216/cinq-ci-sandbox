// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Region of fields that keeps the keys typed into them
//!
//! iced's text fields let some keys through, such as the arrows, and a
//! read-only field lets every key through, as does a drop-down list apart
//! from Enter, which opens its menu.  Those keys would reach the key
//! bindings and act on the cue list while the operator types.  While one of
//! the fields has the keyboard focus, this region keeps the keys that the
//! field let through, apart from the keys that act in fields, which are
//! those of GO and Panic, those that move between fields, and those that
//! open and save shows.  Enter leaves a text field.  A field that takes a
//! key of GO or Panic without typing it, as iced's text fields take Escape
//! to give up the keyboard, leaves the key to act all the same.  The region
//! decides as each key arrives, so keys act in the order the operator typed
//! them.

use iced::{
    Element, Event, Rectangle, Renderer,
    advanced::{
        Shell,
        layout::Layout,
        widget::{Id, Operation, Tree, operation::Focusable},
    },
    keyboard::{self, Key, key::Named},
};

use super::wrap::{Intercept, Wrap};
use crate::front::{
    Focus, Message,
    keymap::{Keymap, Press},
};

/// Keeps the keys typed into the fields of `content`, apart from the keys
/// that `keymap` lets act in fields
pub fn keep_keys<'a>(
    content: impl Into<Element<'a, Message>>,
    keymap: &'a Keymap,
) -> Element<'a, Message> {
    Wrap {
        content: content.into(),
        intercept: Keep(keymap),
    }
    .into()
}

/// Keeps the keys that a focused field let through
struct Keep<'a>(&'a Keymap);

impl Intercept for Keep<'_> {
    fn after(
        &self,
        content: &mut Element<'_, Message>,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        renderer: &Renderer,
        shell: &mut Shell<'_, Message>,
    ) {
        let Event::Keyboard(keyboard::Event::KeyPressed {
            key,
            modified_key,
            modifiers,
            repeat,
            ..
        }) = event
        else {
            return;
        };
        let press = Press::from_key(modified_key, *modifiers);
        if shell.is_event_captured() {
            let typed = matches!(key, Key::Character(_) | Key::Named(Named::Space));
            let at_once = press.filter(|&press| !typed && self.0.acts_at_once(press));
            if let Some(press) = at_once {
                shell.publish(Message::Typed {
                    press,
                    repeat: *repeat,
                });
            }
            return;
        }
        let mut focus = HasFocus(false);
        content
            .as_widget_mut()
            .operate(tree, layout, renderer, &mut focus);
        if !focus.0 {
            return;
        }
        let acts = press.is_some_and(|press| self.0.acts_in_fields(press));
        match key {
            _ if acts => {}
            Key::Named(Named::Enter) => {
                shell.publish(Message::Focus(Focus::Cues));
                shell.capture_event();
            }
            _ => shell.capture_event(),
        }
    }
}

/// Operation that finds out whether a text field or a drop-down list has
/// the keyboard focus, read-only fields included
struct HasFocus(bool);

impl Operation for HasFocus {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn focusable(&mut self, _id: Option<&Id>, _bounds: Rectangle, state: &mut dyn Focusable) {
        self.0 |= state.is_focused();
    }
}
