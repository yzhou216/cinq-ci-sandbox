// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Opening the links of the help page, the configuration file among them,
//! in the programs that the desktop opens them with

use std::{ffi::OsStr, path::Path};

use iced::Task;

use super::on_own_thread;
use crate::front::{App, Message, keymap};

impl App {
    /// Returns the state without the last failure to open a link, along
    /// with the task that runs `open` on a thread of its own, since an
    /// opener can take a moment to tell that it failed
    pub(super) fn opening(
        self,
        open: impl FnOnce() -> Result<(), String> + Send + 'static,
    ) -> (Self, Task<Message>) {
        let task = on_own_thread(open).map(Message::LinkOpened);
        let app = Self {
            link_problem: None,
            ..self
        };
        (app, task)
    }
}

/// Opens a web page or a file in the program that the desktop opens it
/// with: through xdg-open on desktops that follow the XDG specifications,
/// and through the system's own opener elsewhere.  Returns why it failed,
/// if it did.
pub(super) fn open_link(target: &OsStr) -> Result<(), String> {
    open::that_detached(target)
        .map_err(|error| format!("Cannot open {}: {error}", target.to_string_lossy()))
}

/// Opens the configuration file `file` as [`open_link`] does, after making
/// it when it is not there yet, except on Darwin, which opens it in the
/// default text editor, since few programs there say they open TOML.
/// Returns why it failed, if it did.
pub(super) fn edit_config(file: &Path) -> Result<(), String> {
    keymap::config::make(file)
        .map_err(|error| format!("Cannot make {}: {error}", file.display()))?;
    if cfg!(target_os = "macos") {
        open_in_text_editor(file)
    } else {
        open_link(file.as_os_str())
    }
}

/// Opens `file` in the default text editor through `open -t`, and waits
/// for `open`, which tells whether there is one.  Returns why it failed, if
/// it did.
fn open_in_text_editor(file: &Path) -> Result<(), String> {
    let cannot = |why: String| format!("Cannot open {}: {why}", file.display());
    let status = std::process::Command::new("/usr/bin/open")
        .args(["-t", "--"])
        .arg(file)
        .status()
        .map_err(|error| cannot(error.to_string()))?;
    if status.success() {
        Ok(())
    } else {
        Err(cannot(format!("open -t failed with {status}")))
    }
}
