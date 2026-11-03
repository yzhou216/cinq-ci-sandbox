// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the key bindings, with the helpers that read the keys they
//! type

mod file;
mod manual;
mod notation;
mod typing;

use strum::VariantArray;

use super::{
    Action, Keymap, Preset, Press, binding, config,
    press::{sequence, written},
    typing::Outcome,
};

/// Reads keys written as the configuration file writes them
fn keys(text: &str) -> Vec<Press> {
    sequence(text).expect("keys in the tests are valid")
}

/// Reads the first of the keys written
fn one(text: &str) -> Option<Press> {
    keys(text).first().copied()
}

/// Types keys in turn with the default bindings, and returns what the last
/// one amounts to
fn typed(text: &str) -> Outcome {
    let keymap = Keymap::default();
    keys(text).into_iter().fold(
        Outcome {
            pending: Vec::new(),
            action: None,
        },
        |outcome, press| keymap.parse(&outcome.pending, press),
    )
}

fn action(text: &str) -> Option<(Action, Option<usize>)> {
    typed(text).action
}

#[test]
fn shipped_keymaps_are_valid_and_hide_no_binding() {
    let invalid: Vec<&str> = Preset::VARIANTS
        .iter()
        .flat_map(|preset| Action::VARIANTS.iter().map(|&action| preset.keys(action)))
        .flatten()
        .copied()
        .filter(|keys| binding(keys).is_err())
        .collect();
    assert!(invalid.is_empty(), "{invalid:?}");
    let conflicts: Vec<(&str, Vec<String>)> = Preset::VARIANTS
        .iter()
        .map(|&preset| (preset.name(), Keymap::from(preset).conflicts()))
        .filter(|(_, conflicts)| !conflicts.is_empty())
        .collect();
    assert!(conflicts.is_empty(), "{conflicts:?}");
}

#[test]
fn keys_of_go_and_panic_hide_every_other_sequence_that_holds_them() {
    let keymap = Keymap::default()
        .with(Action::Panic, vec![keys("Escape"), keys("Shift+Escape")])
        .with(Action::NextList, vec![keys("Shift+Space")])
        .with(Action::First, vec![keys("g Space")])
        .with(Action::Delete, vec![keys("Space x")]);
    assert_eq!(
        keymap.conflicts(),
        [
            "`Space` comes first, so `Space x` can never be typed",
            "`Space` acts at once as go, so `Shift+Space` can never be typed",
            "`Space` acts at once as go, so `g Space` can never be typed",
        ]
    );
}
