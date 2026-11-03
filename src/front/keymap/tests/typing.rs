// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of reading keys typed in a row: counts, commands of several keys,
//! the keys of GO and Panic, and keys that act alone

use super::*;

#[test]
fn counts_repeat_motions() {
    assert_eq!(action("j"), Some((Action::Next, None)));
    assert_eq!(action("1 2 j"), Some((Action::Next, Some(12))));
    assert_eq!(action("3 ArrowUp"), Some((Action::Previous, Some(3))));
    assert_eq!(action("Control+n"), Some((Action::Next, None)));
    assert_eq!(action("2 Control+p"), Some((Action::Previous, Some(2))));
}

#[test]
fn keys_act_alone_when_bound_by_themselves_or_to_go_and_panic() {
    let keymap = Keymap::default();
    let alone = |text: &str| one(text).and_then(|press| keymap.alone(press));
    assert_eq!(alone("Control+n"), Some(Action::Next));
    assert_eq!(alone("Shift+Escape"), Some(Action::Panic));
    assert_eq!(alone("g"), None);
    assert_eq!(alone("x"), None);
}

#[test]
fn enter_acts_at_once_only_when_bound_to_go_or_panic() {
    let enter = || one("Enter").expect("Enter is a named key");
    assert!(!Keymap::default().acts_at_once(enter()));
    let bound = Keymap::default().with(Action::Go, vec![keys("Enter")]);
    assert!(bound.acts_at_once(enter()));
}

#[test]
fn commands_of_two_keys_wait_for_the_second() {
    assert_eq!(typed("2 d").pending, keys("2 d"));
    assert_eq!(action("2 d d"), Some((Action::Delete, Some(2))));
    assert_eq!(action("g g"), Some((Action::First, None)));
    assert_eq!(action("7 G"), Some((Action::Last, Some(7))));
    assert_eq!(action("g T"), Some((Action::PreviousTab, None)));
}

#[test]
fn go_and_panic_act_at_once_with_any_modifiers() {
    let outcome = typed("3 d Space");
    assert_eq!(outcome.action, Some((Action::Go, None)));
    assert!(outcome.pending.is_empty());
    assert_eq!(action("g Control+Escape"), Some((Action::Panic, None)));
}

#[test]
fn key_bound_with_the_modifiers_held_goes_to_its_own_action() {
    let keymap = Keymap::default().with(Action::Panic, vec![keys("Shift+Space")]);
    let alone = |text: &str| one(text).and_then(|press| keymap.alone(press));
    assert_eq!(alone("Shift+Space"), Some(Action::Panic));
    assert_eq!(alone("Control+Space"), Some(Action::Go));
    assert_eq!(alone("Space"), Some(Action::Go));
}

#[test]
fn key_that_breaks_off_a_command_starts_a_new_one() {
    assert_eq!(action("d i"), Some((Action::Edit, None)));
    assert_eq!(action("2 g j"), Some((Action::Next, None)));
    assert_eq!(typed("g x"), typed("x"));
    assert!(typed("4 x").pending.is_empty());
}

#[test]
fn keys_typed_halfway_list_what_can_follow_them() {
    let keymap = Keymap::default();
    let next = |text: &str| -> Vec<(String, Action)> {
        let pending = keys(text);
        keymap
            .continuations(&pending)
            .map(|(rest, action)| (written(rest), action))
            .collect()
    };
    let g = [
        ("g".to_owned(), Action::First),
        ("t".to_owned(), Action::NextTab),
        ("T".to_owned(), Action::PreviousTab),
    ];
    assert_eq!(next("g"), g);
    assert_eq!(next("1 2 g"), g);
    assert_eq!(next("2 d"), [("d".to_owned(), Action::Delete)]);
    assert!(next("3").is_empty());
    assert_eq!(keymap.continuations(&[]).count(), 0);
}
