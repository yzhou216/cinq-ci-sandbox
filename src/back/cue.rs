// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Cue data model, with the cue lists that hold cues

mod clip;
mod edit;
mod kind;
mod list;
mod media;
mod seconds;
#[cfg(test)]
mod tests;

use std::{ffi::OsStr, fmt, ops::RangeInclusive, path::Path};

use bon::Builder;

pub use clip::Clip;
pub use edit::Edit;
pub use kind::{Advance, Kind};
pub use list::CueList;
pub use media::{Media, Problem};
pub use seconds::Seconds;

/// Level in dB that counts as silence, which is the bottom of every level
pub const SILENCE: f32 = -60.0;

/// Loudest level in dB that the operator can give a cue
pub const LOUDEST: f32 = 12.0;

/// Levels in dB that the operator can give a cue
pub const LEVELS: RangeInclusive<f32> = SILENCE..=LOUDEST;

/// Returns the level in dB that `text` reads as, with its unit or without
/// it, as in `-6` or `-6 dB`, if it lies between [`SILENCE`] and
/// [`LOUDEST`]
pub fn parse_level(text: &str) -> Option<f32> {
    text.trim()
        .trim_end_matches(['d', 'D', 'b', 'B'])
        .trim()
        .parse::<f32>()
        .ok()
        .filter(|level| LEVELS.contains(level))
}

/// Identity of a cue that stays stable when the operator renumbers it
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CueId(pub u32);

/// One cue of a cue list, which displays as its number and name
///
/// [`Cue::new`] takes the settings every cue needs and returns a builder
/// for the rest, which `build` finishes.  By default a cue has no target,
/// no waits, and a manual advance, and an audio cue plays its whole file
/// once.  Fade and wait cues last five seconds, and fade cues fade their
/// target out and stop it, which is what most fades do.
#[derive(Debug, Clone, PartialEq, Builder)]
#[builder(start_fn = new)]
pub struct Cue {
    #[builder(start_fn)]
    pub id: CueId,
    #[builder(start_fn, into)]
    pub number: String,
    #[builder(start_fn)]
    pub kind: Kind,
    #[builder(start_fn, into)]
    pub name: String,
    /// Sound file an audio cue plays
    #[builder(default, into)]
    pub file: String,
    /// What loading the file found
    #[builder(skip)]
    pub media: Media,
    /// Time in the sound file at which an audio cue starts
    #[builder(default, into)]
    pub start: Seconds,
    /// Time in the sound file at which an audio cue ends, which is the end
    /// of the file when the text reads as no time
    #[builder(default, into)]
    pub end: Seconds,
    /// Whether an audio cue plays from its start again each time it
    /// reaches its end, until something stops it
    #[builder(default)]
    pub looping: bool,
    /// Cue a fade or stop cue acts on
    pub target: Option<CueId>,
    #[builder(default, into)]
    pub pre_wait: Seconds,
    /// Length of the action of a fade or wait cue
    #[builder(default = if kind.has_duration() { 5.0 } else { 0.0 }, into)]
    pub duration: Seconds,
    #[builder(default, into)]
    pub post_wait: Seconds,
    #[builder(default)]
    pub advance: Advance,
    #[builder(default, into)]
    pub notes: String,
    /// Level in dB an audio cue plays at, or a fade cue fades its target to
    #[builder(default = if kind == Kind::Fade { SILENCE } else { 0.0 })]
    pub level: f32,
    /// Whether a fade cue stops its target once the fade is over
    #[builder(default = kind == Kind::Fade)]
    pub stop_target: bool,
}

impl fmt::Display for Cue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.number, self.name)
    }
}

impl Cue {
    /// Returns the name that a new cue of `kind` goes by
    fn new_name(kind: Kind) -> String {
        format!("New {kind} cue")
    }

    /// Returns the edit that makes the cue play `file`, which names the cue
    /// after the file as well while the cue goes by the name it was added
    /// with, or by the name of the file it played before, as QLab names
    /// audio cues
    pub fn playing(&self, file: String) -> Edit {
        let unnamed =
            self.name == Self::new_name(self.kind) || Some(self.name.as_str()) == stem(&self.file);
        match stem(&file) {
            Some(name) if unnamed => Edit::Sound {
                name: name.to_owned(),
                file,
            },
            _ => Edit::File(file),
        }
    }

    /// Returns the sound file that the cue names, without the spaces around
    /// it, which do not count, so that a pasted path with a trailing newline
    /// still loads; or none when the cue names no file
    pub fn file_name(&self) -> Option<&str> {
        Some(self.file.trim()).filter(|name| !name.is_empty())
    }

    /// Returns the part of its sound file that an audio cue plays: from its
    /// start time to its end time, or to the end of the file where that
    /// comes first.  There is none for other kinds of cue, before the file
    /// has loaded, or when the start is not before the end.
    pub(in crate::back) fn clip(&self) -> Option<Clip> {
        let Media::Loaded { duration } = self.media else {
            return None;
        };
        let start = self.start.value();
        let end = self.end.time().map_or(duration, |end| end.min(duration));
        (self.kind == Kind::Audio && start < end).then_some(Clip {
            start: start.into(),
            end: end.into(),
            looping: self.looping,
        })
    }

    /// Returns whether the cue is an audio cue that loops, and so plays
    /// until something stops it
    pub fn loops(&self) -> bool {
        self.kind == Kind::Audio && self.looping
    }

    /// Returns the length of the cue's action, which for an audio cue is
    /// one pass through its clip.  It is zero for instant cues and for
    /// audio cues without a clip to play.
    pub fn action(&self) -> f32 {
        match self.kind {
            Kind::Audio => self.clip().map_or(0.0, |clip| clip.length() as f32),
            kind if kind.has_duration() => self.duration.value(),
            _ => 0.0,
        }
    }

    /// Returns whether this cue can act on `other`.  Fade cues act on audio
    /// cues, stop cues on any other cue, and the remaining kinds on none.
    pub fn can_target(&self, other: &Cue) -> bool {
        other.id != self.id
            && match self.kind {
                Kind::Fade => other.kind == Kind::Audio,
                Kind::Stop => true,
                Kind::Audio | Kind::Wait | Kind::Memo => false,
            }
    }

    /// Returns the delay between this cue firing and it firing the next cue,
    /// or `None` when the next cue waits for GO, as it does after an audio
    /// cue that loops and follows, whose action never ends
    fn next_delay(&self) -> Option<f32> {
        match self.advance {
            Advance::Manual => None,
            Advance::Continue => Some(self.pre_wait.value() + self.post_wait.value()),
            Advance::Follow if self.loops() => None,
            Advance::Follow => Some(self.pre_wait.value() + self.action()),
        }
    }
}

/// Returns the name of a sound file without its directory and its ending,
/// or none when there is no such name
fn stem(file: &str) -> Option<&str> {
    Path::new(file.trim()).file_stem().and_then(OsStr::to_str)
}
