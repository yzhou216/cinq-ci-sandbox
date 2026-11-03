<!--
SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>

SPDX-License-Identifier: GPL-3.0-or-later
-->

# cinq (CINQ Is Not QLab)

Minimal cue-based show controller, the
[free](https://www.fsf.org/about/what-is-free-software) alternative to QLab

## Features

- **Built not to stall.**  The sounds of the next GO are opened and decoded
  ahead of time, and loading, saving, and keeping unsaved changes all run in
  the background, so a slow disk does not hold up GO or Panic.
- **Your work survives.**  Unsaved changes are kept as you make them, through
  closing the window or a crash, and come back as one change that undo takes
  back.  Undo covers every edit, and nothing ever asks before acting.
- **Keyboard first.**  vi and CUA keymaps, rebindable in a file that reloads
  as you save it.  GO and Panic act from anywhere, even halfway through
  another key sequence.
- **Audio cues** in WAV, FLAC, Ogg Vorbis, or MP3, with start and end times,
  seamless loops, and levels that change while they play.
- **Fade, stop, wait, and memo cues**, chained by auto-continue and
  auto-follow, with pre-waits and post-waits.
- **Panic** fades everything out in a second, and a second Panic stops at
  once.
- **Show mode** locks editing during a performance.
- **Several cue lists**, and a warnings list that says why any cue cannot
  play.
- **Readable show files** in TOML, which `cinq SHOW` opens, easy to diff and
  keep under version control.

## Building

### Requirements

#### General

- Cargo
- [scdoc](https://git.sr.ht/~sircmpwn/scdoc)

#### GNU/Linux and the BSDs

- pkg-config
- ALSA
- Wayland
- libxkbcommon
- D-Bus
- XDG desktop portal with a file chooser

### Installing

Every build lays out what cinq installs besides its program in
`target/share`, as the share directory of a prefix holds it: the manual
pages, completions for bash, zsh, fish, and Nushell, and, for GNU/Linux
and the BSDs, the desktop entry with its icon and the type of show files.
The Nix package installs it all.  Elsewhere, after `cargo build --release`,
copy `target/release/cinq` into the `bin` directory of the prefix, which is
often `/usr`, and what `target/share` holds into its `share` directory,
then run `update-mime-database` with its `share/mime` directory.

Builds for Darwin also lay out the app bundle, `cinq.app`, in
`target/Applications`, with all but the program, which goes into its
`Contents/MacOS`.  On GNU/Linux, `nix build .#aarch64-darwin-cross`
cross-builds the package for Apple silicon.

## Configuration

See [cinq(5)](man/cinq.5.scd).

## Show files

See [cinq-show(5)](man/cinq-show.5.scd).

## License

cinq is free software: you can redistribute it and/or modify it under the terms
of the GNU General Public License as published by the Free Software Foundation,
either version 3 of the License, or any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.
