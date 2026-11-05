// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of reading how long video files play and of decoding their
//! pictures in time, from the sync sample before a clip and on later passes
//! through a clip that loops, along with those of [`player`] and [`runs`],
//! on videos that [`movie`] makes

pub(in crate::back) mod movie;
mod player;
mod runs;

use std::{fs, path::Path, time::Duration};

use super::{decoder, probe};
use crate::back::{Clip, Media};
use movie::{RATE, movie, picture_number};

/// Longest that a test waits for a thread to decode a picture
const PATIENCE: Duration = Duration::from_secs(10);

/// Returns the clip from `start` to `end` seconds into a file
fn clip(start: f64, end: f64, looping: bool) -> Clip {
    Clip {
        start,
        end,
        looping,
    }
}

/// Returns the numbers of the pictures that decoding pass `pass` through
/// `clip` of the video at `path` sends, each with the time it is due at in
/// pictures of the video since the start of the clip
fn decoded(path: &Path, clip: Clip, pass: u32) -> Vec<(usize, f64)> {
    decoder::start(path.to_owned(), clip, pass)
        .iter()
        .map(|sent| {
            let frame = sent.unwrap_or_else(|reason| panic!("the pictures decode: {reason}"));
            let at = frame.at * f64::from(RATE);
            (picture_number(&frame.picture.pixels), at)
        })
        .collect()
}

/// Returns whether a picture decoded is the one expected, due at the time
/// expected but for rounding
fn same((number, at): (usize, f64), (expected, due): (usize, f64)) -> bool {
    number == expected && (at - due).abs() < 1e-6
}

/// Asserts that the pictures decoded are those expected, each with the
/// time it is due at in pictures since the start of its clip
fn assert_pictures(decoded: &[(usize, f64)], expected: &[(usize, f64)]) {
    let matching =
        decoded.len() == expected.len() && decoded.iter().zip(expected).all(|(&a, &b)| same(a, b));
    assert!(matching, "{decoded:?} is not {expected:?}");
}

#[test]
fn probe_reads_how_long_the_picture_lasts_and_whether_the_file_has_sound() {
    let (_directory, path) = movie(15, 5);
    assert_eq!(
        probe(&path),
        Media::Loaded {
            duration: 0.5,
            sound: false
        }
    );
}

#[test]
fn file_that_is_missing_or_holds_no_h264_video_cannot_play() {
    let failed = |path: &Path| match probe(path) {
        Media::Failed(reason) => reason,
        media => panic!("{media:?} plays"),
    };
    assert_eq!(failed(Path::new("/nowhere/movie.mp4")), "file not found");
    let (directory, _) = movie(1, 1);
    let notes = directory.path().join("notes.mp4");
    fs::write(&notes, "not a video").expect("the file can be written");
    assert_eq!(failed(&notes), "holds no H.264 video");
}

#[test]
fn pictures_come_in_the_order_they_show_each_due_at_its_time() {
    let (_directory, path) = movie(15, 5);
    let expected: Vec<(usize, f64)> = (0..15).map(|number| (number, number as f64)).collect();
    assert_pictures(&decoded(&path, clip(0.0, 0.5, false), 0), &expected);
}

#[test]
fn clip_between_sync_samples_starts_with_the_picture_showing_at_its_start() {
    let (_directory, path) = movie(20, 8);
    // Picture 10 shows from a third of a second on, a little before the
    // start, and decodes only after pictures 8 and 9, which it refers to
    let start = 0.35 * f64::from(RATE);
    let expected: Vec<(usize, f64)> = (10..15)
        .map(|number| (number, (number as f64 - start).max(0.0)))
        .collect();
    assert_pictures(&decoded(&path, clip(0.35, 0.5, false), 0), &expected);
}

#[test]
fn later_pass_through_a_clip_that_loops_is_due_a_pass_later() {
    let (_directory, path) = movie(12, 4);
    // Pictures 3 to 5 make a tenth of a second, three pictures long
    let pass = |passed: f64| [3, 4, 5].map(|number| (number, passed + (number - 3) as f64));
    assert_pictures(&decoded(&path, clip(0.1, 0.2, true), 2), &pass(6.0));
}
