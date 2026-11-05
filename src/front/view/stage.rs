// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Stage window, which shows the pictures of the video cues that play
//!
//! Each picture fits the window whole, on the black background of the
//! theme, which shows around a picture whose shape differs from that of the
//! window, and on its own while no video plays.  The video cue that fired
//! last shows on top, and hides those that fired before it while its
//! picture is wholly opaque, so that a picture fading out as its cue stops
//! fades to whatever shows once it has gone.  The pointer stays hidden over
//! the stage, out of the audience's sight.

use iced::{
    ContentFit, Element, Fill, mouse,
    widget::{image, mouse_area, stack},
};

use crate::front::{App, Message};

pub fn view(app: &App) -> Element<'_, Message> {
    let layers: Vec<(&image::Handle, f32)> = app
        .workspace
        .active()
        .iter()
        .filter(|active| active.shows_picture())
        .filter_map(|active| Some((app.stage.picture(active.id)?, active.opacity())))
        .collect();
    let hidden = layers
        .iter()
        .rposition(|&(_, opacity)| opacity >= 1.0)
        .unwrap_or(0);
    let pictures = layers[hidden..].iter().map(|&(picture, opacity)| {
        image(picture.clone())
            .width(Fill)
            .height(Fill)
            .content_fit(ContentFit::Contain)
            .opacity(opacity)
            .into()
    });
    mouse_area(stack(pictures).width(Fill).height(Fill))
        .interaction(mouse::Interaction::Hidden)
        .into()
}
