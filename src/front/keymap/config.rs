// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Configuration file, which chooses a keymap and binds actions to keys
//!
//! The file is `cinq/config.toml` in the platform's configuration
//! directory, as the dirs crate finds it, unless the command line names a
//! directory of its own to read `config.toml` from.  [`watch`](fn@watch)
//! reads the file again whenever it changes, so that the keys follow the
//! file while the program runs.
//!
//! `keymap` names the keymap that the keys start from, which is vi unless
//! the file names another: a shipped one, or one of the `[keymaps]` table,
//! which starts from the keymap that its `from` names, or from no keys at
//! all without one.  A keymap of the file's own and the `[keys]` table bind
//! each action they name to a key sequence or a list of them, in place of
//! the keys that the keymap they build on gives it:
//!
//! ```toml
//! keymap = "booth"
//!
//! [keymaps.booth]
//! from = "cua"
//! go = ["Space", "F5"]
//!
//! [keys]
//! delete = []
//! ```

mod watch;

use std::{
    fs::{self, File},
    io::{self, Write},
    path::{self, Path, PathBuf},
    slice,
};

use itertools::Itertools;
use toml::{Table, Value};

use super::{Action, Keymap, Preset, Press, binding};
use crate::front::read;
pub use watch::watch;

/// Settings that the file can hold
const SETTINGS: [&str; 3] = ["keymap", "keymaps", "keys"];

/// Key bindings, along with where they came from and what was wrong with
/// the file they came from
#[derive(Debug, Clone, Default)]
pub struct Config {
    pub keymap: Keymap,
    pub source: Source,
    /// Problems with the file, each of which leaves the keys of the keymap
    /// in place, and each written to follow the name of the file
    pub problems: Vec<String>,
}

/// Entry of a table that binds keys, `[keys]` or a keymap of the file's
/// own: an action and the key sequences bound to it
type Entry = (Action, Vec<Vec<Press>>);

/// Where the key bindings came from
#[derive(Debug, Clone, Default)]
pub enum Source {
    /// Configuration file that is there, even when it cannot be read
    File(PathBuf),
    /// Defaults, which the missing file would change
    Missing(PathBuf),
    /// Defaults, since the platform has no configuration directory
    #[default]
    NoDirectory,
}

impl Source {
    /// Returns the configuration file, which the keys follow as it changes
    pub fn file(&self) -> Option<&Path> {
        match self {
            Source::File(file) | Source::Missing(file) => Some(file),
            Source::NoDirectory => None,
        }
    }
}

/// Returns the file that may hold the configuration: `config.toml` in
/// `directory`, when one is given, or else `cinq/config.toml` in the
/// platform's configuration directory, as the dirs crate finds it, if it
/// has one.  The directory goes by its absolute path, which the watcher
/// reports changes by.
pub fn file(directory: Option<&Path>) -> Option<PathBuf> {
    directory
        .map(|directory| path::absolute(directory).unwrap_or_else(|_| directory.to_owned()))
        .or_else(|| dirs::config_dir().map(|directory| directory.join("cinq")))
        .map(|directory| directory.join("config.toml"))
}

/// Text of a configuration file that [`make`] makes: a line that points
/// to the manual page of the file, and nothing that changes a key
const MADE: &str = "# Key bindings of cinq, which the manual page cinq(5) describes\n";

/// Makes the configuration file `file`, along with the directories that it
/// goes in, holding [`MADE`], unless the file is there already
pub fn make(file: &Path) -> io::Result<()> {
    if let Some(directory) = file.parent() {
        fs::create_dir_all(directory)?;
    }
    match File::create_new(file) {
        Ok(mut made) => made.write_all(MADE.as_bytes()),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(error),
    }
}

/// Reads the configuration from `file`, or gives the defaults when there
/// is no such file
pub fn load(file: Option<&Path>) -> Config {
    let Some(file) = file else {
        return Config::default();
    };
    match read(file) {
        Ok(Some(text)) => {
            let (keymap, problems) = parse(&text);
            Config {
                keymap,
                source: Source::File(file.to_owned()),
                problems,
            }
        }
        Err(error) => Config {
            source: Source::File(file.to_owned()),
            problems: vec![format!("cannot be read: {error}")],
            ..Config::default()
        },
        Ok(None) => Config {
            source: Source::Missing(file.to_owned()),
            ..Config::default()
        },
    }
}

/// Reads the text of a configuration file into key bindings, along with
/// what was wrong with it
pub(super) fn parse(text: &str) -> (Keymap, Vec<String>) {
    let table = match text.parse::<Table>() {
        Ok(table) => table,
        Err(error) => {
            let line = error.span().map_or_else(String::new, |span| {
                format!(" on line {}", text[..span.start].matches('\n').count() + 1)
            });
            let problem = format!("is not TOML{line}: {}", error.message());
            return (Keymap::default(), vec![problem]);
        }
    };
    let unknown = table
        .keys()
        .filter(|name| !SETTINGS.contains(&name.as_str()))
        .map(|name| format!("no setting is named `{name}`"));
    let none = Table::new();
    let (keymaps, malformed) = match table.get("keymaps") {
        Some(Value::Table(keymaps)) => (keymaps, Vec::new()),
        Some(_) => (&none, vec!["keymaps should be a table".to_owned()]),
        None => (&none, Vec::new()),
    };
    let shadowed = keymaps
        .keys()
        .filter(|name| name.parse::<Preset>().is_ok())
        .map(|name| format!("keymaps.{name} goes unused, since {name} is a shipped keymap"));
    let default = Preset::default().name();
    let (name, unnamed) = match table.get("keymap") {
        Some(Value::String(name)) => (name.as_str(), None),
        Some(_) => (default, Some("keymap should be the name of a keymap")),
        None => (default, None),
    };
    let (chosen, built) =
        build(name, keymaps, &[]).unwrap_or_else(|problem| (Keymap::default(), vec![problem]));
    let (keymap, unbound) = match table.get("keys") {
        Some(Value::Table(keys)) => bind(chosen, keys, ""),
        Some(_) => (chosen, vec!["keys should be a table".to_owned()]),
        None => (chosen, Vec::new()),
    };
    let problems = unknown
        .chain(malformed)
        .chain(shadowed)
        .chain(unnamed.map(str::to_owned))
        .chain(built)
        .chain(unbound)
        .chain(keymap.conflicts())
        .collect();
    (keymap, problems)
}

/// Builds the keymap named `name`: a shipped one, or one of `keymaps`,
/// which starts from the keymap that its `from` names, or from no keys at
/// all without one, and binds the keys of its other entries.  `chain`
/// holds the keymaps that start from this one, so that a keymap that comes
/// back around to itself starts from no keys instead.
fn build(name: &str, keymaps: &Table, chain: &[&str]) -> Result<(Keymap, Vec<String>), String> {
    if let Ok(preset) = name.parse::<Preset>() {
        return Ok((preset.into(), Vec::new()));
    }
    let entries = match keymaps.get(name) {
        Some(Value::Table(entries)) => entries,
        Some(_) => return Err(format!("keymaps.{name} should be a table")),
        None => return Err(format!("no keymap is named `{name}`")),
    };
    if chain.contains(&name) {
        return Err(format!("keymap {name} starts from itself"));
    }
    let chain = [chain, &[name]].concat();
    let start = match entries.get("from") {
        Some(Value::String(from)) => build(from, keymaps, &chain),
        Some(_) => Err(format!(
            "keymaps.{name}.from should be the name of a keymap"
        )),
        None => Ok((Keymap::empty(name), Vec::new())),
    };
    let (start, problems) = start.unwrap_or_else(|problem| (Keymap::empty(name), vec![problem]));
    let bindings = entries.iter().filter(|(key, _)| key.as_str() != "from");
    let (keymap, unbound) = bind(start.named(name), bindings, &format!("keymaps.{name}: "));
    Ok((keymap, [problems, unbound].concat()))
}

/// Binds the keys of each entry of `entries` to its action in place of the
/// keys that `keymap` gives it, along with the problems of the entries,
/// each written after `place`
fn bind<'a>(
    keymap: Keymap,
    entries: impl IntoIterator<Item = (&'a String, &'a Value)>,
    place: &str,
) -> (Keymap, Vec<String>) {
    let (entries, problems): (Vec<Option<Entry>>, Vec<Vec<String>>) = entries
        .into_iter()
        .map(|(name, value)| entry(name, value))
        .unzip();
    let keymap = entries
        .into_iter()
        .flatten()
        .fold(keymap, |keymap, (action, keys)| keymap.with(action, keys));
    let problems = problems
        .into_iter()
        .flatten()
        .map(|problem| format!("{place}{problem}"))
        .collect();
    (keymap, problems)
}

/// Reads the keys that one entry of the `[keys]` table or of a keymap binds
/// to its action, along with the problems that leave some of them out
fn entry(name: &str, value: &Value) -> (Option<Entry>, Vec<String>) {
    let Ok(action) = name.parse::<Action>() else {
        return (None, vec![format!("no action is named `{name}`")]);
    };
    let values = match value {
        Value::Array(values) => values.as_slice(),
        value => slice::from_ref(value),
    };
    let (sequences, problems): (Vec<_>, Vec<_>) = values
        .iter()
        .map(|value| match value {
            Value::String(text) => binding(text).map_err(|problem| format!("{name}: {problem}")),
            _ => Err(format!("{name} should be keys or a list of keys")),
        })
        .partition_result();
    // Sequence listed twice binds it once
    let sequences = sequences.into_iter().unique().collect();
    (Some((action, sequences)), problems)
}
