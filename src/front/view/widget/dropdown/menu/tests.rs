// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the keys that move through an open menu, of how far they move
//! its highlight, and of what keys and clicks leave to the widgets under it

use iced::{
    advanced::Shell,
    keyboard::{Key, Modifiers, key::Named},
};

use super::{Choice, Reply, Shared, reply};
use crate::front::{
    Message,
    keymap::{Keymap, Preset, Press},
    step,
};

/// Options of the menus in the tests, each picked as the step it makes
const OPTIONS: [isize; 3] = [1, 2, 3];

/// What a menu does with the events of a test
#[derive(Debug, PartialEq)]
struct Outcome {
    open: bool,
    /// Whether the menu keeps the events from the widgets under it
    captured: bool,
    picked: Option<isize>,
}

/// Opens a menu of [`OPTIONS`] with the highlight on the first, lets `act`
/// hand it events, and returns what they amount to
fn outcome(act: impl Fn(Choice<'_, isize>, &mut Shell<'_, Message>)) -> Outcome {
    let keymap = Keymap::default();
    let shared = Shared::default();
    let on_select = Message::Step;
    let choice = Choice::new(&OPTIONS, &on_select, &keymap, &shared);
    choice.open(Some(0));
    let mut messages = Vec::new();
    let mut shell = Shell::new(&mut messages);
    act(choice, &mut shell);
    let captured = shell.is_event_captured();
    let picked = messages.iter().find_map(|message| match message {
        Message::Step(option) => Some(*option),
        _ => None,
    });
    Outcome {
        open: shared.is_open(),
        captured,
        picked,
    }
}

/// Presses `key` alone in a menu
fn press(choice: Choice<'_, isize>, key: Key, shell: &mut Shell<'_, Message>) {
    choice.press(&key, Press::from_key(&key, Modifiers::empty()), shell);
}

#[test]
fn keys_move_pick_close_or_wait_as_the_keymap_binds_them() {
    let (none, ctrl) = (Modifiers::empty(), Modifiers::CTRL);
    let (letter, named) = (|text: &str| Key::Character(text.into()), Key::Named);
    let cases = [
        (Preset::Vi, letter("n"), ctrl, Reply::Step(1)),
        (Preset::Vi, letter("p"), ctrl, Reply::Step(-1)),
        (Preset::Vi, named(Named::ArrowDown), none, Reply::Step(1)),
        (Preset::Vi, letter("k"), none, Reply::Step(-1)),
        (Preset::Cua, named(Named::ArrowUp), none, Reply::Step(-1)),
        (Preset::Cua, letter("n"), ctrl, Reply::Keep),
        (Preset::Vi, named(Named::Enter), none, Reply::Pick),
        (Preset::Vi, named(Named::Escape), none, Reply::Close),
        (Preset::Vi, named(Named::Space), none, Reply::Close),
        (Preset::Vi, letter("n"), none, Reply::Keep),
        (Preset::Vi, letter("d"), none, Reply::Keep),
    ];
    let wrong: Vec<_> = cases
        .into_iter()
        .map(|(preset, key, modifiers, expected)| {
            let press = Press::from_key(&key, modifiers);
            let got = reply(&Keymap::from(preset), &key, press);
            (preset, key, modifiers, expected, got)
        })
        .filter(|(.., expected, got)| expected != got)
        .collect();
    assert!(
        wrong.is_empty(),
        "expected one reply but got another: {wrong:#?}"
    );
}

#[test]
fn highlight_stays_put_past_either_end() {
    assert_eq!(step(None, -1, 3), Some(0));
    assert_eq!(step(Some(1), 1, 3), Some(2));
    assert_eq!(step(Some(2), 1, 3), Some(2));
    assert_eq!(step(Some(0), -1, 3), Some(0));
    assert_eq!(step(Some(0), 1, 0), None);
}

#[test]
fn enter_picks_the_highlight_and_go_closes_the_menu_without_keeping_its_key() {
    let (letter, named) = (|text: &str| Key::Character(text.into()), Key::Named);
    let closed = |captured, picked| Outcome {
        open: false,
        captured,
        picked,
    };
    assert_eq!(
        outcome(|choice, shell| press(choice, named(Named::Enter), shell)),
        closed(true, Some(1))
    );
    assert_eq!(
        outcome(|choice, shell| {
            press(choice, letter("j"), shell);
            press(choice, named(Named::Enter), shell);
        }),
        closed(true, Some(2))
    );
    assert_eq!(
        outcome(|choice, shell| press(choice, named(Named::Space), shell)),
        closed(false, None)
    );
    assert_eq!(
        outcome(|choice, shell| press(choice, letter("d"), shell)),
        Outcome {
            open: true,
            captured: true,
            picked: None,
        }
    );
}

#[test]
fn click_beside_the_menu_closes_it_and_acts_unless_it_lands_on_the_list() {
    let closed = |captured| Outcome {
        open: false,
        captured,
        picked: None,
    };
    assert_eq!(
        outcome(|choice, shell| choice.click_beside(false, shell)),
        closed(false)
    );
    assert_eq!(
        outcome(|choice, shell| choice.click_beside(true, shell)),
        closed(true)
    );
}
