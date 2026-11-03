// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Keymaps that cinq ships, which the configuration file chooses from or
//! builds on
//!
//! [`Preset::Vi`] binds keys in the manner of vi, and [`Preset::Cua`] as
//! IBM's Common User Access does, which most desktop programs follow.  Each
//! keymap lists every action, which the compiler checks, so that no keymap
//! forgets an action added later.

mod cua;
mod vi;

use strum::{EnumString, IntoStaticStr, VariantArray};

use super::Action;

/// Keymap that cinq ships, which the configuration file names in lower case
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, EnumString, IntoStaticStr, VariantArray)]
#[strum(serialize_all = "lowercase")]
pub enum Preset {
    #[default]
    Vi,
    Cua,
}

impl Preset {
    /// Returns the name of the keymap in the configuration file
    pub(super) fn name(self) -> &'static str {
        self.into()
    }

    /// Returns the key sequences that the keymap binds to an action, as the
    /// configuration file writes them
    pub(super) fn keys(self, action: Action) -> &'static [&'static str] {
        match self {
            Preset::Vi => vi::keys(action),
            Preset::Cua => cua::keys(action),
        }
    }
}
