// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Table that marks one of its rows
//!
//! iced draws no mark on the rows of a table.  The cells of a row could
//! each draw a background, but a cell shorter than its row would leave part
//! of the row unmarked, as when a long name wraps onto two lines in the
//! cell beside it.  This table draws the background of the selection under
//! the whole row instead, across every column and from the top of its cells
//! to the bottom of the tallest.  It reports the area of the row under an
//! identity as well, through which an operation finds the row, as scrolling
//! it into view does.

use iced::{
    Element, Rectangle, Renderer,
    advanced::{
        layout::Layout,
        renderer::{self, Renderer as _},
        widget::{Id, Operation},
    },
};

use super::wrap::{Intercept, Wrap};
use crate::front::{Message, view::theme};

/// Returns `table`, which has `columns` columns, with the row at index
/// `row` below its header drawn on the background of the selection and
/// reported under the identity `id`, or with no row marked without one
pub fn marked_row<'a>(
    table: impl Into<Element<'a, Message>>,
    columns: usize,
    row: Option<usize>,
    id: &'static str,
) -> Element<'a, Message> {
    Wrap {
        content: table.into(),
        intercept: Mark { columns, row, id },
    }
    .into()
}

/// Background under one row of a table, and the identity of the row
struct Mark {
    columns: usize,
    row: Option<usize>,
    id: &'static str,
}

impl Mark {
    /// Returns the area of the marked row: across the whole table, and from
    /// the top of its cells to the bottom of the tallest
    fn bounds(&self, layout: Layout<'_>) -> Option<Rectangle> {
        let row = self.row?;
        // iced lays out the cells of a table row by row, header first
        let cells = layout
            .children()
            .skip((row + 1) * self.columns)
            .take(self.columns)
            .map(|cell| cell.bounds())
            .reduce(|cells, cell| cells.union(&cell))?;
        let table = layout.bounds();
        Some(Rectangle {
            x: table.x,
            width: table.width,
            ..cells
        })
    }
}

impl Intercept for Mark {
    fn under(&self, layout: Layout<'_>, renderer: &mut Renderer) {
        if let Some(bounds) = self.bounds(layout) {
            renderer.fill_quad(
                renderer::Quad {
                    bounds,
                    ..renderer::Quad::default()
                },
                theme::SELECTION,
            );
        }
    }

    fn operate(&self, layout: Layout<'_>, operation: &mut dyn Operation) {
        if let Some(bounds) = self.bounds(layout) {
            operation.container(Some(&Id::new(self.id)), bounds);
        }
    }
}
