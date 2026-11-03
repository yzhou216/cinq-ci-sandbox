// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of how keys are written, and of how the keys that iced reports
//! match what is written

use iced::keyboard::{Key, Modifiers, key::Named};

use super::*;

#[test]
fn notation_writes_keys_the_way_it_reads_them() {
    let written_as = |text: &str| written(&keys(text));
    assert_eq!(
        written_as("Space Enter  Escape F35"),
        "Space Enter Escape F35"
    );
    assert_eq!(
        written_as("Control+d Alt+x Shift+Tab"),
        "Control+d Alt+x Shift+Tab"
    );
    assert_eq!(
        written_as("Alt+Control+ArrowUp Control++"),
        "Control+Alt+ArrowUp Control++"
    );
    assert_eq!(
        written_as("Shift+a G < + Shift+Space"),
        "A G < + Shift+Space"
    );
}

#[test]
fn notation_rejects_what_cannot_be_bound() {
    let accepted: Vec<&str> = [
        "", "Foo", "escape", "gg", "Control+", "Shift", "Super", "Meta+x", "Shift+1",
    ]
    .into_iter()
    .filter(|text| sequence(text).is_ok())
    .collect();
    assert!(accepted.is_empty(), "{accepted:?}");
}

#[test]
fn keys_from_iced_match_the_names_of_the_specification() {
    let none = Modifiers::empty();
    let named = |named: Named, modifiers| Press::from_key(&Key::Named(named), modifiers);
    assert_eq!(named(Named::ArrowDown, none), one("ArrowDown"));
    assert_eq!(named(Named::MediaPlayPause, none), one("MediaPlayPause"));
    assert_eq!(named(Named::Space, none), one("Space"));
    assert_eq!(named(Named::Space, Modifiers::SHIFT), one("Shift+Space"));
    assert_eq!(named(Named::Tab, Modifiers::SHIFT), one("Shift+Tab"));
    assert_eq!(
        Press::from_key(&Key::Character("G".into()), Modifiers::SHIFT),
        one("G")
    );
    assert_eq!(named(Named::Shift, Modifiers::SHIFT), None);
    assert_eq!(named(Named::Super, none), None);
}
