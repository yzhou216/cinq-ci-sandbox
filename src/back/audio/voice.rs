// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Sound of one run of an audio or video cue

use std::{path::Path, time::Duration};

use kira::{
    AudioManager, Tween,
    sound::{
        FromFileError, Region,
        streaming::{StreamingSoundData, StreamingSoundHandle},
    },
};

use super::preload::Preloaded;
use crate::back::{Active, Clip, TICK};

/// Time a sound takes to reach a level the workspace asks for.  It lasts
/// two ticks of the playback clock, so fades move smoothly from one tick to
/// the next.
const GLIDE: Duration = TICK.saturating_mul(2);

/// Sound of one run of an audio or video cue
pub(super) struct Voice {
    pub(super) run: u32,
    sound: StreamingSoundHandle<FromFileError>,
    /// Level last sent to the sound
    level: f32,
    /// Pause state last sent to the sound
    paused: bool,
}

impl Voice {
    /// Opens the file of a run and starts its sound at the level of the
    /// run, as far into its clip as the run has played, since the clock may
    /// have passed the start by part of a tick.  The sound streams from the
    /// file, so long files start as quickly as short ones, but opening the
    /// file can wait on the disk, which [`Voice::start`] avoids.  Returns
    /// why the sound cannot start, as when its file has gone since it
    /// loaded or the output plays as many sounds as it can.
    pub(super) fn open(
        manager: &mut AudioManager,
        file: &Path,
        active: &Active,
        clip: Clip,
    ) -> Result<Self, String> {
        let data = StreamingSoundData::from_file(file).map_err(super::reason)?;
        let data = clipped(data, clip, clip.position(active.played()), active.level);
        let sound = manager.play(data).map_err(|error| error.to_string())?;
        Ok(Self::of(sound, active))
    }

    /// Starts a sound made ready ahead, which waits at the start of its
    /// clip, at the level that the run has now.  A run that the clock
    /// started between two ticks, as a pre-wait or a chain does, has played
    /// part of a tick already, and its sound starts late by that part:
    /// kira keeps what it has decoded ahead when a playing sound seeks, so
    /// seeking would play the start twice.  Returns why the sound cannot
    /// start, as when the output plays as many sounds as it can.
    pub(super) fn start(
        manager: &mut AudioManager,
        preloaded: Preloaded,
        active: &Active,
    ) -> Result<Self, String> {
        let mut sound = manager.play(preloaded).map_err(|error| error.to_string())?;
        let now = Tween {
            duration: Duration::ZERO,
            ..Tween::default()
        };
        sound.set_volume(active.level, now);
        Ok(Self::of(sound, active))
    }

    /// Returns the voice of `sound`, which plays at the level of `active`
    fn of(sound: StreamingSoundHandle<FromFileError>, active: &Active) -> Self {
        Self {
            run: active.run,
            sound,
            level: active.level,
            paused: false,
        }
    }

    /// Sends the sound any change in the level or pause state of its run
    pub(super) fn followed(mut self, active: &Active) -> Self {
        match (self.paused, active.paused) {
            (false, true) => self.sound.pause(Tween::default()),
            (true, false) => self.sound.resume(Tween::default()),
            _ => {}
        }
        if self.level != active.level {
            let glide = Tween {
                duration: GLIDE,
                ..Tween::default()
            };
            self.sound.set_volume(active.level, glide);
        }
        Self {
            level: active.level,
            paused: active.paused,
            ..self
        }
    }

    /// Stops the sound with kira's default ramp, which is short enough to
    /// sound immediate and long enough to avoid a click
    pub(super) fn stop(mut self) {
        self.sound.stop(Tween::default());
    }
}

/// Returns `data` set to play `clip`, from `position` in the file and at
/// `level`.  The sound ends at the end of the clip, or goes back to the
/// start of the clip there when the clip loops.
pub(super) fn clipped(
    data: StreamingSoundData<FromFileError>,
    clip: Clip,
    position: f64,
    level: f32,
) -> StreamingSoundData<FromFileError> {
    // Cut from the start of the file, so that times in the cut are times in
    // the file, which kira seeks to directly.  A cut is left out where it
    // would end past the last frame of the file.
    let cut = (clip.end < data.duration().as_secs_f64()).then(|| Region::from(0.0..clip.end));
    let looped = clip.looping.then(|| Region::from(clip.start..));
    data.slice(cut)
        .loop_region(looped)
        .start_position(position)
        .volume(level)
}
