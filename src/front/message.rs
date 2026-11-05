// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Messages that the update applies: the operator's intent, which the view
//! and the keys send, and the reports of the clock, of the configuration
//! file's watch, and of the work done on threads of its own

use std::{ffi::OsString, path::PathBuf, time::Instant};

use iced::window;
use strum::{Display, VariantArray};

use super::{
    keymap::{Press, config::Config},
    update::{Read, Saved},
};
use crate::back::{Command, CueId, Edit, Kind, Place};

/// Page of the inspector, which goes by its name
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Display, VariantArray)]
pub enum Tab {
    #[default]
    Basics,
    #[strum(to_string = "Time & Loops")]
    Time,
    Levels,
}

impl Tab {
    /// Returns the tab `offset` places to the right, wrapping around at
    /// either end
    pub fn stepped(self, offset: isize) -> Self {
        let count = Self::VARIANTS.len() as isize;
        Self::VARIANTS[(self as isize + offset).rem_euclid(count) as usize]
    }
}

/// Text field that keys move the keyboard focus to, since iced itself
/// moves it only on clicks
#[derive(Debug, Clone, Copy)]
pub enum Focus {
    /// Name field of the inspector, for editing the selected cue
    Name,
    /// Next field, or the first one when no field has focus
    Next,
    Previous,
    /// No field, which hands the keys back to the cue list
    Cues,
}

/// Operator intent or a report of work done, which becomes a back-end
/// command or an interface change
#[derive(Debug, Clone)]
pub enum Message {
    Go,
    Panic,
    PauseAll,
    ResumeAll,
    Stop(CueId),
    /// Time of a tick of the playback clock, or of a frame of the stage
    /// window while its picture moves
    Tick(Instant),
    Select(CueId),
    /// Moves the selection by a number of cues, downwards when positive
    Step(isize),
    /// Selects the cue on a row counted from one, or the last cue for a row
    /// past the end
    Row(usize),
    StandbySelected,
    SwitchList(usize),
    /// Shows the cue list a number of lists further down the sidebar
    StepList(isize),
    ShowMode(bool),
    Tab(Tab),
    /// Shows the inspector tab a number of tabs to the right
    StepTab(isize),
    NewKind(Kind),
    /// Adds a cue next to the selection, on the side that the function
    /// picks
    Add(fn(CueId) -> Place),
    /// Moves the selected cue by a number of places, downwards when
    /// positive
    Move(isize),
    /// Deletes a number of cues from the selection down
    Delete(usize),
    Edit(CueId, Edit),
    /// Text typed into the level field of a cue
    LevelTyped(CueId, String),
    /// Asks which file an audio or video cue plays
    ChooseFile(CueId),
    /// File chosen for an audio or video cue
    FileChosen(CueId, PathBuf),
    /// Takes back the latest change to the cues
    Undo,
    /// Makes again the change that undo took back last
    Redo,
    Focus(Focus),
    /// Shows the help page with every key binding, or hides it
    ToggleHelp,
    CloseHelp,
    /// Key that no widget kept, whether typed outside any text field or one
    /// that acts in fields, or a key of GO or Panic that a field took
    /// without typing it, and whether it is a repeat of a key held down
    Typed {
        press: Press,
        repeat: bool,
    },
    /// What loading the files of audio and video cues found, for the show
    /// of an identity
    Loaded(u64, Vec<Command>),
    /// Time to load again the files that could not be played
    Recheck,
    /// What loading again the files that could not be played found, for
    /// the show of an identity
    Rechecked(u64, Vec<Command>),
    /// Configuration as its file holds it, read once the watch on the file
    /// is in place and again after each change to it
    Reconfigured(Config),
    /// Opens a web page or a file in the program that the desktop opens it
    /// with
    OpenLink(OsString),
    /// Opens the configuration file in the program that the desktop opens
    /// it with, making it first when it is not there yet
    EditConfig(PathBuf),
    /// Why opening a link failed, if it did
    LinkOpened(Result<(), String>),
    /// Asks which show to open
    Open,
    /// Show read from its file, with the unsaved changes kept for it
    Opened(Box<Read>),
    /// Saves the show in its file, or asks where to save a show that has
    /// none
    Save,
    /// Asks where to save the show, and saves it there
    SaveAs,
    /// Saves the show in a file that the operator chose
    SaveTo(PathBuf),
    /// What saving the show found
    Saved(Saved),
    /// Mouse button pressed anywhere in a window
    Clicked,
    /// Window changed size
    Resized,
    /// Toggles the stage window between fullscreen and a window
    Fullscreen,
    /// Window closed, as the operator closes it
    Closed(window::Id),
}
