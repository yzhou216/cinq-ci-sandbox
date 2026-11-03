// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Actions that keys can be bound to

use strum::{EnumString, IntoStaticStr, VariantArray};

use crate::{
    back::Place,
    front::{Focus, Message},
};

/// Something the operator can do with a key, which the configuration file
/// names in snake case, as `next_list`.  [`Action::VARIANTS`] lists every
/// action in the order of the help page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumString, IntoStaticStr, VariantArray)]
#[strum(serialize_all = "snake_case")]
pub enum Action {
    Go,
    Panic,
    Next,
    Previous,
    NextList,
    PreviousList,
    First,
    Last,
    NextTab,
    PreviousTab,
    Standby,
    Edit,
    NextField,
    PreviousField,
    AddBelow,
    AddAbove,
    MoveDown,
    MoveUp,
    Delete,
    Undo,
    Redo,
    Open,
    Save,
    SaveAs,
    Help,
    CloseHelp,
}

impl Action {
    /// Returns the name of the action in the configuration file
    pub fn name(self) -> &'static str {
        self.into()
    }

    /// Returns what the action does, as the help page tells it
    pub fn description(self) -> &'static str {
        match self {
            Action::Go => "GO: fire the standby cue and its chain",
            Action::Panic => "Panic: fade everything out, or stop it at once",
            Action::Next => "Select the next cue or menu option",
            Action::Previous => "Select the previous cue or menu option",
            Action::NextList => "Show the next cue list",
            Action::PreviousList => "Show the previous cue list",
            Action::First => "Select the first cue, or the row counted",
            Action::Last => "Select the last cue, or the row counted",
            Action::NextTab => "Show the next inspector tab",
            Action::PreviousTab => "Show the previous inspector tab",
            Action::Standby => "Put the selected cue on standby",
            Action::Edit => "Edit the name of the selected cue",
            Action::NextField => "Move to the next field",
            Action::PreviousField => "Move to the previous field",
            Action::AddBelow => "Add a cue below the selection",
            Action::AddAbove => "Add a cue above the selection",
            Action::MoveDown => "Move the selected cue down",
            Action::MoveUp => "Move the selected cue up",
            Action::Delete => "Delete the selected cue",
            Action::Undo => "Undo the latest change to the show",
            Action::Redo => "Redo the change undone last",
            Action::Open => "Open a show",
            Action::Save => "Save the show",
            Action::SaveAs => "Save the show in another file",
            Action::Help => "Show or hide this help",
            Action::CloseHelp => "Hide this help",
        }
    }

    /// Actions that keep the show in hand wherever the keyboard is: their
    /// keys act alone, with any modifiers held, and over text fields, menus,
    /// and the help page alike
    pub(super) const AT_ONCE: [Action; 2] = [Action::Go, Action::Panic];

    /// Returns whether the action is one of [`Action::AT_ONCE`]
    pub fn acts_at_once(self) -> bool {
        Self::AT_ONCE.contains(&self)
    }

    /// Returns whether the keys of the action act in a text field, as the
    /// keys of GO, Panic, the moves between fields, and those of opening and
    /// saving shows do
    pub(super) fn acts_in_fields(self) -> bool {
        self.acts_at_once()
            || matches!(
                self,
                Action::NextField
                    | Action::PreviousField
                    | Action::Open
                    | Action::Save
                    | Action::SaveAs
            )
    }

    /// Returns whether the action acts while the help page is open, as GO,
    /// Panic, and the actions that show and hide the page do
    pub fn acts_over_help(self) -> bool {
        self.acts_at_once() || matches!(self, Action::Help | Action::CloseHelp)
    }

    /// Returns the message that the action sends.  A count typed before
    /// its keys repeats a move of the selection or of a cue, deletes as many
    /// cues, or picks the row that [`Action::First`] and [`Action::Last`] go
    /// to.
    pub fn message(self, count: Option<usize>) -> Message {
        let times = count.unwrap_or(1);
        let step = isize::try_from(times).unwrap_or(isize::MAX);
        match self {
            Action::Go => Message::Go,
            Action::Panic => Message::Panic,
            Action::Next => Message::Step(step),
            Action::Previous => Message::Step(-step),
            Action::NextList => Message::StepList(step),
            Action::PreviousList => Message::StepList(-step),
            Action::First => Message::Row(times),
            Action::Last => Message::Row(count.unwrap_or(usize::MAX)),
            Action::NextTab => Message::StepTab(1),
            Action::PreviousTab => Message::StepTab(-1),
            Action::Standby => Message::StandbySelected,
            Action::Edit => Message::Focus(Focus::Name),
            Action::NextField => Message::Focus(Focus::Next),
            Action::PreviousField => Message::Focus(Focus::Previous),
            Action::AddBelow => Message::Add(Place::After),
            Action::AddAbove => Message::Add(Place::Before),
            Action::MoveDown => Message::Move(step),
            Action::MoveUp => Message::Move(-step),
            Action::Delete => Message::Delete(times),
            Action::Undo => Message::Undo,
            Action::Redo => Message::Redo,
            Action::Open => Message::Open,
            Action::Save => Message::Save,
            Action::SaveAs => Message::SaveAs,
            Action::Help => Message::ToggleHelp,
            Action::CloseHelp => Message::CloseHelp,
        }
    }
}
