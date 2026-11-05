// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the times at which the samples of a track show, B-frames among
//! them, and of the NAL units of samples as openh264 reads them

use super::{
    Sample, Track,
    annex::{annex_b, units},
    timeline,
};
use crate::back::video::tests::movie::movie;

#[test]
fn samples_show_at_their_offsets_from_the_first_picture() {
    // I, P, and two B-frames, which decode in that order and show as I, B,
    // B, and P, two pictures of 512 units after they start to decode
    let (samples, end) = timeline(
        [512; 4].into_iter(),
        [1024, 2048, 512, 512].into_iter(),
        Some(&[1]),
        15360,
    );
    let picture = 512.0 / 15360.0;
    let expected =
        [(0.0, true), (3.0, false), (1.0, false), (2.0, false)].map(|(shows, sync)| Sample {
            shows: shows * picture,
            sync,
        });
    assert_eq!(samples, expected);
    assert_eq!(end, 4.0 * picture);
}

#[test]
fn samples_without_offsets_or_a_list_of_sync_samples_show_as_they_decode() {
    let (samples, end) = timeline([20; 3].into_iter(), [].into_iter(), None, 600);
    let shows: Vec<f64> = samples.iter().map(|sample| sample.shows * 30.0).collect();
    assert_eq!(shows, [0.0, 1.0, 2.0]);
    assert!(samples.iter().all(|sample| sample.sync));
    assert_eq!(end, 0.1);
}

#[test]
fn units_of_a_sample_go_after_start_codes() {
    // Second unit of two bytes, then a third whose length runs past the end
    let sample = [0, 0, 0, 1, 0x65, 0, 0, 0, 2, 0x41, 0x9a, 0, 0, 0, 9, 1];
    let found: Vec<&[u8]> = units(&sample, 4).collect();
    assert_eq!(found, [&[0x65][..], &[0x41, 0x9a]]);
    assert_eq!(annex_b(found), [0, 0, 0, 1, 0x65, 0, 0, 0, 1, 0x41, 0x9a]);
    let short: Vec<&[u8]> = units(&[0, 1, 0x67], 2).collect();
    assert_eq!(short, [&[0x67][..]]);
}

#[test]
fn decoding_starts_at_the_sync_sample_before_the_picture_that_shows_first() {
    let (_directory, path) = movie(20, 8);
    let track = Track::open(&path).expect("the video opens");
    let syncs: Vec<usize> = track
        .samples
        .iter()
        .enumerate()
        .filter(|(_, sample)| sample.sync)
        .map(|(index, _)| index)
        .collect();
    assert_eq!(syncs, [0, 8, 16]);
    let (from, to, first) = track.span(0.35, 0.5).expect("the part has pictures");
    assert_eq!((from, to), (8, 14));
    assert!((first * 30.0 - 10.0).abs() < 1e-9, "{first}");
}
