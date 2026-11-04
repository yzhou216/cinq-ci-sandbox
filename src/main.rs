// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Cue-based show controller in the spirit of QLab
//!
//! Audio cues play sound files, fade cues change the level of a playing
//! audio cue, stop cues end another cue, and wait and memo cues pace and
//! annotate the show.  GO fires the standby cue together with every cue that
//! auto-continue or auto-follow chains to it.  Selection and standby are
//! independent: inspecting a cue never changes what the next GO fires.
//!
//! The back end in [`back`] holds the show, every rule for changing it, and
//! the audio output, and it does not depend on iced.  The front end in
//! [`front`] draws the show with iced and turns operator input into
//! back-end commands.  The command line in [`cli`] names the show to open,
//! and can name the directory that the key bindings come from.

// Program with a window of its own, which Windows then starts without a
// console window beside it
#![cfg_attr(windows, windows_subsystem = "windows")]

mod back;
mod cli;
mod front;
// App bundle, logo, and manual pages, which only the build script lays out,
// draws, and renders, compiled here for their tests
#[cfg(test)]
mod bundle;
#[cfg(test)]
mod logo;
#[cfg(test)]
mod man;

use std::process::ExitCode;

use clap::Parser;

fn main() -> ExitCode {
    let cli = cli::Cli::parse();
    match front::run(cli.config_dir, cli.show) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            // Reason for the failure, which iced leaves out of its own message
            let reason = match &error {
                iced::Error::ExecutorCreationFailed(reason) => reason.to_string(),
                iced::Error::WindowCreationFailed(reason) => reason.to_string(),
                iced::Error::GraphicsCreationFailed(reason) => reason.to_string(),
            };
            eprintln!("cinq: {error}: {reason}");
            ExitCode::FAILURE
        }
    }
}
