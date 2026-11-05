// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Threads that decode the clip of one run of a video cue, a pass through
//! the clip each, a few pictures ahead of the clock of the run
//!
//! A pass starts a decoder at the sync sample before the start of the clip,
//! and decodes from there to the last sample that shows before its end.
//! openh264 hands out pictures in the order they show, so they take the
//! times at which the samples decoded show, in order, and the pictures
//! before the start of the clip are let go without turning them into RGBA.
//! The decoder flushes nothing until the pass has read every sample, since
//! flushing after each sample, as openh264 does by default, fails on the
//! second picture of streams with B-frames.

use std::{
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, SyncSender},
    thread,
};

use itertools::Itertools;
use openh264::{
    OpenH264API,
    decoder::{DecodedYUV, Decoder, DecoderConfig, Flush},
    formats::YUVSource,
};

use super::track::Track;
use crate::back::Clip;

/// Pictures that a thread decodes ahead of the clock of its run, enough to
/// ride out the pictures that take longest to decode, such as those that
/// start a group of pictures
const AHEAD: usize = 4;

/// Picture of a video, in RGBA, row by row from the top
pub struct Picture {
    pub width: u32,
    pub height: u32,
    /// Red, green, blue, and alpha of each pixel, a byte each
    pub pixels: Vec<u8>,
}

/// Picture along with the time of its run at which it is due: the seconds
/// of the clip played, counting every pass of a clip that loops
pub(super) struct Frame {
    pub(super) at: f64,
    pub(super) picture: Picture,
}

/// What a thread sends: a picture, or why the picture cannot play
pub(super) type Sent = Result<Frame, String>;

/// Why decoding stops before the end of the clip
enum Stop {
    /// Run has ended, and its player has let go of the pictures
    Gone,
    /// File cannot be read, for the given reason
    Failed(String),
}

/// Starts decoding pass `pass` through `clip` of the video file at `path`
/// on a thread of its own, and returns the way to its pictures.  The thread
/// ends once it has decoded the pass, or once the way to its pictures goes.
pub(super) fn start(path: PathBuf, clip: Clip, pass: u32) -> Receiver<Sent> {
    let (sender, pictures) = mpsc::sync_channel(AHEAD);
    thread::spawn(move || {
        if let Err(Stop::Failed(reason)) = decoded(&path, clip, pass, &sender) {
            // Player that would hear of it may have gone too
            let _ = sender.send(Err(reason));
        }
    });
    pictures
}

/// Decodes pass `pass` through `clip` of the file at `path`, and sends each
/// of its pictures to `sender` with the time of the run at which it is due
fn decoded(path: &Path, clip: Clip, pass: u32, sender: &SyncSender<Sent>) -> Result<(), Stop> {
    let mut track = Track::open(path).map_err(Stop::Failed)?;
    let Some((from, to, first)) = track.span(clip.start, clip.end) else {
        return Ok(());
    };
    // Times at which the pictures decoded show, in the order they show
    let shows: Vec<f64> = track.samples[from..=to]
        .iter()
        .map(|sample| sample.shows)
        .sorted_by(f64::total_cmp)
        .collect();
    let passed = f64::from(pass) * (clip.end - clip.start);
    let send = |picture: DecodedYUV<'_>, shows: Option<&f64>| match shows {
        Some(&shows) if (first..clip.end).contains(&shows) => {
            let at = passed + (shows - clip.start).max(0.0);
            let frame = Frame {
                at,
                picture: rgba(&picture),
            };
            sender.send(Ok(frame)).map_err(|_| Stop::Gone)
        }
        _ => Ok(()),
    };
    let config = DecoderConfig::new().flush_after_decode(Flush::NoFlush);
    let mut decoder = Decoder::with_api_config(OpenH264API::from_source(), config)
        .map_err(|error| Stop::Failed(error.to_string()))?;
    // Samples that fail to decode give no picture, and decoding goes on
    let decoded = (from..=to).try_fold(0, |count, index| {
        let packet = track.packet(index, index == from).map_err(Stop::Failed)?;
        match decoder.decode(&packet) {
            Ok(Some(picture)) => send(picture, shows.get(count)).map(|()| count + 1),
            _ => Ok(count),
        }
    })?;
    let rest = shows.get(decoded..).unwrap_or_default();
    decoder
        .flush_remaining()
        .unwrap_or_default()
        .into_iter()
        .zip(rest)
        .try_for_each(|(picture, shows)| send(picture, Some(shows)))
}

/// Returns a decoded picture in RGBA
fn rgba(picture: &DecodedYUV<'_>) -> Picture {
    let (width, height) = picture.dimensions();
    let mut pixels = vec![0; width * height * 4];
    picture.write_rgba8(&mut pixels);
    // H.264 limits pictures to far fewer pixels a side than u32 counts
    Picture {
        width: width as u32,
        height: height as u32,
        pixels,
    }
}
