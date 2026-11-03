// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Manual pages, which the build script renders for the platform that it
//! builds for
//!
//! cinq(1) comes from the command line, which clap_mangen renders.  cinq(5)
//! and cinq-show(5) come from their scdoc sources, of which the build keeps
//! only the parts for the platform, for scdoc to render in turn.  The pages
//! differ by platform only in where the configuration file and the unsaved
//! changes are, which the dirs crate decides, and in the Wayland compositor
//! that the window needs on GNU/Linux and the BSDs.  The program itself
//! leaves this module out: only the build script and the tests compile it.

#[cfg(test)]
mod tests;

use std::io::{self, Write};

use clap::CommandFactory;
use clap_mangen::{
    Man,
    roff::{Inline, Roff, bold, italic, line_break, roman},
};
use strum::{IntoStaticStr, VariantArray};

use super::cli::Cli;

/// Pages that scdoc renders, by name, with their sources: cinq(5), of the
/// configuration file, and cinq-show(5), of show files
pub const SCDOC: [(&str, &str); 2] = [
    (
        "cinq.5",
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/man/cinq.5.scd")),
    ),
    (
        "cinq-show.5",
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/man/cinq-show.5.scd")),
    ),
];

/// Line of a source in scdoc that starts a part for the platforms named
/// after it, which lasts until the next such line or [`END`]
const IF: &str = "; if ";

/// Line of a source in scdoc that ends a part for some platforms, after
/// which the source is for every platform again
const END: &str = "; end";

/// Section of cinq(1) as clap_mangen renders it
type Section = fn(&Man, &mut dyn Write) -> io::Result<()>;

/// Sections of cinq(1) that clap_mangen derives from the command line, in
/// the order of man-pages(7)
const SECTIONS: [Section; 5] = [
    Man::render_title,
    Man::render_name_section,
    Man::render_synopsis_section,
    Man::render_description_section,
    Man::render_options_section,
];

/// Platform, as far as the manual pages tell platforms apart: by where the
/// dirs crate finds the configuration directory
#[derive(Debug, Clone, Copy, PartialEq, Eq, VariantArray, IntoStaticStr)]
#[strum(serialize_all = "lowercase")]
pub enum Platform {
    /// POSIX systems that follow the XDG Base Directory Specification, such
    /// as GNU/Linux and the BSDs
    Xdg,
    /// Apple's systems, whose directory is `~/Library/Application Support`
    Macos,
    /// Windows, whose directory holds the roaming application data
    Windows,
}

impl Platform {
    /// Returns the platform of a target, which Rust names by its
    /// `target_os`, as the dirs crate tells targets apart
    pub fn of(target_os: &str) -> Self {
        match target_os {
            "windows" => Platform::Windows,
            "macos" | "ios" => Platform::Macos,
            _ => Platform::Xdg,
        }
    }

    /// Returns the name that marks the parts of the scdoc sources for the
    /// platform
    fn name(self) -> &'static str {
        self.into()
    }

    /// Returns the configuration file, as users of the platform write it
    fn config_file(self) -> &'static str {
        match self {
            Platform::Xdg => "$XDG_CONFIG_HOME/cinq/config.toml",
            Platform::Macos => "~/Library/Application Support/cinq/config.toml",
            Platform::Windows => r"%APPDATA%\cinq\config.toml",
        }
    }

    /// Returns the directory that keeps unsaved changes, as users of the
    /// platform write it
    fn unsaved_directory(self) -> &'static str {
        match self {
            Platform::Xdg => "$XDG_STATE_HOME/cinq/unsaved",
            Platform::Macos => "~/Library/Application Support/cinq/unsaved",
            Platform::Windows => r"%LOCALAPPDATA%\cinq\unsaved",
        }
    }
}

/// Renders cinq(1) for `platform`, dated `date`: the sections that
/// clap_mangen derives from the command line, then those that it cannot.
/// Each section comes with the same preamble, which defines the
/// apostrophe, and the page keeps it only once.
pub fn command_page(platform: Platform, date: &str) -> String {
    let command = Cli::command();
    // man-pages(7) starts the summary in the NAME section in lower case
    let summary = command
        .get_about()
        .map(ToString::to_string)
        .unwrap_or_default();
    // Notice that --version prints after the version, with its link on a
    // line of its own, so that roff neither hyphenates the link nor spreads
    // out the words before it
    let notice = command
        .get_long_version()
        .and_then(|version| version.split_once('\n'))
        .map_or_else(String::new, |(_, notice)| notice.replace(" <", "\n<"));
    let man = Man::new(command.about(uncapitalized(&summary)))
        .date(date)
        .manual("General Commands Manual");
    let rendered = |render: &Section| {
        let mut section = Vec::new();
        render(&man, &mut section).expect("writing to memory does not fail");
        String::from_utf8(section).expect("the page is UTF-8")
    };
    let preamble = Roff::new().render();
    let body: String = SECTIONS
        .iter()
        .map(rendered)
        .chain([epilogue(platform, &notice).render()])
        .map(|section| section.replacen(&preamble, "", 1))
        .collect();
    [preamble, body].concat()
}

/// Returns the sections of cinq(1) that clap_mangen cannot derive from the
/// command line: the variables that name the Wayland compositor where
/// `platform` needs one, the configuration file and the directory of
/// unsaved changes of `platform`, the copyright `notice`, and the pages to
/// see also
fn epilogue(platform: Platform, notice: &str) -> Roff {
    // Lines of the notice, each after a line break but the first
    let notice: Vec<Inline> = notice
        .lines()
        .flat_map(|line| [line_break(), roman(line)])
        .skip(1)
        .collect();
    let mut roff = Roff::new();
    if platform == Platform::Xdg {
        roff.control("SH", ["ENVIRONMENT"])
            .control("TP", [])
            .text([bold("WAYLAND_DISPLAY"), roman(", "), bold("WAYLAND_SOCKET")])
            .text([
                roman("Wayland compositor that the window opens on, which "),
                bold("cinq"),
                roman(" needs, since it does not run on X11."),
            ]);
    }
    roff.control("SH", ["FILES"])
        .control("TP", [])
        .text([italic(platform.config_file())])
        .text([
            roman("The configuration file, which "),
            bold("cinq"),
            roman("(5) describes."),
        ])
        .control("TP", [])
        .text([italic(platform.unsaved_directory())])
        .text([
            roman("Directory that keeps the unsaved changes of shows, which "),
            bold("cinq-show"),
            roman("(5) describes."),
        ])
        .control("SH", ["COPYRIGHT"])
        .text(notice)
        .control("SH", ["SEE ALSO"])
        .text([
            bold("cinq"),
            roman("(5), "),
            bold("cinq-show"),
            roman("(5)"),
        ]);
    roff
}

/// Returns the scdoc `source` of a page for `platform`, without the parts
/// for other platforms, and without the lines that mark the parts
pub fn for_platform(source: &str, platform: Platform) -> String {
    source
        .lines()
        .scan(true, |kept, line| {
            Some(match line.strip_prefix(IF) {
                Some(platforms) => {
                    *kept = platforms
                        .split_whitespace()
                        .any(|name| name == platform.name());
                    None
                }
                None if line == END => {
                    *kept = true;
                    None
                }
                None => kept.then_some(line),
            })
        })
        .flatten()
        .flat_map(|line| [line, "\n"])
        .collect()
}

/// Returns the text with its first letter in lower case
fn uncapitalized(text: &str) -> String {
    text.chars()
        .take(1)
        .flat_map(char::to_lowercase)
        .chain(text.chars().skip(1))
        .collect()
}
