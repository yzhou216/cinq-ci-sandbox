// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Key bindings, which turn key presses into the messages that buttons send
//!
//! Every keymap in [`preset`] binds keys to every action.  The configuration
//! file in [`config`] chooses one, or builds keymaps of its own, and can
//! replace the keys of any action, and [`press`] names keys as the W3C UI
//! Events specification does.  [`typing`] reads a command of several keys,
//! such as a count followed by a key, once it is complete, and the status
//! bar shows the keys typed so far.  The keys bound to GO and Panic act at
//! once, whatever was typed before them and whatever modifiers are held, so
//! that nothing holds them back.

mod action;
pub mod config;
mod preset;
mod press;
#[cfg(test)]
mod tests;
mod typing;

use std::collections::HashMap;

use itertools::Itertools;
use strum::VariantArray;

pub use action::Action;
pub use preset::Preset;
pub use press::{Press, written};

/// Keys bound to each action
#[derive(Debug, Clone)]
pub struct Keymap {
    /// Name of the keymap, a shipped one or one that the configuration
    /// file defines
    name: String,
    /// Key sequences bound to each action, which may be none
    bindings: HashMap<Action, Vec<Vec<Press>>>,
}

impl From<Preset> for Keymap {
    fn from(preset: Preset) -> Self {
        Action::VARIANTS
            .iter()
            .fold(Self::empty(preset.name()), |keymap, &action| {
                let keys = preset
                    .keys(action)
                    .iter()
                    .filter_map(|keys| binding(keys).ok());
                keymap.with(action, keys.collect())
            })
    }
}

impl Default for Keymap {
    fn default() -> Self {
        Preset::default().into()
    }
}

impl Keymap {
    /// Returns a keymap named `name` that binds no keys at all
    fn empty(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            bindings: HashMap::new(),
        }
    }

    /// Returns the keymap under the name `name`, with the same keys
    fn named(self, name: &str) -> Self {
        Self {
            name: name.to_owned(),
            ..self
        }
    }

    /// Returns the keymap with the keys of `action` replaced by `keys`
    fn with(self, action: Action, keys: Vec<Vec<Press>>) -> Self {
        let mut bindings = self.bindings;
        bindings.insert(action, keys);
        Self { bindings, ..self }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the key sequences bound to an action
    fn keys(&self, action: Action) -> &[Vec<Press>] {
        self.bindings.get(&action).map_or(&[], Vec::as_slice)
    }

    /// Writes the first key sequence bound to an action, which stands for
    /// the rest where there is room for one
    pub fn key(&self, action: Action) -> Option<String> {
        self.keys(action).first().map(|keys| written(keys))
    }

    /// Writes the first key sequence bound to an action together with
    /// what it does, as in `Control+s: save`
    pub fn hint(&self, action: Action, label: &str) -> Option<String> {
        self.key(action).map(|keys| format!("{keys}: {label}"))
    }

    /// Writes the key sequences bound to an action as a list, or `none`
    pub fn listed(&self, action: Action) -> String {
        match self.keys(action) {
            [] => "none".to_owned(),
            keys => keys.iter().map(|keys| written(keys)).join(", "),
        }
    }

    /// Describes the bindings that hide others: a sequence bound to two
    /// actions, which only the first gets, a sequence that begins a longer
    /// one, which can then never be typed, and a key of
    /// [`Action::AT_ONCE`], which keeps every other sequence that holds it
    /// from being typed
    fn conflicts(&self) -> Vec<String> {
        let hidden = self
            .bound()
            .array_combinations()
            .filter_map(|[earlier, later]| conflict(earlier, later));
        let cut_short = self.bound().filter_map(|(_, keys)| self.cut_short(keys));
        hidden.chain(cut_short).collect()
    }

    /// Describes how a key of [`Action::AT_ONCE`] keeps `keys` from being
    /// typed, if one does, by acting halfway through them or when held with
    /// other modifiers than it is bound with.  Keys that begin with the
    /// very key bound are the binding of the key itself, or one that
    /// [`conflict`] describes.
    fn cut_short(&self, keys: &[Press]) -> Option<String> {
        let (action, key) = keys.iter().find_map(|&press| self.at_once_bound(press))?;
        (keys[0] != key).then(|| {
            format!(
                "`{}` acts at once as {}, so `{}` can never be typed",
                written(&[key]),
                action.name(),
                written(keys)
            )
        })
    }

    /// Returns every binding, in the order of [`Action::VARIANTS`]
    fn bound(&self) -> impl Iterator<Item = Binding<'_>> + Clone {
        Action::VARIANTS.iter().flat_map(move |&action| {
            self.keys(action)
                .iter()
                .map(move |keys| (action, keys.as_slice()))
        })
    }
}

/// Action with one key sequence bound to it
type Binding<'a> = (Action, &'a [Press]);

/// Reads the keys of a binding, written as the configuration file writes
/// them.  A binding cannot start with a digit, since digits typed first
/// count the command.
fn binding(text: &str) -> Result<Vec<Press>, String> {
    press::sequence(text).and_then(|keys| match keys.first().and_then(|key| key.digit()) {
        Some(_) => Err(format!("`{text}` starts with a digit, which counts")),
        None => Ok(keys),
    })
}

/// Describes how one of two bindings, in the order of [`Action::VARIANTS`],
/// hides the other, if one does: a sequence bound to both actions goes to
/// the earlier alone, and a sequence that begins a longer one, earlier or
/// later, keeps the longer from being typed
fn conflict((earlier, keys): Binding, (later, other): Binding) -> Option<String> {
    if keys == other {
        let (earlier, later) = (earlier.name(), later.name());
        return Some(format!(
            "`{}` is bound to both {earlier} and {later}, so {later} goes without",
            written(keys)
        ));
    }
    [(keys, other), (other, keys)]
        .into_iter()
        .find(|(short, long)| long.starts_with(short))
        .map(|(short, long)| {
            format!(
                "`{}` comes first, so `{}` can never be typed",
                written(short),
                written(long)
            )
        })
}
