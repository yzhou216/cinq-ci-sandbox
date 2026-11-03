// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the manual page of the configuration file, whose table lists
//! the keys that each keymap binds to every action in the order of the help
//! page, since a key bound to two actions goes to the one listed first

use std::iter;

use super::*;

/// Manual page of the configuration file, in scdoc
const MANUAL: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/man/cinq.5.scd"));

/// Tells whether a line of scdoc opens a row of a table: `[`, `|`, or `]`,
/// then the alignment of its first cell, then a space
fn opens_row(line: &str) -> bool {
    matches!(
        line.as_bytes(),
        [
            b'[' | b'|' | b']',
            b'[' | b'-' | b']' | b'<' | b'=' | b'>' | b' ',
            b' ',
            ..
        ]
    )
}

/// Returns the rows of the first table of the manual, each as its cells.
/// Each cell takes a line of its own, which opens a row or, with `:`, adds
/// to it, and which holds the alignment of the cell, a space, and its text.
fn table() -> Vec<Vec<String>> {
    MANUAL
        .lines()
        .skip_while(|line| !opens_row(line))
        .take_while(|line| opens_row(line) || line.starts_with(':'))
        .fold(Vec::new(), |mut rows: Vec<Vec<String>>, line| {
            // Bold, as in the header, is no part of the text
            let cell = line.get(2..).unwrap_or_default().trim().trim_matches('*');
            match rows.last_mut() {
                Some(row) if line.starts_with(':') => row.push(cell.to_owned()),
                _ => rows.push(vec![cell.to_owned()]),
            }
            rows
        })
}

/// Returns a row of the table of keymaps: its name, then its cells
fn row(name: &str, cells: impl IntoIterator<Item = String>) -> Vec<String> {
    iter::chain([name.to_owned()], cells).collect()
}

#[test]
fn manual_lists_the_keys_of_every_keymap_in_order() {
    let keymaps: Vec<Keymap> = Preset::VARIANTS.iter().copied().map(Keymap::from).collect();
    let header = row(
        "Action",
        keymaps.iter().map(|keymap| keymap.name().to_owned()),
    );
    let actions = Action::VARIANTS.iter().map(|&action| {
        row(
            action.name(),
            keymaps.iter().map(|keymap| keymap.listed(action)),
        )
    });
    // Fails on the first row of man/cinq.5.scd that differs
    itertools::assert_equal(table(), iter::chain([header], actions));
}
