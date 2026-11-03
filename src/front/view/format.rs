// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Text for back-end values, kept apart from the widgets that show it

#[cfg(test)]
mod tests;

use std::path::Path;

use itertools::Itertools;

use crate::back::{Active, Advance, Change, Cue, CueList, Edit, Kind, Media, Phase, Workspace};

/// Formats seconds as `m:ss.s`, the way the cue list shows times
fn clock(seconds: f64) -> String {
    let tenths = (seconds.max(0.0) * 10.0).round() as u32;
    format!("{}:{:02}.{}", tenths / 600, tenths / 10 % 60, tenths % 10)
}

/// Formats a pre-wait or post-wait, leaving zero blank so that real waits
/// stand out
pub fn wait(seconds: f64) -> String {
    if seconds > 0.0 {
        clock(seconds)
    } else {
        String::new()
    }
}

/// Formats the length of a cue's action: the length of the clip for an
/// audio cue, marked when it loops, a question mark while that is unknown,
/// and a dash for instant cues
pub fn action(cue: &Cue) -> String {
    let length = clock(cue.action().into());
    match (cue.kind, &cue.media) {
        (Kind::Audio, Media::Loaded { .. }) if cue.loops() => format!("{length} loop"),
        (Kind::Audio, Media::Loaded { .. }) | (Kind::Fade | Kind::Wait, _) => length,
        (Kind::Audio, _) => "?".to_owned(),
        (Kind::Stop | Kind::Memo, _) => "-".to_owned(),
    }
}

/// Formats the post-wait, leaving it blank for auto-follow, which ignores it
pub fn post_wait(cue: &Cue) -> String {
    if cue.advance == Advance::Follow {
        String::new()
    } else {
        wait(cue.post_wait.value().into())
    }
}

/// Returns the short label of the Then column, blank for manual cues
pub fn advance(advance: Advance) -> &'static str {
    match advance {
        Advance::Manual => "",
        Advance::Continue => "continue",
        Advance::Follow => "follow",
    }
}

/// Describes what a cue acts on: the file name of an audio cue, or the
/// number of the cue that a fade or stop cue targets, or its quoted name
/// when it has no number
pub fn target(workspace: &Workspace, cue: &Cue) -> String {
    match cue.kind {
        Kind::Audio => cue
            .file_name()
            .and_then(|name| Path::new(name).file_name())
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned()),
        Kind::Fade | Kind::Stop => workspace.target(cue).map_or_else(String::new, number),
        Kind::Wait | Kind::Memo => String::new(),
    }
}

/// Formats a level in dB with its sign, without the unit, as the level
/// field holds it
pub fn decibels(db: f32) -> String {
    format!("{db:+.1}")
}

/// Counts things by name, as in `1 cue` and `2 cues`
pub fn count(number: usize, noun: &str) -> String {
    let plural = if number == 1 { "" } else { "s" };
    format!("{number} {noun}{plural}")
}

/// Describes where an active cue is in its lifetime: how soon its action
/// starts, after the chain that fired it and its own pre-wait, or how long
/// it has left
pub fn phase(active: &Active) -> String {
    match active.phase() {
        Phase::Scheduled { starts_in: wait } | Phase::PreWait { remaining: wait } => {
            format!("in {wait:.1}s")
        }
        Phase::Running { remaining } => format!("{} left", clock(remaining)),
        Phase::Looping => "looping".to_owned(),
    }
}

/// Describes an active cue in full for the list of running cues: whether
/// it is paused, where it is in its lifetime, whether a fade is changing
/// its level, and the level of an audio cue
pub fn activity(active: &Active, cue: &Cue) -> String {
    let paused = if active.paused { "paused, " } else { "" };
    let fading = active.fade.map_or("", |_| ", fading");
    let level = if cue.kind == Kind::Audio {
        format!(", {} dB", decibels(active.level))
    } else {
        String::new()
    };
    format!("{paused}{}{fading}{level}", phase(active))
}

/// Describes every cue of the chain that the next GO starts by its number,
/// with how long after GO each cue after the first fires, so that
/// auto-continue and auto-follow chains are visible before they run.  The
/// count leaves out the cues that cannot play, which GO skips, and each of
/// them says why.
pub fn chain(workspace: &Workspace, list: &CueList, standby: &Cue) -> String {
    let fired = count(workspace.next_go(list).count(), "cue");
    let described = list
        .chain(standby.id)
        .enumerate()
        .map(|(index, (cue, delay))| {
            let delay = (index > 0).then(|| format!("+{delay:.1}s"));
            let skipped = workspace
                .problem(cue)
                .map(|problem| format!("skipped: {problem}"));
            let notes = delay.into_iter().chain(skipped).join(", ");
            if notes.is_empty() {
                number(cue)
            } else {
                format!("{} ({notes})", number(cue))
            }
        });
    format!("GO fires {fired}: {}", described.format(", "))
}

/// Describes a change to the show in few words, starting in lower case, as
/// the status bar tells what undo or redo acts on
pub fn change(workspace: &Workspace, change: &Change) -> String {
    let named = |id| workspace.cue(id).map_or_else(String::new, number);
    match change {
        Change::Added { placed, .. } => format!("added cue {}", number(&placed.cue)),
        Change::Deleted { cues, .. } => match cues.as_slice() {
            [placed] => format!("deleted cue {}", number(&placed.cue)),
            cues => format!("deleted {} cues", cues.len()),
        },
        Change::Moved { cue, .. } => format!("moved cue {}", named(*cue)),
        Change::Edited { cue, redo, .. } => {
            let cue = named(*cue);
            match redo {
                Edit::Number(_) => format!("changed the number of cue {cue}"),
                Edit::Name(_) => format!("changed the name of cue {cue}"),
                Edit::File(_) => format!("changed the file of cue {cue}"),
                Edit::Sound { .. } => format!("chose the sound file of cue {cue}"),
                Edit::Start(_) => format!("changed the start of cue {cue}"),
                Edit::End(_) => format!("changed the end of cue {cue}"),
                Edit::Loop(_) => format!("changed whether cue {cue} loops"),
                Edit::Target(_) => format!("changed the target of cue {cue}"),
                Edit::Notes(_) => format!("changed the notes of cue {cue}"),
                Edit::PreWait(_) => format!("changed the pre-wait of cue {cue}"),
                Edit::Duration(_) => format!("changed the duration of cue {cue}"),
                Edit::PostWait(_) => format!("changed the post-wait of cue {cue}"),
                Edit::Advance(_) => format!("changed what follows cue {cue}"),
                Edit::Level(_) => format!("changed the level of cue {cue}"),
                Edit::StopTarget(_) => format!("changed whether cue {cue} stops its target"),
            }
        }
        Change::Standby(standby) => match standby.after.and_then(|id| workspace.cue(id)) {
            Some(cue) => format!("moved standby to cue {}", number(cue)),
            None => "moved standby past the end".to_owned(),
        },
        Change::Restored { .. } => "brought back the unsaved changes".to_owned(),
    }
}

/// Names a cue by its number, or by its name in quotes when it has no
/// number
fn number(cue: &Cue) -> String {
    if cue.number.trim().is_empty() {
        format!("\"{}\"", cue.name)
    } else {
        cue.number.clone()
    }
}
