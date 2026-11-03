// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Reading and writing presses as the configuration file writes them

use std::fmt;

use itertools::Itertools;
use keyboard_types::{Key, NamedKey};

use super::{MODIFIERS, Name, Press};

/// Name of the space bar, whose key value would read as the space between
/// two keys
const SPACE: &str = "Space";

/// Modifiers that a press can hold, in the order they are written
const HELD: [NamedKey; 3] = [NamedKey::Control, NamedKey::Alt, NamedKey::Shift];

impl Press {
    /// Reads one press, such as `j`, `Enter`, or `Control+d`
    fn read(token: &str) -> Result<Self, String> {
        let (held, key) = modifiers(token)?;
        let key = match key {
            SPACE => Some(Name::Char(' ')),
            key => match key.parse::<Key>() {
                Ok(Key::Named(named)) if MODIFIERS.contains(&named) => {
                    return Err(format!("`{token}` only changes the keys pressed with it"));
                }
                Ok(Key::Named(named)) => Some(Name::Named(named)),
                Ok(Key::Character(text)) => text.chars().exactly_one().ok().map(Name::Char),
                Err(_) => None,
            },
        }
        .ok_or_else(|| format!("`{token}` names no key"))?;
        let shift = held.contains(&NamedKey::Shift);
        // Shift on a letter makes the capital, which is the key it types
        let (key, shift) = match key {
            Name::Char(c) if shift && c.is_ascii_alphabetic() => {
                (Name::Char(c.to_ascii_uppercase()), false)
            }
            Name::Char(c) if shift && c != ' ' => {
                return Err(format!("`{token}` is written as the character it types"));
            }
            key => (key, shift),
        };
        Ok(Self {
            key,
            ctrl: held.contains(&NamedKey::Control),
            alt: held.contains(&NamedKey::Alt),
            shift,
        })
    }
}

impl fmt::Display for Press {
    /// Writes the press as the configuration file does
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        HELD.iter()
            .zip([self.ctrl, self.alt, self.shift])
            .filter(|(_, held)| *held)
            .try_for_each(|(modifier, _)| write!(f, "{modifier}+"))?;
        match self.key {
            Name::Char(' ') => f.write_str(SPACE),
            Name::Char(c) => write!(f, "{c}"),
            Name::Named(named) => write!(f, "{named}"),
        }
    }
}

/// Reads a sequence of presses separated by spaces, such as `g g` or
/// `Control+w j`
pub fn sequence(text: &str) -> Result<Vec<Press>, String> {
    let presses = text
        .split_whitespace()
        .map(Press::read)
        .collect::<Result<Vec<_>, _>>()?;
    if presses.is_empty() {
        Err("a binding needs at least one key".to_owned())
    } else {
        Ok(presses)
    }
}

/// Writes a sequence of presses as the configuration file does
pub fn written(presses: &[Press]) -> String {
    presses.iter().join(" ")
}

/// Splits the modifiers that a press holds off its key, as in `Control+d`.
/// A `+` with nothing after it is the key itself.
fn modifiers(token: &str) -> Result<(Vec<NamedKey>, &str), String> {
    match token.split_once('+') {
        Some((name, key)) if !key.is_empty() => match name.parse::<NamedKey>() {
            Ok(modifier) if HELD.contains(&modifier) => {
                let (held, key) = modifiers(key)?;
                Ok(([vec![modifier], held].concat(), key))
            }
            Ok(_) => Err(format!(
                "`{token}` holds `{name}`, but only Control, Alt, and Shift can be held"
            )),
            Err(_) => Ok((Vec::new(), token)),
        },
        _ => Ok((Vec::new(), token)),
    }
}
