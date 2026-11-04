// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Layers of the logo, each a shape and its paint, which the SVG document
//! holds as one path apiece

use std::fmt::{self, Display};

use kurbo::{Affine, BezPath};

use super::SIZE;

/// Shape painted onto the logo
#[derive(Debug, Clone)]
pub(super) struct Layer {
    outline: BezPath,
    paint: Paint,
    opacity: f64,
}

/// How a shape is painted: filled with a color, or stroked in a color at a
/// width, with round ends and corners
#[derive(Debug, Clone, Copy)]
enum Paint {
    Fill(&'static str),
    Stroke(&'static str, f64),
}

impl Layer {
    pub(super) fn fill(outline: BezPath, color: &'static str) -> Self {
        Self {
            outline,
            paint: Paint::Fill(color),
            opacity: 1.0,
        }
    }

    pub(super) fn stroke(outline: BezPath, color: &'static str, width: f64) -> Self {
        Self {
            paint: Paint::Stroke(color, width),
            ..Self::fill(outline, color)
        }
    }

    pub(super) fn opacity(self, opacity: f64) -> Self {
        Self { opacity, ..self }
    }

    /// Returns the layer mirrored from left to right across the middle of
    /// the grid
    pub(super) fn mirrored(self) -> Self {
        let mirror = Affine::translate((SIZE, 0.0)) * Affine::FLIP_X;
        Self {
            outline: mirror * self.outline,
            ..self
        }
    }
}

impl Display for Layer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let paint = match self.paint {
            Paint::Fill(color) => format!("fill=\"{color}\""),
            Paint::Stroke(color, width) => format!(
                "fill=\"none\" stroke=\"{color}\" stroke-width=\"{}\" \
                 stroke-linecap=\"round\" stroke-linejoin=\"round\"",
                rounded(width)
            ),
        };
        let opacity = if self.opacity < 1.0 {
            format!(" opacity=\"{}\"", self.opacity)
        } else {
            String::new()
        };
        writeln!(
            f,
            "  <path d=\"{}\" {paint}{opacity}/>",
            self.outline.to_svg()
        )
    }
}

/// Returns `value` to two decimal places, as SVG gets the numbers that come
/// out of arithmetic, such as 2.4 rather than 2.4000000000000004
pub(super) fn rounded(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}
