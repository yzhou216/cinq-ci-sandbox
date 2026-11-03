// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Changes to vectors that move their items instead of cloning them

/// Returns the items with one more inserted at an index
pub(super) fn inserted<T>(mut items: Vec<T>, index: usize, item: T) -> Vec<T> {
    items.insert(index, item);
    items
}

/// Returns the items with the one at an index changed, or unchanged when
/// the index is past the end
pub(super) fn replaced<T>(items: Vec<T>, index: usize, change: impl FnOnce(T) -> T) -> Vec<T> {
    let mut change = Some(change);
    items
        .into_iter()
        .enumerate()
        .map(|(position, item)| {
            if let Some(change) = change.take_if(|_| position == index) {
                change(item)
            } else {
                item
            }
        })
        .collect()
}

/// Returns the items with the first one that `matches` accepts changed, or
/// unchanged when none does
pub(super) fn replaced_where<T>(
    items: Vec<T>,
    matches: impl Fn(&T) -> bool,
    change: impl FnOnce(T) -> T,
) -> Vec<T> {
    let mut change = Some(change);
    items
        .into_iter()
        .map(|item| match change.take_if(|_| matches(&item)) {
            Some(change) => change(item),
            None => item,
        })
        .collect()
}
