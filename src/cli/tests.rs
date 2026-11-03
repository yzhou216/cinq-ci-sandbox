// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the command line and its shell completions, of the desktop
//! entry that runs it, and of the share directory and the app bundle for
//! Darwin that the build lays out

use std::path::{Path, PathBuf};

use clap::{CommandFactory, Parser};
use clap_complete::{Generator, Shell};
use clap_complete_nushell::Nushell;

use super::Cli;

/// Desktop entry, through which application launchers list cinq
const DESKTOP_ENTRY: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/share/applications/cinq.desktop"
));

#[test]
fn command_line_is_well_formed() {
    Cli::command().debug_assert();
}

#[test]
fn completions_of_every_shell_offer_the_options() {
    let scripts = [
        ("bash", script(Shell::Bash)),
        ("zsh", script(Shell::Zsh)),
        ("fish", script(Shell::Fish)),
        ("Nushell", script(Nushell)),
    ];
    let lacking: Vec<&str> = scripts
        .iter()
        .filter(|(_, script)| !script.contains("config-dir"))
        .map(|&(shell, _)| shell)
        .collect();
    assert!(lacking.is_empty(), "{lacking:?} offer no --config-dir");
}

/// Returns the completions of the command line that `generator` writes
fn script(generator: impl Generator) -> String {
    let mut script = Vec::new();
    clap_complete::generate(generator, &mut Cli::command(), "cinq", &mut script);
    String::from_utf8(script).expect("completions are text")
}

#[test]
fn desktop_entry_names_describes_and_runs_cinq_as_its_manifest_does() {
    let value = |key: &str| {
        DESKTOP_ENTRY
            .lines()
            .find_map(|line| line.strip_prefix(key)?.strip_prefix('='))
    };
    assert_eq!(value("Name"), Some(env!("CARGO_PKG_NAME")));
    assert_eq!(value("Icon"), Some(env!("CARGO_PKG_NAME")));
    assert_eq!(value("Comment"), Some(env!("CARGO_PKG_DESCRIPTION")));
    assert_eq!(value("Exec"), Some(concat!(env!("CARGO_PKG_NAME"), " %f")));
}

#[test]
fn build_lays_out_the_share_directory_of_a_prefix() {
    // Build that finds no `.cargo/config.toml`, as one run from another
    // directory, lays out nothing
    let Some(share) = option_env!("CINQ_SHARE_DIR") else {
        return;
    };
    let missing: Vec<&str> = [
        "man/man1/cinq.1",
        "man/man5/cinq.5",
        "man/man5/cinq-show.5",
        "bash-completion/completions/cinq",
        "zsh/site-functions/_cinq",
        "fish/vendor_completions.d/cinq.fish",
        "nushell/vendor/autoload/cinq.nu",
        // Icon that the desktop entry names
        concat!(
            "icons/hicolor/scalable/apps/",
            env!("CARGO_PKG_NAME"),
            ".svg"
        ),
        "applications/cinq.desktop",
        "mime/packages/cinq.xml",
    ]
    .into_iter()
    .filter(|file| !Path::new(share).join(file).is_file())
    .collect();
    assert!(missing.is_empty(), "{share} lacks {missing:?}");
}

#[test]
fn build_for_darwin_lays_out_the_app_bundle_beside_the_share_directory() {
    // Builds for other systems lay out no bundle
    let Some(share) = option_env!("CINQ_SHARE_DIR").filter(|_| cfg!(target_os = "macos")) else {
        return;
    };
    let contents = Path::new(share).with_file_name("Applications/cinq.app/Contents");
    let missing: Vec<&str> = ["Info.plist", "MacOS", "Resources/cinq.icns"]
        .into_iter()
        .filter(|path| !contents.join(path).exists())
        .collect();
    assert!(
        missing.is_empty(),
        "{} lacks {missing:?}",
        contents.display()
    );
}

#[test]
fn config_dir_takes_its_directory_after_an_equals_sign_or_a_space() {
    let config_dir = |args: &[&str]| Cli::try_parse_from(args).map(|cli| cli.config_dir).ok();
    let directory = Some(Some(PathBuf::from("/config")));
    assert_eq!(config_dir(&["cinq"]), Some(None));
    assert_eq!(config_dir(&["cinq", "--config-dir=/config"]), directory);
    assert_eq!(config_dir(&["cinq", "--config-dir", "/config"]), directory);
    assert_eq!(config_dir(&["cinq", "--config-dir"]), None);
}

#[test]
fn version_tells_the_copyright_and_license() {
    let version = Cli::command().render_long_version();
    assert!(
        version.contains("Copyright (C) 2026 Yiyu Zhou"),
        "{version}"
    );
    assert!(version.contains("License GPLv3+"), "{version}");
}
