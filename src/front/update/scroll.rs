// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Scrolling the cue list so that the selected cue stays in view, along
//! with a few rows around it
//!
//! The list measures the selected row as it is laid out, so a row that a
//! long name wraps onto two lines scrolls into view as surely as any other.
//! Measuring and scrolling happen in one operation, which iced carries out
//! before it draws the next frame.

use iced::{
    Rectangle, Task, Vector,
    advanced::widget::{
        Id, operate,
        operation::{Operation, Outcome, Scrollable, scrollable},
    },
    widget::operation::{self, AbsoluteOffset, RelativeOffset},
};

use crate::front::{App, Message, reveal, view};

/// Rows that the cue list keeps in view above and below the selected cue,
/// as far as the list has room, so that the cues next to it stay in sight,
/// such as those that come next as GO moves the selection down the list
const MARGIN_ROWS: f32 = 3.0;

impl App {
    /// Returns the state along with the operation that scrolls the selected
    /// cue into view
    pub(super) fn revealed(self) -> (Self, Task<Message>) {
        self.with_task(Self::reveal)
    }

    /// Scrolls the cue list as little as it takes to show the selected cue.
    /// The first cue snaps the list back to its start, as the window opened
    /// it, since the row itself begins a separator below the start.
    fn reveal(&self) -> Task<Message> {
        match self.selected_row() {
            Some(0) => operation::snap_to(view::CUES, RelativeOffset::START),
            Some(_) => operate(Measure::default()).discard(),
            None => Task::none(),
        }
    }
}

/// Operation that finds the selected cue in the cue list, then scrolls the
/// list to show it, if it is out of view
#[derive(Default)]
struct Measure {
    /// Top of the list content, how far the list has scrolled, and the
    /// height of the part in view
    list: Option<(f32, f32, f32)>,
    /// Area of the selected row, which the cue list reports
    row: Option<Rectangle>,
}

impl Operation for Measure {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        id: Option<&Id>,
        bounds: Rectangle,
        content_bounds: Rectangle,
        translation: Vector,
        _state: &mut dyn Scrollable,
    ) {
        if id == Some(&Id::new(view::CUES)) {
            self.list = Some((content_bounds.y, translation.y, bounds.height));
        }
    }

    fn container(&mut self, id: Option<&Id>, bounds: Rectangle) {
        if id == Some(&Id::new(view::SELECTED)) {
            self.row = Some(bounds);
        }
    }

    fn finish(&self) -> Outcome<()> {
        let (Some((origin, scrolled, height)), Some(row)) = (self.list, self.row) else {
            return Outcome::None;
        };
        let margin = (MARGIN_ROWS * row.height).min(((height - row.height) / 2.0).max(0.0));
        let Some(offset) = reveal(
            row.y - origin - margin,
            row.y + row.height - origin + margin,
            scrolled,
            height,
        ) else {
            return Outcome::None;
        };
        let offset = AbsoluteOffset {
            x: None,
            y: Some(offset.max(0.0)),
        };
        Outcome::Chain(Box::new(scrollable::scroll_to(Id::new(view::CUES), offset)))
    }
}
