// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Show model, with audio and video playback, with no knowledge of any
//! interface
//!
//! A front end reads a [`Workspace`] and changes it only by applying a
//! [`Command`], so every rule about what may change lives here.  Every
//! change consumes the workspace and returns a new one, without side
//! effects.  [`Audio`], which makes the sound that the workspace describes,
//! [`Video`], which makes its pictures, and [`load_files`], which reads the
//! files that cues name, are the parts with side effects.

mod audio;
mod cue;
mod items;
mod load;
mod video;
mod workspace;

use std::time::Duration;

use cue::Clip;

pub use audio::{Audio, SOUND_FILES};
pub use cue::{
    Advance, Cue, CueId, CueList, Edit, Kind, LEVELS, LOUDEST, Media, Problem, SILENCE, parse_level,
};
pub use load::{Load, load_files};
pub use video::{Picture, VIDEO_FILES, Video};
pub use workspace::{Active, Change, Command, Phase, Place, Turn, Workspace};

/// Interval of the playback clock while any cue is running, which the front
/// end keeps and the sound output glides by
pub const TICK: Duration = Duration::from_millis(25);
