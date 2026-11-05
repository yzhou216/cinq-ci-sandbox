// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! H.264 track of an MP4 file: its samples, the times at which they show,
//! and their bytes as the decoder reads them
//!
//! MP4 holds the samples of a track in the order they decode, which with
//! B-frames differs from the order they show in.  The tables of the track
//! give each sample the time at which it decodes, and the offset from there
//! to the time at which it shows.  The picture starts with the first sample
//! to show, so that it starts with the sound, as the edit lists that encoders
//! write for B-frames shift it.  Each sample holds NAL units after their
//! lengths, which [`annex`] turns into the stream of start codes that
//! openh264 reads, and the parameter sets that decoding needs are in the
//! description of the track rather than in its samples.

mod annex;
#[cfg(test)]
mod tests;

use std::{
    fs::File,
    io::{self, BufReader},
    iter,
    path::Path,
};

use mp4::{MediaType, Mp4Reader, TrackType};

use crate::back::Media;
use annex::{annex_b, units};

/// Why a file holds no picture to play
const NO_VIDEO: &str = "holds no H.264 video";

/// Profiles of H.264 that openh264 decodes: Baseline, Main, and High, in
/// eight bits for each color with the colors shared by four pixels
const PROFILES: [u8; 3] = [66, 77, 100];

/// H.264 track of a video file, ready to read
pub(super) struct Track {
    reader: Mp4Reader<BufReader<File>>,
    /// Identity of the track in its file
    id: u32,
    /// Samples in the order they decode
    pub(super) samples: Vec<Sample>,
    /// Bytes before each NAL unit of a sample that give its length
    length_size: usize,
    /// Sequence and picture parameter sets of the track, in Annex B
    parameters: Vec<u8>,
    /// Seconds that the picture lasts
    pub(super) duration: f64,
    /// Whether the file holds a sound track besides
    sound: bool,
}

/// Sample of a track
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Sample {
    /// Seconds into the picture at which the sample shows
    pub(super) shows: f64,
    /// Whether decoding can start at the sample
    pub(super) sync: bool,
}

/// Reads how long the picture of a video file plays and whether the file
/// holds a sound track besides, or why the picture cannot be played.  This
/// reads the tables of the tracks of the file without decoding a picture.
pub(in crate::back) fn probe(path: &Path) -> Media {
    match Track::open(path) {
        Ok(track) => Media::Loaded {
            duration: track.duration as f32,
            sound: track.sound,
        },
        Err(reason) => Media::Failed(reason),
    }
}

impl Track {
    /// Opens the video file at `path` and reads the tables of its first
    /// H.264 track, or tells why it cannot be played
    pub(super) fn open(path: &Path) -> Result<Self, String> {
        let file = File::open(path).map_err(|error| match error.kind() {
            io::ErrorKind::NotFound => "file not found".to_owned(),
            _ => error.to_string(),
        })?;
        let size = file.metadata().map_err(|error| error.to_string())?.len();
        let reader =
            Mp4Reader::read_header(BufReader::new(file), size).map_err(|_| NO_VIDEO.to_owned())?;
        let track = reader
            .tracks()
            .values()
            .filter(|track| track.media_type().is_ok_and(|kind| kind == MediaType::H264))
            .min_by_key(|track| track.track_id())
            .ok_or(NO_VIDEO)?;
        let avc = &track
            .trak
            .mdia
            .minf
            .stbl
            .stsd
            .avc1
            .as_ref()
            .ok_or(NO_VIDEO)?
            .avcc;
        if !PROFILES.contains(&avc.avc_profile_indication) {
            return Err("holds H.264 video in a profile that cannot be decoded".to_owned());
        }
        // Tables of the track, whose runs of equal values give each sample
        // the time it takes to decode and its offset from there to the time
        // it shows, and list the samples that decoding can start at
        let stbl = &track.trak.mdia.minf.stbl;
        let deltas = stbl
            .stts
            .entries
            .iter()
            .flat_map(|entry| iter::repeat_n(entry.sample_delta, entry.sample_count as usize))
            .take(track.sample_count() as usize);
        let offsets = stbl
            .ctts
            .iter()
            .flat_map(|ctts| &ctts.entries)
            .flat_map(|entry| iter::repeat_n(entry.sample_offset, entry.sample_count as usize));
        let syncs = stbl.stss.as_ref().map(|stss| stss.entries.as_slice());
        let (samples, duration) = timeline(deltas, offsets, syncs, track.timescale());
        if samples.is_empty() || duration <= 0.0 {
            return Err(NO_VIDEO.to_owned());
        }
        let parameters = annex_b(
            avc.sequence_parameter_sets
                .iter()
                .chain(&avc.picture_parameter_sets)
                .map(|set| set.bytes.as_slice()),
        );
        Ok(Self {
            id: track.track_id(),
            length_size: usize::from(avc.length_size_minus_one) + 1,
            parameters,
            sound: reader.tracks().values().any(|track| {
                track
                    .track_type()
                    .is_ok_and(|kind| kind == TrackType::Audio)
            }),
            samples,
            duration,
            reader,
        })
    }

    /// Reads the sample at `index` in the order of decoding, in Annex B,
    /// after the parameter sets of the track when it is the `first` that a
    /// decoder reads
    pub(super) fn packet(&mut self, index: usize, first: bool) -> Result<Vec<u8>, String> {
        let number = u32::try_from(index + 1).map_err(|error| error.to_string())?;
        let sample = self
            .reader
            .read_sample(self.id, number)
            .map_err(|error| error.to_string())?;
        let length_size = self.length_size;
        let units = sample
            .iter()
            .flat_map(|sample| units(&sample.bytes, length_size));
        let parameters = if first {
            self.parameters.as_slice()
        } else {
            &[]
        };
        Ok([parameters, &annex_b(units)].concat())
    }

    /// Returns the samples that decoding the picture from `start` to `end`
    /// seconds reads, by their indices in the order of decoding: from the
    /// sync sample that decoding has to start at, to the last sample that
    /// shows before `end`.  Along with them comes the time at which the
    /// picture that shows at `start` shows, which is the first of the part.
    pub(super) fn span(&self, start: f64, end: f64) -> Option<(usize, usize, f64)> {
        let by_time = |a: &(usize, &Sample), b: &(usize, &Sample)| a.1.shows.total_cmp(&b.1.shows);
        let indexed = || self.samples.iter().enumerate();
        let (first, shown) = indexed()
            .filter(|(_, sample)| sample.shows <= start)
            .max_by(by_time)
            .or_else(|| indexed().min_by(by_time))?;
        let from = self.samples[..=first]
            .iter()
            .rposition(|sample| sample.sync)
            .unwrap_or(0);
        let to = self
            .samples
            .iter()
            .rposition(|sample| sample.shows < end)
            .unwrap_or(first);
        Some((from, to.max(first), shown.shows))
    }
}

/// Returns the samples of a track in the order of decoding, each with the
/// time at which it shows, counted in seconds from the first picture to
/// show, along with the time at which the picture ends.  Each sample takes
/// the time of its `deltas` to decode and shows its `offsets` after it
/// decodes, which are none past the end of the offsets, in units of which
/// `timescale` make a second.  Decoding can start at the samples whose
/// numbers, counted from one, `syncs` lists, or at every sample without the
/// list.
fn timeline(
    deltas: impl Iterator<Item = u32>,
    offsets: impl Iterator<Item = i32>,
    syncs: Option<&[u32]>,
    timescale: u32,
) -> (Vec<Sample>, f64) {
    // Time at which each sample shows and then stops showing, in units of
    // the timescale
    let shown: Vec<(i64, i64)> = deltas
        .scan(0_i64, |decoded, delta| {
            let at = *decoded;
            *decoded += i64::from(delta);
            Some((at, i64::from(delta)))
        })
        .zip(offsets.chain(iter::repeat(0)))
        .map(|((decoded, delta), offset)| {
            let shows = decoded + i64::from(offset);
            (shows, shows + delta)
        })
        .collect();
    let first = shown.iter().map(|&(shows, _)| shows).min().unwrap_or(0);
    let end = shown.iter().map(|&(_, ends)| ends).max().unwrap_or(0);
    let seconds = |time: i64| (time - first) as f64 / f64::from(timescale.max(1));
    let sync = |number: u32| syncs.is_none_or(|syncs| syncs.binary_search(&number).is_ok());
    let samples = shown
        .iter()
        .zip(1..)
        .map(|(&(shows, _), number)| Sample {
            shows: seconds(shows),
            sync: sync(number),
        })
        .collect();
    (samples, seconds(end))
}
