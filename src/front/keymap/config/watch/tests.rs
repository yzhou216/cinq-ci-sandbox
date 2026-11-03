// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of watching the configuration file, whose keys follow the file as
//! it comes, as an editor saves it, and as it goes

use std::{fs, io, iter, path::PathBuf, sync::mpsc, thread, time::Duration};

use futures::executor;
use iced::futures::{StreamExt, future};
use tempfile::TempDir;

use super::watch;
use crate::front::keymap::{
    Preset,
    config::{Config, Source},
};

/// Time that the watcher has to see a change before the test fails
const PATIENCE: Duration = Duration::from_secs(10);

/// Waits for a configuration from the watcher that `wanted` accepts
fn until(configs: &mpsc::Receiver<Config>, wanted: impl Fn(&Config) -> bool) {
    let found = iter::from_fn(|| configs.recv_timeout(PATIENCE).ok()).any(|config| wanted(&config));
    assert!(found, "the watcher sent no such configuration");
}

/// Accepts a configuration read from a file that starts from `preset`
fn from_file(preset: Preset) -> impl Fn(&Config) -> bool {
    move |config| matches!(config.source, Source::File(_)) && config.keymap.name() == preset.name()
}

/// Accepts the defaults of a configuration whose file is missing
fn defaults(config: &Config) -> bool {
    matches!(config.source, Source::Missing(_))
}

/// Watches `file` on a thread of its own, and returns what the watcher
/// sends
fn watched(file: PathBuf) -> mpsc::Receiver<Config> {
    let (sender, configs) = mpsc::channel();
    thread::spawn(move || {
        executor::block_on(watch(file).for_each(|config| {
            let _ = sender.send(config);
            future::ready(())
        }));
    });
    configs
}

#[test]
fn keys_follow_the_file_as_it_comes_changes_and_goes() -> io::Result<()> {
    let home = TempDir::new()?;
    let directory = home.path().join("cinq");
    let file = directory.join("config.toml");
    let configs = watched(file.clone());
    until(&configs, defaults);
    fs::create_dir(&directory)?;
    fs::write(&file, "keymap = \"cua\"\n")?;
    until(&configs, from_file(Preset::Cua));
    // Saved as editors often save, by renaming another file over it
    let draft = directory.join("config.toml.draft");
    fs::write(&draft, "keymap = \"vi\"\n")?;
    fs::rename(&draft, &file)?;
    until(&configs, from_file(Preset::Vi));
    fs::remove_file(&file)?;
    until(&configs, defaults);
    Ok(())
}

#[test]
fn reading_the_file_does_not_make_the_watcher_read_it_again() -> io::Result<()> {
    let home = TempDir::new()?;
    let file = home.path().join("config.toml");
    fs::write(&file, "keymap = \"cua\"\n")?;
    let configs = watched(file);
    until(&configs, from_file(Preset::Cua));
    // Reading opens the file, which the watcher sees as well, so a watcher
    // that took the opening for a change would read the file over and over.
    // FSEvents on Darwin may still report the write made just before the
    // watch began, which reads the file once more, and no more.
    let again = iter::from_fn(|| configs.recv_timeout(Duration::from_secs(2)).ok())
        .take(2)
        .count();
    assert!(again <= 1, "the watcher kept reading the file unasked");
    Ok(())
}

#[test]
fn keys_follow_a_file_whose_directories_come_later() -> io::Result<()> {
    let home = TempDir::new()?;
    let directory = home.path().join("config").join("cinq");
    let file = directory.join("config.toml");
    let configs = watched(file.clone());
    until(&configs, defaults);
    fs::create_dir_all(&directory)?;
    fs::write(&file, "keymap = \"cua\"\n")?;
    until(&configs, from_file(Preset::Cua));
    Ok(())
}
