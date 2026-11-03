// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Watching the configuration file, so that the keys follow it while the
//! program runs
//!
//! Editors often save a file by writing another and renaming it over the
//! first, which ends any watch on the file itself, so the watcher watches
//! the directory that holds the file instead.  While that directory is
//! missing, it watches the nearest one above that exists, so that making
//! the directories and then the file comes through as well.  Each burst of
//! changes reads the file once, after it settles.

#[cfg(test)]
mod tests;

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use iced::{
    futures::{SinkExt, Stream, StreamExt, channel::mpsc, future},
    stream,
};
use notify_debouncer_full::{
    DebounceEventResult, Debouncer, RecommendedCache, new_debouncer,
    notify::{self, RecommendedWatcher, RecursiveMode},
};

use super::{Config, load};

/// Time that changes must settle for before the file is read again, which
/// lets an editor finish saving
const SETTLE: Duration = Duration::from_millis(200);

/// Watcher of the directory, which stops watching once it is dropped
type Watcher = Debouncer<RecommendedWatcher, RecommendedCache>;

/// Returns the configuration that `file` holds once the watch is in place,
/// so that no change made before goes unseen, and again after each change
/// to the file.  When the file cannot be watched, the configuration comes
/// once more, with a problem that tells why the changes do not apply.
pub fn watch(file: PathBuf) -> impl Stream<Item = Config> {
    stream::channel(1, async move |mut output| {
        if let Err(error) = follow(&file, &mut output).await {
            let _ = output.send(unwatched(&file, &error)).await;
        }
    })
}

/// Sends the configuration that `file` holds to `output` once the watch is
/// in place and again after each change to the file, or returns why the
/// file cannot be watched
async fn follow(file: &Path, output: &mut mpsc::Sender<Config>) -> notify::Result<()> {
    let (sender, mut events) = mpsc::unbounded();
    let mut watcher = new_debouncer(SETTLE, None, move |result| {
        // Channel closes only once the stream has gone
        let _ = sender.unbounded_send(result);
    })?;
    let mut watched = None;
    loop {
        watched = rewatch(&mut watcher, watched.as_deref(), file)?;
        let _ = output.send(load(Some(file))).await;
        let touched = (&mut events)
            .any(|result| future::ready(touches(&result, file)))
            .await;
        if !touched {
            return Ok(());
        }
    }
}

/// Returns the configuration that `file` holds, with a problem that tells
/// why changes to it apply only after a restart
fn unwatched(file: &Path, error: &notify::Error) -> Config {
    let config = load(Some(file));
    let problem = format!("is not watched, so changes apply after a restart: {error}");
    Config {
        problems: [config.problems, vec![problem]].concat(),
        ..config
    }
}

/// Returns whether a burst of changes touches `file`, or a directory on the
/// way to it.  Opening a file, which reading it does, changes nothing.  A
/// watch that failed may have missed a change, so it counts as touching the
/// file.
fn touches(result: &DebounceEventResult, file: &Path) -> bool {
    match result {
        Ok(events) => events
            .iter()
            .filter(|event| !event.kind.is_access())
            .flat_map(|event| &event.paths)
            .any(|path| file.starts_with(path)),
        Err(_) => true,
    }
}

/// Watches the [`directory`] of `file` in place of the directory `watched`,
/// and returns the directory now watched, or why it cannot be watched.  The
/// watch goes in anew even when the directory is the same, since a
/// directory that was deleted and made again takes its old watch with it.
fn rewatch(
    watcher: &mut Watcher,
    watched: Option<&Path>,
    file: &Path,
) -> notify::Result<Option<PathBuf>> {
    // Directory that has gone took its watch with it, so a failure is moot
    if let Some(watched) = watched {
        let _ = watcher.unwatch(watched);
    }
    directory(file)
        .map(|directory| {
            watcher
                .watch(&directory, RecursiveMode::NonRecursive)
                .map(|()| directory)
        })
        .transpose()
}

/// Returns the directory to watch for `file`: the nearest one on the way to
/// it that exists, which is the one that holds it once that is there
fn directory(file: &Path) -> Option<PathBuf> {
    file.ancestors()
        .skip(1)
        .find(|directory| directory.is_dir())
        .map(Path::to_owned)
}
