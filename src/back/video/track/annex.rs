// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! NAL units of H.264, as MP4 holds them and as openh264 reads them
//!
//! A sample of MP4 holds its NAL units each after its length, while the
//! stream of Annex B of H.264, which openh264 reads, puts a start code
//! before each unit instead.

use std::iter;

/// Start code that goes before each NAL unit in Annex B
const START: [u8; 4] = [0, 0, 0, 1];

/// Returns `units` in Annex B, each after a start code
pub(super) fn annex_b<'a>(units: impl IntoIterator<Item = &'a [u8]>) -> Vec<u8> {
    units
        .into_iter()
        .flat_map(|unit| START.iter().chain(unit))
        .copied()
        .collect()
}

/// Returns the NAL units of a sample, each of which follows its length in
/// `length_size` bytes, most significant first.  A sample cut short ends at
/// the last whole unit, and units without bytes are left out.
pub(super) fn units(sample: &[u8], length_size: usize) -> impl Iterator<Item = &[u8]> {
    iter::successors(split(sample, length_size), move |(_, rest)| {
        split(rest, length_size)
    })
    .map(|(unit, _)| unit)
    .filter(|unit| !unit.is_empty())
}

/// Splits the first NAL unit off `bytes`, where it follows its length in
/// `length_size` bytes, from the units after it, or returns none when no
/// whole unit is left
fn split(bytes: &[u8], length_size: usize) -> Option<(&[u8], &[u8])> {
    let (length, rest) = bytes.split_at_checked(length_size)?;
    let length = length
        .iter()
        .fold(0_usize, |length, &byte| length << 8 | usize::from(byte));
    rest.split_at_checked(length)
}
