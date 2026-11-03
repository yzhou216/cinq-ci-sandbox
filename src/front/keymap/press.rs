// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Key presses, named as the W3C UI Events specification names keys
//!
//! A key is written as its value in the specification's list of key values,
//! <https://www.w3.org/TR/uievents-key/>: a key that types a character as
//! that character, as in `j`, `G`, or `?`, and any other key by name, as in
//! `Enter`, `Escape`, or `ArrowDown`.  The space bar, whose key value is the
//! space it types, is written `Space`, as the specification's code values
//! name it.  Control, Alt, and Shift go before a key, joined to it by `+`,
//! as in `Control+d` or `Shift+Tab`, and Shift on a letter makes the
//! capital that it types.

mod notation;

use iced::keyboard::{Key, Modifiers, key::Named};
use itertools::Itertools;
use keyboard_types::NamedKey;

pub use notation::{sequence, written};

/// Keys that the specification lists as modifier keys, which change the
/// keys pressed with them rather than act alone.  Hyper and Super are
/// among them, although the specification keeps them only as legacy
/// names, since iced still reports Hyper and a file may name either.
#[expect(deprecated)]
const MODIFIERS: [NamedKey; 14] = [
    NamedKey::Alt,
    NamedKey::AltGraph,
    NamedKey::CapsLock,
    NamedKey::Control,
    NamedKey::Fn,
    NamedKey::FnLock,
    NamedKey::Meta,
    NamedKey::NumLock,
    NamedKey::ScrollLock,
    NamedKey::Shift,
    NamedKey::Symbol,
    NamedKey::SymbolLock,
    NamedKey::Hyper,
    NamedKey::Super,
];

/// One press of a key, with the modifiers held
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Press {
    key: Name,
    ctrl: bool,
    alt: bool,
    /// Shift, which only named keys and the space bar record, since for a
    /// key that types any other character it has already chosen the
    /// character
    shift: bool,
}

/// Key itself, apart from the modifiers held
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Name {
    Char(char),
    Named(NamedKey),
}

impl Press {
    /// Reads a key press as iced reports it, with Shift applied to the
    /// key.  Keys held with the logo key belong to the system, and
    /// modifier keys pressed alone, like keys that the specification does
    /// not name, are left alone.
    pub fn from_key(key: &Key, modifiers: Modifiers) -> Option<Self> {
        if modifiers.logo() {
            return None;
        }
        let (key, shift) = match key {
            Key::Character(text) => (Name::Char(text.chars().exactly_one().ok()?), false),
            Key::Named(named) => match standard(*named)? {
                Name::Named(named) if MODIFIERS.contains(&named) => return None,
                key => (key, modifiers.shift()),
            },
            _ => return None,
        };
        Some(Self {
            key,
            ctrl: modifiers.control(),
            alt: modifiers.alt(),
            shift,
        })
    }

    /// Returns the same key without modifiers
    pub(super) fn bare(self) -> Self {
        Self {
            ctrl: false,
            alt: false,
            shift: false,
            ..self
        }
    }

    /// Returns the value of a digit pressed alone, which counts the keys
    /// after it
    pub(super) fn digit(self) -> Option<usize> {
        match self {
            Self {
                key: Name::Char(c),
                ctrl: false,
                alt: false,
                ..
            } => c.to_digit(10).map(|digit| digit as usize),
            _ => None,
        }
    }
}

/// Returns the key of the specification that iced reports as `named`.
/// iced names its keys after the specification, except that it gives the
/// space bar a name, so the name of any other variant is the name of its
/// key.  Its `Super` is the legacy name of Meta, which [`MODIFIERS`] holds
/// as well.
fn standard(named: Named) -> Option<Name> {
    match named {
        Named::Space => Some(Name::Char(' ')),
        named => format!("{named:?}").parse().ok().map(Name::Named),
    }
}
