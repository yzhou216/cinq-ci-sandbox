// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the text that describes cues

use super::target;
use crate::back::{Cue, CueId, CueList, Kind, Workspace};

#[test]
fn target_goes_by_its_number_or_else_by_its_quoted_name() {
    let fade = |target: u32| {
        Cue::new(CueId(10 + target), "10", Kind::Fade, "Fade")
            .target(CueId(target))
            .build()
    };
    let numbered = Cue::new(CueId(1), "1", Kind::Audio, "Rain").build();
    let unnumbered = Cue::new(CueId(2), "", Kind::Audio, "Thunder").build();
    let workspace = Workspace::new(vec![CueList::new("Main", vec![numbered, unnumbered])]);
    assert_eq!(target(&workspace, &fade(1)), "1");
    assert_eq!(target(&workspace, &fade(2)), "\"Thunder\"");
    assert_eq!(target(&workspace, &fade(3)), "");
}
