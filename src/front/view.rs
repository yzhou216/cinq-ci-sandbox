// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! View functions, which keep iced's default look, apart from the
//! exceptions that [`theme`] lists, and only space the controls apart
//!
//! Each part of the window has a module of its own, with a `view` function
//! that draws it.  The text of back-end values comes from
//! [`format`](mod@format), the colors from [`theme`], and the widgets that
//! iced lacks from [`widget`].

mod cue_table;
mod format;
mod help;
mod inspector;
mod next_keys;
mod sidebar;
mod status_bar;
pub(super) mod theme;
mod toolbar;
mod transport;
mod widget;

use std::sync::LazyLock;

use iced::{
    Element, Font, Pixels, Size,
    advanced::text::{self as core_text, Paragraph as _},
    alignment,
    widget::{
        column, container, responsive, row, rule, stack,
        text::{LineHeight, Shaping, Wrapping},
    },
};
use itertools::Itertools;

use super::{
    App, Message,
    keymap::{Action, Keymap},
};
use widget::{button, dropdown, keep_keys, marked_row};

/// Identity of the cue list, through which the selection scrolls into view
pub const CUES: &str = "cues";

/// Identity of the selected row of the cue list, through which the list
/// finds where the selected cue is, to scroll it into view
pub const SELECTED: &str = "selected";

/// Font of every text, a monospace one, so that columns of text line up
pub const FONT: Font = Font::MONOSPACE;

/// Size of every text that sets no size of its own
pub const TEXT_SIZE: Pixels = Pixels(16.0);

/// Identity of the help page, which the keys that move the selection
/// scroll while it is open
pub const HELP: &str = "help";

/// Returns the height of a line of text, which sets the height of the
/// status bar and how far the help page scrolls for one key
pub fn line_height() -> f32 {
    f32::from(LineHeight::default().to_absolute(TEXT_SIZE))
}

/// Identity of the Name field, through which the key of the edit action
/// gives it the keyboard focus
pub const NAME: &str = "name";

/// Gap between neighbouring controls and around each part of the window,
/// so that every control stands apart from the next
const SPACING: f32 = 10.0;

/// Widest that the sidebar grows, which it is in a wide window
const SIDEBAR: f32 = 300.0;

/// Paragraph that the renderer lays text out in
type Paragraph = <iced::Renderer as core_text::Renderer>::Paragraph;

/// Line between the parts of the status bar, and between its hints, which
/// also parts a message from the hint of the key that answers it
pub const LINE: &str = " | ";

/// Returns `told` followed by the hint of the key of `action`, which
/// answers it, when `keymap` binds the action to a key
pub fn answered(told: String, keymap: &Keymap, action: Action) -> String {
    match keymap.hint(action, action.name()) {
        Some(hint) => format!("{told}{LINE}{hint}"),
        None => told,
    }
}

/// Draws the window, its parts stacked with rules between them, the keys
/// that can come next over them just above the status bar, and the help
/// page over everything while it is open.  The layers that come and go lie
/// over those that stay, since iced leaves a missing layer out of its stack
/// and matches the state of each widget to the widget in the same place,
/// so that the cue list stays scrolled where it was, and the fields keep
/// what they hold, as the layers come and go.
pub fn view(app: &App) -> Element<'_, Message> {
    let parts = column![
        transport::view(app),
        rule::horizontal(1),
        toolbar::view(app),
        rule::horizontal(1),
        responsive(move |size| {
            // Sidebar takes at most a quarter of the width, so that a narrow
            // window, such as one tiled beside another, leaves the cue list
            // room, which keeps to its own part of the window
            let sidebar = (size.width / 4.0).min(SIDEBAR);
            row![
                container(cue_table::view(app)).clip(true),
                rule::vertical(1),
                sidebar::view(app, sidebar),
            ]
            .into()
        }),
        rule::horizontal(1),
        inspector::view(app),
    ];
    let window = column![
        stack![parts, next_keys::view(app)],
        rule::horizontal(1),
        status_bar::view(app),
    ];
    stack![window, app.help.then(|| help::view(app))].into()
}

/// Returns what `shown` makes of the items of `yielding` that have given
/// way, as they give way in turn, the first to go first: the first result
/// that `fits` accepts, or else the one in which all of them have gone
fn giving_way<Y, T>(yielding: &[Y], shown: impl Fn(&[Y]) -> T, fits: impl Fn(&T) -> bool) -> T {
    (0..=yielding.len())
        .map(|gone| shown(&yielding[..gone]))
        .find_or_last(fits)
        .expect("there is always the way in which none give way")
}

/// Returns the width of one column of the font, which is monospace, so
/// that a line of text is as wide as the columns that it takes, two for a
/// wide character.  It is measured once, as the font stays the same while
/// the program runs.
fn char_width() -> f32 {
    static WIDTH: LazyLock<f32> = LazyLock::new(|| {
        Paragraph::with_text(core_text::Text {
            content: "0",
            bounds: Size::INFINITE,
            size: TEXT_SIZE,
            line_height: LineHeight::default(),
            font: FONT,
            align_x: core_text::Alignment::Default,
            align_y: alignment::Vertical::Top,
            shaping: Shaping::Basic,
            wrapping: Wrapping::None,
        })
        .min_width()
    });
    *WIDTH
}

/// Counts the cues that cannot play, a missing audio device, a problem
/// with the show file, and the problems with the configuration file
fn warnings(app: &App) -> usize {
    app.workspace.problems().count()
        + usize::from(app.audio_error.is_some())
        + usize::from(app.file_problem.is_some())
        + app.config.problems.len()
}
