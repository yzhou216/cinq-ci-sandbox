// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of how typed seconds read

use super::Seconds;

fn seconds(text: &str) -> Seconds {
    Seconds::from(text.to_owned())
}

#[test]
fn text_stays_as_typed() {
    assert_eq!(seconds("1.").text(), "1.");
    assert_eq!(seconds("1.").value(), 1.0);
}

#[test]
fn value_is_a_finite_number_of_seconds_or_zero() {
    assert_eq!(seconds(" 2.5 ").value(), 2.5);
    assert_eq!(seconds("-2").value(), 0.0);
    assert_eq!(seconds("1e40").value(), 0.0);
    assert_eq!(seconds("inf").value(), 0.0);
    assert_eq!(seconds("soon").value(), 0.0);
}

#[test]
fn times_read_as_the_cue_list_shows_them() {
    assert_eq!(seconds("1:30").value(), 90.0);
    assert_eq!(seconds("0:02.5").value(), 2.5);
    assert_eq!(seconds("1:00:05").value(), 3605.0);
}

#[test]
fn text_that_reads_as_no_time_is_invalid_unless_empty() {
    assert!(seconds("").is_valid());
    assert!(seconds(" ").is_valid());
    assert!(seconds("1:30").is_valid());
    assert!(!seconds("soon").is_valid());
    assert!(!seconds("1:").is_valid());
    assert!(!seconds("-2").is_valid());
    assert!(!seconds("1e40").is_valid());
}
