// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Cue list, one row per cue, under a header that stays in view
//!
//! The selected row stands out on the background of the selection, and its
//! radio button marks it too.  The selection only drives the inspector,
//! while the State column marks the standby cue that GO fires as NEXT.
//! Each column is as wide as its title or its longest text, counted in the
//! columns of the monospace font, of which a wide character such as one of
//! Chinese takes two, so that the header, a table of its own above the rows
//! that scroll, lines up with them.  The number makes room for its radio
//! button as well, the target wraps past [`TARGET_ROOM`] columns of the
//! font, and the state keeps the width of [`STATE_ROOM`] while cues run.
//! The name takes the room that the other columns leave, and when the list
//! is too narrow for a name to be read, the columns of the waits, of what
//! follows, of the kind of cue, and of its target give way, those empty in
//! every row first, since the inspector shows them all.

use iced::{
    Element, Fill, Length, Theme, padding,
    widget::{
        Radio, column, container, mouse_area, radio, responsive, rule, scrollable, space, table,
        text, text::Wrapping,
    },
};
use itertools::Itertools;
use strum::{IntoStaticStr, VariantArray};
use unicode_width::UnicodeWidthStr;

use super::{CUES, SELECTED, char_width, format, giving_way, marked_row};
use crate::{
    back::{Cue, CueList, Problem, Workspace},
    front::{App, Message, keymap::Action},
};

/// Width of the scroll bar, which the list sets and leaves free at its
/// right, so that the bar covers no text and the rows keep the width of the
/// header with or without the bar
const SCROLL_BAR: f32 = 10.0;

/// Space above and below the text of each row, which is iced's own padding
/// for tables
const ROW_PADDING: f32 = 5.0;

/// Space on either side of the text of each cell, which is iced's own
/// padding for tables, and which both tables set, since the widths of the
/// columns count it
const CELL_PADDING: f32 = 10.0;

/// Width of the line between columns, which both tables set as well
const SEPARATOR: f32 = 1.0;

/// Space that each column takes besides its text
const CELL_SPACE: f32 = 2.0 * CELL_PADDING + SEPARATOR;

/// Width of iced's radio button and the space after it, before its label
const RADIO: f32 = Radio::<Message>::DEFAULT_SIZE + Radio::<Message>::DEFAULT_SPACING;

/// Fewest columns of the font that the list keeps for a name before any
/// column gives way
const NAME_ROOM: usize = 16;

/// Most columns of the font that the Target column takes, past which a
/// long file name wraps
const TARGET_ROOM: usize = 24;

/// Columns of the font that the State column holds, so that the column
/// keeps its width as cues run: as many as `10:00.0 left` takes, the
/// longest state of a cue whose sound is under a hundred minutes
const STATE_ROOM: usize = 12;

/// Column of the cue list, which goes by its name, apart from the number
#[derive(Debug, Clone, Copy, PartialEq, Eq, VariantArray, IntoStaticStr)]
enum Column {
    #[strum(serialize = "#")]
    Number,
    Type,
    Name,
    Target,
    Pre,
    Action,
    Post,
    Then,
    State,
}

impl Column {
    /// Columns that give way in turn when the list is too narrow to leave a
    /// name room to be read, after those that are empty in every row
    const YIELDING: [Column; 5] = [
        Column::Post,
        Column::Pre,
        Column::Then,
        Column::Type,
        Column::Target,
    ];

    fn title(self) -> &'static str {
        self.into()
    }

    /// Returns the text of the cell of `cue` in this column
    fn text(self, workspace: &Workspace, list: &CueList, cue: &Cue) -> String {
        match self {
            Column::Number => cue.number.clone(),
            Column::Type => cue.kind.to_string(),
            Column::Name => cue.name.clone(),
            Column::Target => format::target(workspace, cue),
            Column::Pre => format::wait(cue.pre_wait.value().into()),
            Column::Action => format::action(cue),
            Column::Post => format::post_wait(cue),
            Column::Then => format::advance(cue.advance).to_owned(),
            Column::State => state(workspace, list, cue).0,
        }
    }

    /// Returns whether the column has text in any row of `list`
    fn holds_text(self, workspace: &Workspace, list: &CueList) -> bool {
        list.cues
            .iter()
            .any(|cue| !self.text(workspace, list, cue).is_empty())
    }

    /// Returns the width of the column, as wide as its title or its longest
    /// cell, or none for the name, which takes the room left
    fn width(self, workspace: &Workspace, list: &CueList) -> Option<f32> {
        let widest = || {
            list.cues
                .iter()
                .map(|cue| self.text(workspace, list, cue).width())
                .fold(self.title().width(), usize::max)
        };
        let width = |columns: usize| columns as f32 * char_width();
        match self {
            Column::Name => None,
            Column::Number => Some(width(widest()) + RADIO),
            Column::Target => Some(width(widest().min(TARGET_ROOM))),
            Column::State => Some(width(STATE_ROOM)),
            _ => Some(width(widest())),
        }
    }
}

pub(super) fn view(app: &App) -> Element<'_, Message> {
    responsive(move |size| list(app, size.width - SCROLL_BAR)).into()
}

/// Draws the cue list `room` wide: a header, and under it the rows, which
/// scroll in a table of their own that has no header, or a word on adding
/// a cue while the list has none
fn list<'a>(app: &'a App, room: f32) -> Element<'a, Message> {
    let fitting = fitting(app, room);
    let table = |titled: bool, cues: &'a [Cue]| {
        let columns = fitting
            .iter()
            .map(move |&(column, width)| table_column(app, column, width, titled));
        table::table(columns, cues)
            .padding_x(CELL_PADDING)
            .separator_x(SEPARATOR)
    };
    let free = padding::right(SCROLL_BAR);
    let header = table(true, &[]).padding_y(ROW_PADDING);
    let cues = &app.current().cues;
    let rows = if cues.is_empty() {
        empty(app)
    } else {
        let rows = table(false, cues).padding_y(0);
        marked_row(rows, fitting.len(), app.selected_row(), SELECTED)
    };
    column![
        container(header).padding(free),
        // Line under the header, which the rows scroll under
        rule::horizontal(1),
        scrollable(container(rows).padding(free))
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new()
                    .width(SCROLL_BAR)
                    .scroller_width(SCROLL_BAR),
            ))
            .id(CUES)
            .height(Fill),
    ]
    .into()
}

/// Tells how to add a cue to a list that has none, or that show mode locks
/// editing
fn empty(app: &App) -> Element<'_, Message> {
    let hint = match app.config.keymap.key(Action::AddBelow) {
        _ if !app.editable() => "No cues yet.  Leave show mode to add one.".to_owned(),
        Some(keys) => format!("No cues yet.  Press {keys} or Add cue to add one."),
        None => "No cues yet.  Press Add cue to add one.".to_owned(),
    };
    container(text(hint))
        .padding([ROW_PADDING, CELL_PADDING])
        .into()
}

/// Returns the columns that a list `room` wide shows, each with its width:
/// every column, unless that leaves a name less room than [`NAME_ROOM`]
/// columns of the font, when the yielding columns give way in turn, those
/// without text in any row first, as they tell nothing
fn fitting(app: &App, room: f32) -> Vec<(Column, Option<f32>)> {
    let list = app.current();
    let all: Vec<(Column, Option<f32>)> = Column::VARIANTS
        .iter()
        .map(|&column| (column, column.width(&app.workspace, list)))
        .collect();
    let yielding: Vec<Column> = Column::YIELDING
        .into_iter()
        .sorted_by_cached_key(|column| column.holds_text(&app.workspace, list))
        .collect();
    let name_room = |columns: &[(Column, Option<f32>)]| {
        room - columns
            .iter()
            .map(|(_, width)| width.unwrap_or(0.0) + CELL_SPACE)
            .sum::<f32>()
    };
    giving_way(
        &yielding,
        |gone| {
            all.iter()
                .copied()
                .filter(|(column, _)| !gone.contains(column))
                .collect()
        },
        |columns: &Vec<_>| name_room(columns) >= NAME_ROOM as f32 * char_width(),
    )
}

/// Returns `column` at `width`, or taking the room left without one, under
/// its title, or with no header for the rows under the header
fn table_column<'a>(
    app: &'a App,
    column: Column,
    width: Option<f32>,
    titled: bool,
) -> table::Column<'a, 'a, &'a Cue, Message> {
    let header: Element<'a, Message> = if titled {
        text(column.title()).into()
    } else {
        space().into()
    };
    table::column(header, move |cue| cell(app, column, cue))
        .width(width.map_or(Fill, Length::Fixed))
}

/// Returns the cell of `cue` in `column`, padded above and below, which
/// selects the cue when clicked anywhere
fn cell<'a>(app: &'a App, column: Column, cue: &'a Cue) -> Element<'a, Message> {
    let list = app.current();
    let content: Element<'a, Message> = match column {
        Column::Number => radio(cue.number.clone(), cue.id, app.selected, Message::Select).into(),
        Column::Target => text(column.text(&app.workspace, list, cue))
            .wrapping(Wrapping::WordOrGlyph)
            .into(),
        Column::State => {
            let (state, style) = state(&app.workspace, list, cue);
            text(state).style(style).into()
        }
        _ => text(column.text(&app.workspace, list, cue)).into(),
    };
    let cell = container(content)
        .width(Fill)
        .padding(padding::vertical(ROW_PADDING));
    mouse_area(cell).on_press(Message::Select(cue.id)).into()
}

/// Describes a cue in the State column, in the color of what it needs: red
/// while it cannot play, yellow while it is paused, green while it runs,
/// and blue while it waits on standby for GO.  A cue that cannot play names
/// what it lacks, a file or a target, or else is BROKEN, as when its file
/// cannot be played or its start is not before its end.  A file that is
/// still loading stops the cue from playing too, but it needs nothing but
/// time.  The list of running cues tells more of each, such as where a
/// paused cue stopped.
fn state(workspace: &Workspace, list: &CueList, cue: &Cue) -> (String, fn(&Theme) -> text::Style) {
    match (workspace.problem(cue), workspace.active_of(cue.id)) {
        (Some(Problem::NotLoaded), _) => ("loading".to_owned(), text::default),
        (Some(Problem::NoFile), _) => ("NO FILE".to_owned(), text::danger),
        (Some(Problem::NoTarget), _) => ("NO TARGET".to_owned(), text::danger),
        (Some(Problem::LoadFailed(_) | Problem::EmptyClip), _) => {
            ("BROKEN".to_owned(), text::danger)
        }
        (None, Some(active)) if active.paused => ("paused".to_owned(), text::warning),
        (None, Some(active)) => (format::phase(active), text::success),
        (None, None) if list.standby == Some(cue.id) => ("NEXT".to_owned(), text::primary),
        (None, None) => (String::new(), text::default),
    }
}
