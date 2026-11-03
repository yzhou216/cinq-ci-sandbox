// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of GO, the clock, fades, stops, pauses, and panics

use super::*;

#[test]
fn go_fires_the_chain_and_moves_standby_past_it() {
    let workspace = workspace(vec![continuing_after(1, 2.0), audio(2), audio(3)]).apply(GO);
    assert_eq!(workspace.active().len(), 2);
    assert_eq!(
        workspace.active_of(CueId(2)).unwrap().phase(),
        Phase::Scheduled { starts_in: 2.0 }
    );
    assert_eq!(workspace.lists()[0].standby, Some(CueId(3)));
}

#[test]
fn fade_moves_the_level_in_a_straight_line_and_stops_its_target() {
    let workspace = workspace(vec![continuing(1), fade_out(2)])
        .apply(GO)
        .apply(Command::Tick(1.0));
    assert!((level(&workspace, 1).unwrap() - SILENCE / 4.0).abs() < 1e-4);
    assert!(workspace.apply(Command::Tick(3.0)).active().is_empty());
}

#[test]
fn stopping_a_fade_cue_leaves_its_target_at_the_level_reached() {
    let workspace = workspace(vec![continuing(1), fade_out(2)])
        .apply(GO)
        .apply(Command::Tick(1.0))
        .apply(Command::Stop(CueId(2)))
        .apply(Command::Tick(5.0));
    assert!((level(&workspace, 1).unwrap() - SILENCE / 4.0).abs() < 1e-4);
}

#[test]
fn stop_cue_stops_its_target() {
    let stop = Cue::new(CueId(2), "2", Kind::Stop, "Stop")
        .target(CueId(1))
        .build();
    let workspace = workspace(vec![audio(1), stop]).apply(GO).apply(GO);
    assert!(workspace.active().is_empty());
}

#[test]
fn panic_fades_out_and_a_second_panic_stops_at_once() {
    let panicked = workspace(vec![audio(1)]).apply(GO).apply(Command::Panic);
    assert!(panicked.clone().apply(Command::Panic).active().is_empty());
    let halfway = panicked.apply(Command::Tick(PANIC_FADE / 2.0));
    assert!((level(&halfway, 1).unwrap() - SILENCE / 2.0).abs() < 1e-4);
    assert!(halfway.apply(Command::Tick(PANIC_FADE)).active().is_empty());
}

#[test]
fn pause_holds_time_and_fades() {
    let workspace = workspace(vec![continuing(1), fade_out(2)])
        .apply(GO)
        .apply(Command::Pause(true))
        .apply(Command::Tick(2.0));
    assert_eq!(level(&workspace, 1), Some(0.0));
    assert_eq!(workspace.active_of(CueId(1)).unwrap().elapsed, 0.0);
}

#[test]
fn go_skips_broken_cues_in_the_chain() {
    let untargeted = Cue::new(CueId(2), "2", Kind::Fade, "Fade").build();
    let workspace = workspace(vec![continuing(1), untargeted]).apply(GO);
    assert_eq!(workspace.active().len(), 1);
    assert!(workspace.active_of(CueId(1)).is_some());
}

#[test]
fn stopping_a_cue_cancels_the_rest_of_its_chain_that_it_has_yet_to_start() {
    let workspace = workspace(vec![continuing_after(1, 2.0), audio(2)])
        .apply(GO)
        .apply(Command::Tick(1.0))
        .apply(Command::Stop(CueId(1)));
    assert!(workspace.active().is_empty());
}

#[test]
fn stopping_a_cue_leaves_the_rest_of_its_chain_that_has_started() {
    let workspace = workspace(vec![continuing(1), continuing_after(2, 2.0), audio(3)])
        .apply(GO)
        .apply(Command::Tick(1.0))
        .apply(Command::Stop(CueId(1)));
    let left: Vec<CueId> = workspace.active().iter().map(|active| active.id).collect();
    assert_eq!(left, [CueId(2), CueId(3)]);
}

#[test]
fn fade_cue_leaves_a_cue_that_a_panic_is_stopping() {
    let under = Cue::new(CueId(2), "2", Kind::Fade, "Under")
        .target(CueId(1))
        .level(-18.0)
        .stop_target(false)
        .build();
    let workspace = workspace(vec![audio(1), under])
        .apply(GO)
        .apply(Command::Panic)
        .apply(GO)
        .apply(Command::Tick(PANIC_FADE));
    assert!(workspace.active_of(CueId(1)).is_none());
}

#[test]
fn fade_cue_stopped_in_the_same_tick_does_not_fade() {
    let stop = Cue {
        advance: Advance::Continue,
        ..Cue::new(CueId(2), "2", Kind::Stop, "Stop the fade")
            .target(CueId(3))
            .build()
    };
    let workspace = workspace(vec![continuing(1), stop, fade_out(3)]).apply(GO);
    assert_eq!(workspace.active_of(CueId(1)).unwrap().fade, None);
}

#[test]
fn clock_keeps_time_through_an_hour_of_ticks() {
    let hold = Cue::new(CueId(1), "1", Kind::Wait, "Hold")
        .duration(7200.0)
        .build();
    let workspace = (0..144_000).fold(workspace(vec![hold]).apply(GO), |workspace, _| {
        workspace.apply(Command::Tick(0.025))
    });
    let elapsed = workspace.active_of(CueId(1)).unwrap().elapsed;
    assert!((elapsed - 3600.0).abs() < 1e-6, "{elapsed}");
}

#[test]
fn fade_leaves_a_target_that_has_yet_to_sound() {
    let waiting = Cue {
        pre_wait: 5.0.into(),
        ..audio(1)
    };
    let workspace = workspace(vec![waiting, fade_out(2)])
        .apply(GO)
        .apply(Command::Tick(1.0))
        .apply(GO)
        .apply(Command::Tick(4.0));
    assert_eq!(workspace.active_of(CueId(1)).unwrap().fade, None);
}

#[test]
fn fade_that_starts_late_catches_up() {
    let fade = Cue {
        pre_wait: 0.5.into(),
        ..Cue::new(CueId(2), "2", Kind::Fade, "Fade")
            .target(CueId(1))
            .duration(1.0)
            .build()
    };
    let workspace = workspace(vec![continuing(1), fade])
        .apply(GO)
        .apply(Command::Tick(2.0));
    assert!(workspace.active_of(CueId(1)).is_none());
}

#[test]
fn cue_faded_out_cancels_the_rest_of_its_chain_that_it_has_yet_to_start() {
    let workspace = workspace(vec![continuing_after(1, 6.0), audio(2), fade_out(3)])
        .apply(GO)
        .apply(GO)
        .apply(Command::Tick(4.0));
    assert!(workspace.active_of(CueId(2)).is_none());
}

#[test]
fn audio_cue_plays_from_its_start_to_its_end() {
    let trimmed = Cue {
        start: 2.0.into(),
        end: 5.0.into(),
        ..audio(1)
    };
    let workspace = workspace(vec![trimmed]).apply(GO).apply(Command::Tick(2.0));
    let active = workspace.active_of(CueId(1)).unwrap();
    assert_eq!(active.phase(), Phase::Running { remaining: 1.0 });
    assert!(workspace.apply(Command::Tick(1.0)).active().is_empty());
}

#[test]
fn audio_cue_that_starts_at_or_after_its_end_cannot_play() {
    let backwards = Cue {
        start: 6.0.into(),
        end: 3.0.into(),
        ..audio(1)
    };
    let workspace = workspace(vec![backwards]);
    let problems: Vec<Problem> = workspace.problems().map(|(_, problem)| problem).collect();
    assert_eq!(problems, [Problem::EmptyClip]);
    assert!(workspace.apply(GO).active().is_empty());
}

#[test]
fn audio_cue_that_loops_plays_until_something_stops_it() {
    let workspace = workspace(vec![Cue {
        looping: true,
        ..audio(1)
    }])
    .apply(GO)
    .apply(Command::Tick(25.0));
    let active = workspace.active_of(CueId(1)).expect("the loop plays on");
    assert_eq!(active.phase(), Phase::Looping);
    assert!((active.progress() - 0.5).abs() < 1e-6);
    assert!(workspace.apply(Command::Stop(CueId(1))).active().is_empty());
}

#[test]
fn fade_that_stops_its_target_ends_a_loop() {
    let looping = Cue {
        looping: true,
        ..continuing(1)
    };
    let workspace = workspace(vec![looping, fade_out(2)])
        .apply(GO)
        .apply(Command::Tick(4.0));
    assert!(workspace.active().is_empty());
}

#[test]
fn audio_cue_that_loops_leaves_the_cue_it_would_follow_to_go() {
    let following = Cue {
        looping: true,
        advance: Advance::Follow,
        ..audio(1)
    };
    let workspace = workspace(vec![following, audio(2)]).apply(GO);
    assert_eq!(workspace.active().len(), 1);
    assert_eq!(standby(&workspace), Some(2));
}
