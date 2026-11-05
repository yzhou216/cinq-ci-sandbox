// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Pictures of the clip of a video cue, which follow the clock of its run

use std::{
    path::PathBuf,
    sync::mpsc::{Receiver, TryRecvError},
};

use super::decoder::{self, Frame, Picture, Sent};
use crate::back::Clip;

/// Pictures of the clip of a video cue, which a thread of its own decodes
/// for each pass through the clip, a few pictures ahead of the clock of the
/// run that shows them, so that the first ones wait ready until the run
/// starts its clock.  The pass after the one under way of a clip that loops
/// decodes ahead as well, so that it has decoded from the sync sample
/// before the start of the clip by the time the pass under way ends.
/// Letting the player go ends the threads.
pub(super) struct Player {
    path: PathBuf,
    clip: Clip,
    /// Number of the pass under way, counted from none
    pass: u32,
    /// Pictures of the pass under way, each with the time of the run at
    /// which it is due, or why the picture cannot play
    pictures: Receiver<Sent>,
    /// Pictures of the pass after it, for a clip that loops
    following: Option<Receiver<Sent>>,
    /// Picture that arrived ahead of its time, which waits for the clock
    /// of the run to reach it
    next: Option<Frame>,
}

/// What the thread of a pass has sent since the player last looked
enum Arrival {
    Sent(Sent),
    Nothing,
    /// Thread has decoded the whole pass, and sent every picture of it
    Done,
}

impl Player {
    /// Starts decoding `clip` of the video file at `path`
    pub(super) fn start(path: PathBuf, clip: Clip) -> Self {
        Self {
            pictures: decoder::start(path.clone(), clip, 0),
            following: clip.looping.then(|| decoder::start(path.clone(), clip, 1)),
            pass: 0,
            path,
            clip,
            next: None,
        }
    }

    /// Returns the player once the clock of its run has reached `played`
    /// seconds of its clip, along with the newest picture that has come due
    /// since, if one has, or why the picture cannot play.  Pictures that
    /// came due along with a newer one are let go unseen, so that a picture
    /// that arrives late never holds back those after it.
    pub(super) fn advanced(self, played: f64) -> Result<(Self, Option<Picture>), String> {
        self.advanced_from(played, None)
    }

    /// Goes on with [`Player::advanced`], which has found `due` due so far
    fn advanced_from(
        self,
        played: f64,
        due: Option<Frame>,
    ) -> Result<(Self, Option<Picture>), String> {
        let arrival = match self.next {
            Some(frame) => Arrival::Sent(Ok(frame)),
            None => match self.pictures.try_recv() {
                Ok(sent) => Arrival::Sent(sent),
                Err(TryRecvError::Empty) => Arrival::Nothing,
                Err(TryRecvError::Disconnected) => Arrival::Done,
            },
        };
        let player = Self { next: None, ..self };
        match arrival {
            Arrival::Sent(Ok(frame)) if frame.at <= played => {
                player.advanced_from(played, Some(frame))
            }
            Arrival::Sent(Ok(frame)) => Ok((
                Self {
                    next: Some(frame),
                    ..player
                },
                due.map(|frame| frame.picture),
            )),
            Arrival::Sent(Err(reason)) => Err(reason),
            Arrival::Done if player.following.is_some() => {
                player.passed().advanced_from(played, due)
            }
            Arrival::Nothing | Arrival::Done => Ok((player, due.map(|frame| frame.picture))),
        }
    }

    /// Returns the player with the pass after the one under way under way,
    /// and the pass after that decoding ahead
    fn passed(self) -> Self {
        let pass = self.pass + 1;
        Self {
            pictures: self.following.unwrap_or(self.pictures),
            following: Some(decoder::start(self.path.clone(), self.clip, pass + 1)),
            pass,
            ..self
        }
    }
}
