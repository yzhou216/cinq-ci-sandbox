// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Command line, which names the show to open and can name the directory
//! that the key bindings come from

#[cfg(test)]
mod tests;

use std::path::PathBuf;

use clap::{Parser, ValueHint};

/// Version with the copyright and license notice that `--version` prints
/// after it, as the GNU Coding Standards set out
const LONG_VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    "\nCopyright (C) 2026 Yiyu Zhou",
    "\nLicense GPLv3+: GNU GPL version 3 or later <https://gnu.org/licenses/gpl.html>.",
    "\nThis is free software: you are free to change and redistribute it.",
    "\nThere is NO WARRANTY, to the extent permitted by law.",
);

/// Description that `--help` and the manual page give, where `-h` gives
/// the summary from the manifest of the crate
const LONG_ABOUT: &str = "\
cinq is a cue-based show controller in the spirit of QLab.  Its cue lists
hold audio, fade, stop, wait, and memo cues, and GO fires the standby cue
together with every cue that auto-continue or auto-follow chains to it.

cinq opens SHOW, a show file that cinq-show(5) describes, or a new show
without one.  Changes that are not saved yet are kept as they happen, and
come back the next time the show opens, as one change that undo takes back.

The keyboard runs most of cinq.  The status bar shows the key that opens
the help page, which lists every key, and config.toml can change the keys,
as cinq(5) describes.";

// Name, version, and summary come from the manifest of the crate
#[derive(Debug, Parser)]
#[command(version, long_version = LONG_VERSION, about, long_about = LONG_ABOUT)]
pub struct Cli {
    /// Open the show in SHOW, or start a new show that saving puts there
    #[arg(value_name = "SHOW", value_hint = ValueHint::FilePath)]
    pub show: Option<PathBuf>,
    /// Read config.toml from DIRECTORY instead of the configuration
    /// directory
    #[arg(long, value_name = "DIRECTORY", value_hint = ValueHint::DirPath)]
    pub config_dir: Option<PathBuf>,
}
