// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Reading keys typed in a row, which wait until they complete a command

use super::{Action, Keymap, Press};

/// What a key press amounts to, after the keys typed before it
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    /// Keys of a command typed halfway, which wait for more
    pub pending: Vec<Press>,
    /// Action that the keys complete, with the count typed before them
    pub action: Option<(Action, Option<usize>)>,
}

/// How a run of keys relates to the bindings
enum Found {
    Action(Action),
    /// Keys begin a binding but do not complete one
    Prefix,
    Nothing,
}

impl Keymap {
    /// Reads `press` after the keys typed before it.  Digits before a
    /// command count it.  Keys that begin no command are dropped, but a
    /// key that breaks off a command starts a new one, so that a stray key
    /// never swallows the next.
    pub fn parse(&self, pending: &[Press], press: Press) -> Outcome {
        if let Some(action) = self.at_once(press) {
            return Outcome {
                pending: Vec::new(),
                action: Some((action, None)),
            };
        }
        let keys: Vec<Press> = pending.iter().copied().chain([press]).collect();
        let (digits, command) = split_count(&keys);
        let count = digits
            .iter()
            .try_fold(0_usize, |count, key| {
                count.checked_mul(10)?.checked_add(key.digit()?)
            })
            .filter(|&count| count > 0);
        match self.find(command) {
            Found::Action(action) => Outcome {
                pending: Vec::new(),
                action: Some((action, count)),
            },
            Found::Prefix => Outcome {
                pending: keys,
                action: None,
            },
            Found::Nothing if !pending.is_empty() => self.parse(&[], press),
            Found::Nothing => Outcome {
                pending: Vec::new(),
                action: None,
            },
        }
    }

    /// Returns what can follow the keys typed so far, apart from a count
    /// before them: the rest of each binding that they begin, with its
    /// action, in the order of the actions on the help page.  A count alone
    /// waits for any command, so nothing in particular follows it.
    pub fn continuations<'a>(
        &'a self,
        pending: &'a [Press],
    ) -> impl Iterator<Item = (&'a [Press], Action)> + 'a {
        let (_, typed) = split_count(pending);
        self.bound()
            .filter(move |_| !typed.is_empty())
            .filter_map(move |(action, keys)| {
                keys.strip_prefix(typed)
                    .filter(|rest| !rest.is_empty())
                    .map(|rest| (rest, action))
            })
    }

    /// Returns whether a key typed into a text field acts anyway, as
    /// [`Action::acts_in_fields`] tells
    pub fn acts_in_fields(&self, press: Press) -> bool {
        self.alone(press).is_some_and(Action::acts_in_fields)
    }

    /// Returns whether a key acts at once as GO or Panic, as
    /// [`Action::acts_at_once`] tells
    pub fn acts_at_once(&self, press: Press) -> bool {
        self.alone(press).is_some_and(Action::acts_at_once)
    }

    /// Returns the action that a key carries out by itself, apart from any
    /// keys typed before it: one of [`Action::AT_ONCE`], whatever the
    /// modifiers held, or the action bound to the key alone
    pub fn alone(&self, press: Press) -> Option<Action> {
        self.at_once(press).or_else(|| match self.find(&[press]) {
            Found::Action(action) => Some(action),
            Found::Prefix | Found::Nothing => None,
        })
    }

    /// Returns the action of [`Action::AT_ONCE`] whose key is pressed, with
    /// or without modifiers
    fn at_once(&self, press: Press) -> Option<Action> {
        self.at_once_bound(press).map(|(action, _)| action)
    }

    /// Returns the action of [`Action::AT_ONCE`] whose key is pressed,
    /// together with the key as it is bound: as pressed, or else without
    /// the modifiers held, so that an action bound to the key with those
    /// very modifiers takes it over one bound to the key alone
    pub(super) fn at_once_bound(&self, press: Press) -> Option<(Action, Press)> {
        [press, press.bare()].into_iter().find_map(|key| {
            Action::AT_ONCE
                .into_iter()
                .find(|&action| self.keys(action).iter().any(|keys| keys[..] == [key]))
                .map(|action| (action, key))
        })
    }

    /// Finds the action bound to `keys`, or whether some binding begins
    /// with them
    fn find(&self, keys: &[Press]) -> Found {
        match self.bound().find(|(_, bound)| bound[..] == *keys) {
            Some((action, _)) => Found::Action(action),
            None if self.bound().any(|(_, bound)| bound.starts_with(keys)) => Found::Prefix,
            None => Found::Nothing,
        }
    }
}

/// Splits keys into the digits of a count and the command after them
fn split_count(keys: &[Press]) -> (&[Press], &[Press]) {
    keys.split_at(keys.iter().take_while(|key| key.digit().is_some()).count())
}
