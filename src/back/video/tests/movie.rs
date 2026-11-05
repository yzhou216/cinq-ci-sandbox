// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Video files that the tests make as they run, with openh264's encoder and
//! the MP4 writer of the mp4 crate, so that the repository holds none
//!
//! Each picture of a test video is gray, and lighter than the one before
//! it, so that a picture tells which one it is by its shade.

use std::{
    fs::File,
    io::BufWriter,
    path::{Path, PathBuf},
};

use mp4::{AvcConfig, Bytes, MediaConfig, Mp4Config, Mp4Sample, Mp4Writer, TrackConfig, TrackType};
use openh264::{
    OpenH264API,
    encoder::{Encoder, EncoderConfig, FrameType, IntraFramePeriod, RateControlMode},
    formats::YUVBuffer,
};
use tempfile::TempDir;

/// Width and height of a test video, in pixels
pub(in crate::back) const SIZE: (usize, usize) = (64, 48);

/// Pictures a second of a test video
pub(in crate::back) const RATE: u32 = 30;

/// Units of time a second in the track of a test video
const TIMESCALE: u32 = 600;

/// Shade of gray of the luma of the first picture, and how much lighter
/// each picture is than the one before it
const SHADES: (usize, usize) = (24, 8);

/// NAL unit types of the sequence and picture parameter sets of H.264
const PARAMETER_SETS: [u8; 2] = [7, 8];

/// Returns a temporary directory holding `movie.mp4`, a test video of
/// `count` pictures, at most 25, that decoding can start at every
/// `sync_every` pictures, with the path of the video
pub(in crate::back) fn movie(count: usize, sync_every: u32) -> (TempDir, PathBuf) {
    let directory = TempDir::new().expect("a temporary directory can be made");
    let path = directory.path().join("movie.mp4");
    write_movie(&path, count, sync_every);
    (directory, path)
}

/// Returns which picture of a test video `pixels` show, by the shade of
/// their first pixel, which RGBA holds in limited range
pub(in crate::back) fn picture_number(pixels: &[u8]) -> usize {
    let (first, step) = SHADES;
    let luma = f64::from(pixels[0]) * 219.0 / 255.0 + 16.0;
    ((luma - first as f64) / step as f64).round().max(0.0) as usize
}

/// Encodes a test video and writes it to `path`
fn write_movie(path: &Path, count: usize, sync_every: u32) {
    // Every picture, at a fixed quality, with sync samples where asked for
    let config = EncoderConfig::new()
        .skip_frames(false)
        .rate_control_mode(RateControlMode::Off)
        .scene_change_detect(false)
        .intra_frame_period(IntraFramePeriod::from_num_frames(sync_every));
    let mut encoder =
        Encoder::with_api_config(OpenH264API::from_source(), config).expect("the encoder starts");
    let pictures: Vec<(bool, Vec<Vec<u8>>)> = (0..count)
        .map(|number| {
            let encoded = encoder.encode(&gray(number)).expect("the picture encodes");
            let sync = matches!(encoded.frame_type(), FrameType::IDR | FrameType::I);
            let units = (0..encoded.num_layers())
                .filter_map(|layer| encoded.layer(layer))
                .flat_map(|layer| {
                    (0..layer.nal_count()).filter_map(move |unit| {
                        layer
                            .nal_unit(unit)
                            .map(|unit| without_start_code(unit).to_vec())
                    })
                })
                .collect();
            (sync, units)
        })
        .collect();
    let set = |kind: u8| {
        pictures
            .iter()
            .flat_map(|(_, units)| units)
            .find(|unit| unit[0] & 0x1f == kind)
            .cloned()
            .expect("the first picture holds the parameter sets")
    };
    let avc = AvcConfig {
        width: SIZE.0 as u16,
        height: SIZE.1 as u16,
        seq_param_set: set(PARAMETER_SETS[0]),
        pic_param_set: set(PARAMETER_SETS[1]),
    };
    let file = BufWriter::new(File::create(path).expect("the video file can be made"));
    let mut writer = Mp4Writer::write_start(file, &mp4_config()).expect("the video starts");
    writer
        .add_track(&TrackConfig {
            track_type: TrackType::Video,
            timescale: TIMESCALE,
            language: "und".to_owned(),
            media_conf: MediaConfig::AvcConfig(avc),
        })
        .expect("the track goes in");
    let duration = TIMESCALE / RATE;
    pictures
        .into_iter()
        .zip(0..)
        .try_for_each(|((sync, units), number)| {
            let sample = Mp4Sample {
                start_time: u64::from(number * duration),
                duration,
                rendering_offset: 0,
                is_sync: sync,
                bytes: Bytes::from(avcc(&units)),
            };
            writer.write_sample(1, &sample)
        })
        .expect("the pictures go in");
    writer.write_end().expect("the video ends");
}

/// Returns the settings of the MP4 file of a test video
fn mp4_config() -> Mp4Config {
    let brand = |name: &str| name.parse().expect("brands are four characters");
    Mp4Config {
        major_brand: brand("isom"),
        minor_version: 512,
        compatible_brands: ["isom", "iso2", "avc1", "mp41"].map(brand).to_vec(),
        timescale: 1000,
    }
}

/// Returns picture `number` of a test video, in I420: gray, and lighter
/// than the picture before it
fn gray(number: usize) -> YUVBuffer {
    let (width, height) = SIZE;
    let (first, step) = SHADES;
    let luma = (first + step * number).min(235) as u8;
    let pixels = width * height;
    let planes = [vec![luma; pixels], vec![128; pixels / 2]].concat();
    YUVBuffer::from_vec(planes, width, height)
}

/// Returns a NAL unit without the start code before it, as openh264's
/// encoder hands it out
fn without_start_code(unit: &[u8]) -> &[u8] {
    let start = unit
        .iter()
        .position(|&byte| byte == 1)
        .map_or(0, |one| one + 1);
    &unit[start..]
}

/// Returns the NAL units of a picture apart from the parameter sets, which
/// the description of the track holds, each after its length in four
/// bytes, as an MP4 sample holds them
fn avcc(units: &[Vec<u8>]) -> Vec<u8> {
    units
        .iter()
        .filter(|unit| !PARAMETER_SETS.contains(&(unit[0] & 0x1f)))
        .flat_map(|unit| {
            let length = u32::try_from(unit.len()).expect("units are small");
            length.to_be_bytes().into_iter().chain(unit.iter().copied())
        })
        .collect()
}
