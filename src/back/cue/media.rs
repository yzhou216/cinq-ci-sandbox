// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! What loading the file of a cue found, and why a cue cannot play

use strum::Display;

/// What loading the file of an audio or video cue found
#[derive(Debug, Clone, Default, PartialEq)]
pub enum Media {
    /// File has not been loaded since it was set
    #[default]
    Unloaded,
    /// File plays for the given number of seconds, and holds sound unless
    /// it is a video file with a picture alone
    Loaded { duration: f32, sound: bool },
    /// File cannot be played, for the given reason
    Failed(String),
}

/// Reason a cue cannot play, which displays as the warnings list shows it
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display)]
pub enum Problem<'a> {
    /// Audio or video cue has no file
    #[strum(to_string = "no file set")]
    NoFile,
    /// File of an audio or video cue has not been loaded yet
    #[strum(to_string = "file not loaded")]
    NotLoaded,
    /// Loading the file of an audio or video cue failed for the given
    /// reason
    #[strum(transparent)]
    LoadFailed(&'a str),
    /// Audio or video cue starts at or after its end, or past the end of
    /// its file, so it has nothing to play
    #[strum(to_string = "starts at or after its end")]
    EmptyClip,
    /// Fade or stop cue has no cue it can act on
    #[strum(to_string = "no target cue")]
    NoTarget,
}
