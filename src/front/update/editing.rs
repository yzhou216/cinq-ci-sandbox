// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Adding, moving, and deleting cues around the selection, choosing the
//! sound files of audio cues, and undoing and redoing changes to the show

use iced::Task;
use itertools::Itertools;

use crate::{
    back::{Change, Command, CueId, Place, SOUND_FILES},
    front::{App, Message},
};

impl App {
    /// Adds a cue next to the selection, on the side that `place` picks, or
    /// at the end of the list without a selection.  The new cue is
    /// selected, if the back end added it.
    pub(super) fn with_new_cue(self, place: fn(CueId) -> Place) -> Self {
        let id = self.workspace.next_cue_id();
        let add = Command::Add {
            list: self.list,
            kind: self.new_kind,
            at: self.selected_here().map_or(Place::End, place),
        };
        let app = self.send(add);
        Self {
            selected: id
                .filter(|&id| app.workspace.cue(id).is_some())
                .or(app.selected),
            ..app
        }
    }

    /// Asks which sound file the audio cue `id` plays, in the desktop's
    /// file chooser, which starts in the directory of the file that the cue
    /// plays now, or else in the directory that relative paths start from
    pub(super) fn choose_sound(&self, id: CueId) -> Task<Message> {
        let Some(cue) = self.workspace.cue(id) else {
            return Task::none();
        };
        // File choosers on GNU/Linux tell endings apart by case
        let endings = SOUND_FILES
            .into_iter()
            .flat_map(|ending| [ending.to_owned(), ending.to_uppercase()])
            .collect_vec();
        let chooser = rfd::AsyncFileDialog::new()
            .set_title("Choose sound file")
            .add_filter("Sound files", &endings)
            .add_filter("All files", &["*"])
            .set_directory(self.workspace.sound_directory(cue));
        Task::future(chooser.pick_file())
            .and_then(move |chosen| Task::done(Message::SoundChosen(id, chosen.path().to_owned())))
    }

    /// Moves the selected cue by a number of places in its list, downwards
    /// when positive, which the selection follows
    pub(super) fn with_selected_moved(self, by: isize) -> Self {
        match self.selected_here() {
            Some(cue) => self.send(Command::Move { cue, by }),
            None => self,
        }
    }

    /// Deletes a number of cues from the selection down, as one change that
    /// undo takes back whole, and selects the cue after them, or the one
    /// before them at the end of the list.  A count past the end of the
    /// list deletes to the end.
    pub(super) fn without_selected(self, count: usize) -> Self {
        let Some(first) = self.selected_row() else {
            return self;
        };
        let ids: Vec<CueId> = self.current().cues[first..]
            .iter()
            .take(count)
            .map(|cue| cue.id)
            .collect();
        self.send(Command::Delete(ids)).showing(None, Some(first))
    }

    /// Takes back the latest change to the show, and selects the cue it
    /// brought back or changed back
    pub(super) fn undone(self) -> Self {
        let cue = self.workspace.undoable().and_then(Change::cue);
        let row = self.selected_row();
        self.send(Command::Undo).showing(cue, row)
    }

    /// Makes again the change that undo took back last, and selects the cue
    /// it added or changed
    pub(super) fn redone(self) -> Self {
        let cue = self.workspace.redoable().and_then(Change::cue);
        let row = self.selected_row();
        self.send(Command::Redo).showing(cue, row)
    }

    /// Selects a cue that a change touched, if it is still there.  A
    /// selected cue that the change took away, from the row `row`, leaves
    /// the selection to the cue that has taken its place, or to the last
    /// cue when none has.
    fn showing(self, cue: Option<CueId>, row: Option<usize>) -> Self {
        let gone = |id: &CueId| self.workspace.cue(*id).is_none();
        match cue.filter(|id| !gone(id)) {
            Some(id) => self.with_selection(id),
            None if self.selected.as_ref().is_some_and(gone) => match row {
                Some(row) => self.at_row(row + 1),
                None => Self {
                    selected: None,
                    ..self
                },
            },
            None => self,
        }
    }
}
