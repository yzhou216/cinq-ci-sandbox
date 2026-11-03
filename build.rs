// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Lays out what cinq installs besides its program in the directory that
//! `CINQ_SHARE_DIR` names, as the share directory of a prefix holds it, so
//! that a package copies it whole into place.  The manual pages for the
//! platform of the build go in the directory of their section under `man`:
//! cinq(1) from the command line, and cinq(5) and cinq-show(5) from their
//! scdoc sources by scdoc, which has to be installed then.  The pages are
//! dated by `SOURCE_DATE_EPOCH`, which reproducible builds set, or else by
//! the day of the build.  The completions of the command line for bash,
//! zsh, fish, and Nushell go where each shell looks for them, and the logo,
//! drawn as the icon of the desktop entry, goes in the hicolor icon theme,
//! next to the desktop entry and the type of show files from `share`.
//! Builds for Darwin also lay out the app bundle, all but the program, in
//! the Applications directory beside it.  A relative directory goes from
//! the root of the package.

// Modules of the program that the build compiles as well, which keep their
// own modules where the program finds them
#[path = "src"]
mod src {
    pub mod bundle;
    // Command line, of which the build reads only the definition
    #[expect(dead_code)]
    pub mod cli;
    pub mod logo;
    pub mod man;
}

use std::{
    env,
    fs::{self, File},
    io,
    path::Path,
    process::Command,
};

use clap::CommandFactory;
use clap_complete::{Generator, Shell, generate};
use clap_complete_nushell::Nushell;
use time::OffsetDateTime;

use src::{
    bundle,
    cli::Cli,
    logo,
    man::{self, Platform},
};

fn main() {
    // Files that the build reads as it runs, unlike the modules and pages
    // that it compiles in, a change to which rebuilds and so reruns it
    SHARED
        .iter()
        .for_each(|file| println!("cargo::rerun-if-changed={file}"));
    ["CINQ_SHARE_DIR", "SOURCE_DATE_EPOCH"]
        .iter()
        .for_each(|variable| println!("cargo::rerun-if-env-changed={variable}"));
    let Some(share) = env::var_os("CINQ_SHARE_DIR") else {
        return;
    };
    let share = Path::new(&share);
    let darwin = env::var("CARGO_CFG_TARGET_OS").is_ok_and(|target| target == "macos");
    [
        ("render the manual pages", render(&share.join("man"))),
        ("write the shell completions", complete(share)),
        (
            "draw the icon",
            draw(&share.join("icons/hicolor/scalable/apps")),
        ),
        (
            "copy the desktop entry and the type of show files",
            copy(share),
        ),
    ]
    .into_iter()
    .chain(darwin.then(|| ("lay out the app bundle", assemble(share))))
    .filter_map(|(what, done)| done.err().map(|error| (what, error)))
    .for_each(|(what, error)| println!("cargo::error=cannot {what}: {error}"));
}

/// Files of the package that the share directory of a prefix holds as they
/// are, each by its path under `share`
const SHARED: [&str; 2] = [
    "share/applications/cinq.desktop",
    "share/mime/packages/cinq.xml",
];

/// Writes `contents` to `file`, along with the directories that it goes in
fn write(file: &Path, contents: impl AsRef<[u8]>) -> io::Result<()> {
    fs::create_dir_all(file.parent().expect("a file goes in a directory"))?;
    fs::write(file, contents)
}

/// Copies the [`SHARED`] files to their places under `share`
fn copy(share: &Path) -> io::Result<()> {
    SHARED.iter().try_for_each(|file| {
        let place = Path::new(file)
            .strip_prefix("share")
            .expect("shared files are in share");
        write(&share.join(place), fs::read(file)?)
    })
}

/// Writes the completions of the command line for bash, zsh, fish, and
/// Nushell where each shell looks for them under `share`
fn complete(share: &Path) -> io::Result<()> {
    [
        (script(Shell::Bash), "bash-completion/completions/cinq"),
        (script(Shell::Zsh), "zsh/site-functions/_cinq"),
        (script(Shell::Fish), "fish/vendor_completions.d/cinq.fish"),
        (script(Nushell), "nushell/vendor/autoload/cinq.nu"),
    ]
    .into_iter()
    .try_for_each(|(script, file)| write(&share.join(file), script))
}

/// Returns the completions of the command line that `generator` writes
fn script(generator: impl Generator) -> Vec<u8> {
    let mut script = Vec::new();
    generate(generator, &mut Cli::command(), "cinq", &mut script);
    script
}

/// Draws the logo into `directory` as the icon of the desktop entry
fn draw(directory: &Path) -> io::Result<()> {
    write(&directory.join("cinq.svg"), logo::svg())
}

/// Lays out the app bundle for Darwin in the Applications directory beside
/// `share`: Info.plist, the icon, and the empty directory for the program,
/// which the build script runs too early to fill
fn assemble(share: &Path) -> io::Result<()> {
    let contents = share.with_file_name("Applications/cinq.app/Contents");
    fs::create_dir_all(contents.join("MacOS"))?;
    write(&contents.join("Info.plist"), bundle::info_plist())?;
    write(&contents.join("Resources/cinq.icns"), bundle::icns()?)
}

/// Renders the manual pages for the platform of the build into the
/// directory of each one's section under `man`
fn render(man: &Path) -> io::Result<()> {
    let target = env::var("CARGO_CFG_TARGET_OS").expect("cargo names the target");
    let platform = Platform::of(&target);
    let page = |name: &str| {
        let section = Path::new(name)
            .extension()
            .expect("a page names its section");
        man.join(format!("man{}", section.display())).join(name)
    };
    write(&page("cinq.1"), man::command_page(platform, &date()))?;
    man::SCDOC
        .iter()
        .try_for_each(|(name, source)| scdoc(&man::for_platform(source, platform), &page(name)))
}

/// Returns the date of the build, written YYYY-MM-DD: that of
/// `SOURCE_DATE_EPOCH`, in seconds since 1970, when it is set, or else today
fn date() -> String {
    let time = env::var_os("SOURCE_DATE_EPOCH").map_or_else(OffsetDateTime::now_utc, |seconds| {
        seconds
            .to_str()
            .and_then(|seconds| seconds.parse().ok())
            .and_then(|seconds| OffsetDateTime::from_unix_timestamp(seconds).ok())
            .expect("SOURCE_DATE_EPOCH holds a time in seconds since 1970")
    });
    time.date().to_string()
}

/// Renders the scdoc `source` into the manual page `page` with scdoc, which
/// dates it as [`date`] does.  The source reaches scdoc through a file in
/// the output directory of the build, and the page is written only once
/// scdoc has rendered all of it, so that a failure leaves no page behind.
fn scdoc(source: &str, page: &Path) -> io::Result<()> {
    let name = page
        .file_name()
        .expect("a page is a file")
        .to_string_lossy();
    let input = Path::new(&env::var_os("OUT_DIR").expect("cargo names the output directory"))
        .join(format!("{name}.scd"));
    fs::write(&input, source)?;
    let output = Command::new("scdoc")
        .stdin(File::open(&input)?)
        .output()
        .map_err(|error| io::Error::new(error.kind(), format!("cannot run scdoc: {error}")))?;
    if output.status.success() {
        write(page, output.stdout)
    } else {
        let reason = String::from_utf8_lossy(&output.stderr);
        Err(io::Error::other(format!("scdoc failed: {}", reason.trim())))
    }
}
