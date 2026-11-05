// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! iced interface for a workspace
//!
//! The interface keeps only its own state, such as the selection and the
//! open inspector tab.  It reads the show from the back end and changes it
//! only by sending back-end commands.  Operator input arrives as a
//! [`Message`], [`update`] applies it, and [`view`] draws the result.  Keys
//! become messages through [`keymap`], whose bindings a configuration file
//! can change.  Besides the main window, a stage window shows the pictures
//! of video cues once the show holds one.

mod keymap;
mod message;
mod update;
mod view;

use std::{
    env, fs, io,
    path::{self, Path, PathBuf},
    time::Instant,
};

use iced::{Size, window};

use crate::back::{Audio, CueId, CueList, Kind, Video, Workspace};
use keymap::{Press, config::Config};
use message::{Focus, Message, Tab};
use update::{Keeper, Show, Stage};

/// Opens the main window with the show in `show`, or a new show without
/// one, and runs the interface until the main window closes.  The key
/// bindings come from `config.toml` in `config_dir`, when it is given, and
/// otherwise from the platform's configuration directory.
pub fn run(config_dir: Option<PathBuf>, show: Option<PathBuf>) -> iced::Result {
    if let Some(missing) = no_wayland() {
        return Err(missing);
    }
    let config = keymap::config::file(config_dir.as_deref());
    let show = show.map(|show| path::absolute(&show).unwrap_or(show));
    iced::daemon(
        move || Cinq::new(config.as_deref(), show.clone()),
        Cinq::update,
        Cinq::view,
    )
    .title(Cinq::title)
    .theme(view::theme::modus_vivendi())
    .settings(iced::Settings {
        default_font: view::FONT,
        default_text_size: view::TEXT_SIZE,
        ..iced::Settings::default()
    })
    .subscription(Cinq::subscription)
    .run()
}

/// Size that the main window opens at
const MAIN_SIZE: Size = Size::new(1440.0, 900.0);

/// Returns the settings of a window that opens `size` big, as the main
/// window and the stage window do
fn window_settings(size: Size) -> window::Settings {
    window::Settings {
        size,
        // Application ID for window manager rules, since some window
        // managers float a window whose ID iced has left empty
        #[cfg(target_os = "linux")]
        platform_specific: window::settings::PlatformSpecific {
            application_id: "cinq".to_owned(),
            ..window::settings::PlatformSpecific::default()
        },
        ..window::Settings::default()
    }
}

/// Returns why the window cannot open on GNU/Linux and the BSDs, where cinq
/// opens it on Wayland alone, when no variable names a Wayland compositor
/// to open it on, as winit tells before it tries, but only by panicking
fn no_wayland() -> Option<iced::Error> {
    let wayland_only = cfg!(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ));
    let named = ["WAYLAND_DISPLAY", "WAYLAND_SOCKET"]
        .into_iter()
        .any(|name| env::var_os(name).is_some_and(|value| !value.is_empty()));
    (wayland_only && !named).then(|| {
        iced::Error::WindowCreationFailed(
            "cinq runs on Wayland alone, and neither WAYLAND_DISPLAY nor WAYLAND_SOCKET is set"
                .into(),
        )
    })
}

/// Interface together with the sound and video output, which play what the
/// interface shows, and the keeper of the unsaved changes of the show.
/// These stay apart from the state of the interface, since they are the
/// parts with side effects, which only [`Cinq::update`] reaches.
struct Cinq {
    app: App,
    audio: Audio,
    video: Video,
    keeper: Keeper,
}

/// State of the interface, around the workspace that it shows
#[derive(Default)]
struct App {
    workspace: Workspace,
    /// Main window, once it has been asked for, whose closing quits cinq
    main: Option<window::Id>,
    /// Stage window, with the pictures of the video cues that play
    stage: Stage,
    /// File of the show, and what it holds
    show: Show,
    /// What went wrong with show files last, until opening or saving one
    /// succeeds
    file_problem: Option<String>,
    /// What the latest opening or saving told, which the status bar shows
    /// until the show changes
    told: Option<String>,
    /// Directory that keeps the unsaved changes of shows, if the platform
    /// has one
    unsaved_directory: Option<PathBuf>,
    /// Why the audio device could not be opened, if it could not
    audio_error: Option<String>,
    /// Index of the cue list shown in the main table
    list: usize,
    /// Cue shown in the inspector
    selected: Option<CueId>,
    /// Level of a cue as the operator types it, which its field shows in
    /// place of the level until the operator does anything else
    typed_level: Option<(CueId, String)>,
    tab: Tab,
    /// Kind of cue that Add cue creates
    new_kind: Kind,
    /// Time of the last clock tick, or `None` while the clock is stopped
    last_tick: Option<Instant>,
    /// Key bindings, and where they came from
    config: Config,
    /// Keys of a command typed so far, such as a count or the `g` of `g g`
    keys: Vec<Press>,
    /// Whether the help page with every key binding is open
    help: bool,
    /// Why the link that the help page opened last failed to open, which
    /// the page shows while it stays open
    link_problem: Option<String>,
    /// Whether the files of audio and video cues are being loaded
    loading: bool,
    /// Whether the files that could not be played are being read again
    rechecking: bool,
}

impl App {
    fn current(&self) -> &CueList {
        &self.workspace.lists()[self.list]
    }

    /// Returns the selection if it belongs to the list on screen
    fn selected_here(&self) -> Option<CueId> {
        self.selected.filter(|_| self.selected_row().is_some())
    }

    /// Returns the row of the selection in the list on screen, if it
    /// belongs there
    fn selected_row(&self) -> Option<usize> {
        self.selected.and_then(|id| self.current().position(id))
    }

    fn editable(&self) -> bool {
        !self.workspace.show_mode()
    }

    /// Returns whether any fired cue counts time, which a paused cue does
    /// not
    fn running(&self) -> bool {
        self.workspace.active().iter().any(|active| !active.paused)
    }

    /// Returns whether the picture of any video cue moves, as it does while
    /// the cue shows it and is not paused
    fn moving_pictures(&self) -> bool {
        self.workspace
            .active()
            .iter()
            .any(|active| active.shows_picture() && !active.paused)
    }
}

/// Returns the item `offset` places after `from`, or before it when
/// `offset` is negative, among `count` items.  Stepping stays put past
/// either end, and without an item to step from, it starts at the first.
fn step(from: Option<usize>, offset: isize, count: usize) -> Option<usize> {
    let last = count.checked_sub(1)?;
    Some(from.map_or(0, |index| index.saturating_add_signed(offset).min(last)))
}

/// Reads a file, or returns none when there is no such file
fn read(file: &Path) -> Result<Option<String>, String> {
    match fs::read_to_string(file) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

/// Returns where a view that shows `height` of its content from `scrolled`
/// on has to scroll to, to show the span from `top` to `bottom` while
/// scrolling as little as it can, or `None` when the span is in view
fn reveal(top: f32, bottom: f32, scrolled: f32, height: f32) -> Option<f32> {
    if top < scrolled {
        Some(top)
    } else if bottom > scrolled + height {
        Some(bottom - height)
    } else {
        None
    }
}
