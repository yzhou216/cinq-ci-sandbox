// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Logo of cinq, French for five: a 5 center stage under a spotlight,
//! between curtains, with the name on the front of the stage
//!
//! The logo is declared here shape by shape, from the back of the stage to
//! the front, on a square grid [`SIZE`] units across whose y axis points
//! down, as that of SVG does.  Angles are in degrees, clockwise from the
//! right.  kurbo does the geometry, and [`svg`] writes the shapes as an SVG
//! document, which the build script draws as the icon of the desktop entry.
//! The program itself leaves this module out: only the build script and the
//! tests compile it.

#[cfg(test)]
mod tests;

use std::{
    f64::consts::PI,
    fmt::{self, Display},
};

use kurbo::{Affine, Arc, BezPath, Circle, Ellipse, ParamCurve, Point, QuadBez, Shape, Vec2};

/// Width and height of the grid that the logo is drawn on
const SIZE: f64 = 128.0;

/// Farthest that the Bézier curves standing for arcs and ellipses stray
/// from them, far finer than the logo is ever drawn
const TOLERANCE: f64 = 0.01;

/// Stage, and the name
const PLUM: &str = "#221c2b";
/// Curtains
const RED: &str = "#b3202e";
/// Folds of the curtains
const DARK_RED: &str = "#8e1622";
/// Valance, cords, front of the stage, and light
const GOLD: &str = "#e2b33c";
/// The 5
const CREAM: &str = "#f4e6c8";

/// Width of the strokes of the name, at the size that it is lettered at
const PEN: f64 = 3.0;

/// Gaps between the ink of neighbouring letters of the name: c to i, i to
/// n, and n to q.  They differ so that the letters look evenly spaced: the
/// open side of the c adds space of its own, two straight stems need the
/// most, and a round bowl needs less.
const GAPS: [f64; 3] = [2.8, 4.0, 3.4];

/// Size that the name is set at, of the size that it is lettered at
const NAME_SCALE: f64 = 0.8;

/// Returns the logo as an SVG document
pub fn svg() -> String {
    let left = [curtain(), folds(), cord()];
    let right = left.clone().map(Layer::mirrored);
    let layers: String = [stage(), beam(), pool(), five()]
        .into_iter()
        .chain(left)
        .chain(right)
        .chain([valance(), stage_front()])
        .chain(name())
        .map(|layer| layer.to_string())
        .collect();
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {SIZE} {SIZE}\">\n{layers}</svg>\n"
    )
}

/// Stage behind the curtains, under the arch of the valance
fn stage() -> Layer {
    let mut stage = top_arch().to_path(TOLERANCE);
    stage.line_to((116.0, 110.0));
    stage.line_to((12.0, 110.0));
    stage.close_path();
    Layer::fill(stage, PLUM)
}

/// Beam of the spotlight, from the valance down to the floor
fn beam() -> Layer {
    let mut beam = BezPath::new();
    beam.move_to((57.0, 28.0));
    beam.line_to((71.0, 28.0));
    beam.line_to((92.0, 104.0));
    beam.line_to((36.0, 104.0));
    beam.close_path();
    Layer::fill(beam, GOLD).opacity(0.14)
}

/// Pool of light that the spotlight casts on the floor
fn pool() -> Layer {
    let pool = Ellipse::new((64.0, 101.0), (26.0, 5.0), 0.0);
    Layer::fill(pool.to_path(TOLERANCE), GOLD).opacity(0.3)
}

/// The 5, center stage: a top bar, a stem, and a round bowl, drawn as one
/// stroke
fn five() -> Layer {
    let bowl = Circle::new((63.5, 76.0), 14.0);
    let mut five = BezPath::new();
    five.move_to((76.5, 46.0));
    five.line_to((51.5, 46.0));
    five.line_to(at(bowl, -160.0));
    five.extend(around(bowl, -160.0, 145.0).append_iter(TOLERANCE));
    Layer::stroke(five, CREAM, 9.0)
}

/// Left curtain, drawn back by its cord and flaring toward the floor,
/// which the right one mirrors
fn curtain() -> Layer {
    let mut curtain = BezPath::new();
    curtain.move_to((14.0, 38.0));
    curtain.line_to((40.0, 38.0));
    // Inner edge, sweeping in to the cord, then falling to the floor
    curtain.curve_to((36.0, 58.0), (28.0, 72.0), (22.0, 82.0));
    curtain.curve_to((26.0, 92.0), (30.0, 102.0), (33.0, 110.0));
    curtain.line_to((5.0, 110.0));
    // Outer edge
    curtain.curve_to((9.0, 86.0), (12.0, 60.0), (14.0, 38.0));
    curtain.close_path();
    Layer::fill(curtain, RED)
}

/// Folds of the left curtain, which gather at its cord
fn folds() -> Layer {
    let mut folds = BezPath::new();
    folds.move_to((31.0, 42.0));
    folds.curve_to((28.0, 58.0), (22.0, 71.0), (16.0, 81.0));
    folds.move_to((16.0, 85.0));
    folds.curve_to((19.0, 94.0), (21.0, 102.0), (23.0, 109.0));
    Layer::stroke(folds, DARK_RED, 2.5)
}

/// Cord that ties the left curtain back
fn cord() -> Layer {
    let cord = QuadBez::new((9.5, 80.0), (16.0, 86.5), (23.0, 82.0));
    Layer::stroke(cord.to_path(TOLERANCE), GOLD, 3.4)
}

/// Valance across the top of the stage: a band that follows the arch, with
/// its lower edge scalloped
fn valance() -> Layer {
    let top = top_arch();
    let lower = Affine::translate((0.0, 14.0)) * top;
    // Ends of the scallops, five of them, from the right of the lower edge
    // to its left
    let ends: Vec<Point> = (0..=5)
        .rev()
        .map(|step| lower.eval(f64::from(step) / 5.0))
        .collect();
    let mut valance = top.to_path(TOLERANCE);
    valance.line_to(lower.p2);
    valance.extend(
        ends.windows(2)
            .flat_map(|pair| scallop(pair[0], pair[1], 6.0).append_iter(TOLERANCE)),
    );
    valance.close_path();
    Layer::fill(valance, GOLD)
}

/// Front of the stage, curving toward the audience
fn stage_front() -> Layer {
    let top = arch((3.0, 104.0), 108.0, (125.0, 104.0));
    let bottom = arch((123.0, 114.0), 127.0, (5.0, 114.0));
    let mut front = top.to_path(TOLERANCE);
    front.line_to(bottom.p0);
    front.quad_to(bottom.p1, bottom.p2);
    front.close_path();
    Layer::fill(front, GOLD)
}

/// Name on the front of the stage, lettered in strokes like the 5: c, i,
/// n, and q, each 10 units high and spaced by [`GAPS`], then set at
/// [`NAME_SCALE`] of that size in the middle of the stage
fn name() -> [Layer; 2] {
    let half = PEN / 2.0;
    let c = Circle::new((5.0, 5.0), 5.0);
    let i = at(c, 40.0).x + half + GAPS[0] + half;
    let n = i + half + GAPS[1] + half;
    let q = Circle::new((n + 9.0 + half + GAPS[2] + half + 5.0, 5.0), 5.0);
    let tail = q.center.x + 5.0;
    let mut letters = around(c, 40.0, 320.0).to_path(TOLERANCE);
    letters.move_to((i, 0.0));
    letters.line_to((i, 10.0));
    letters.move_to((n, 10.0));
    letters.line_to((n, 0.0));
    letters.move_to((n, 4.5));
    letters.curve_to((n, 1.6), (n + 2.0, 0.0), (n + 4.5, 0.0));
    letters.curve_to((n + 7.0, 0.0), (n + 9.0, 1.6), (n + 9.0, 4.5));
    letters.line_to((n + 9.0, 10.0));
    letters.extend(q.path_elements(TOLERANCE));
    letters.move_to((tail, 0.0));
    letters.line_to((tail, 13.0));
    let dot = Circle::new((i, -4.4), 1.7);
    // Ink runs from half a pen left of the c to half a pen right of the
    // tail of the q, so its middle is halfway to the tail
    let place = Affine::translate((SIZE / 2.0 - NAME_SCALE * tail / 2.0, 114.0))
        * Affine::scale(NAME_SCALE);
    [
        Layer::stroke(place * letters, PLUM, PEN * NAME_SCALE),
        Layer::fill(place * dot.to_path(TOLERANCE), PLUM),
    ]
}

/// Arch that the valance and the stage under it follow
fn top_arch() -> QuadBez {
    arch((14.0, 30.0), 12.0, (114.0, 30.0))
}

/// Returns the curve from `from` to `to` that passes `middle` down the
/// grid halfway along, arching up or sagging down
fn arch(from: (f64, f64), middle: f64, to: (f64, f64)) -> QuadBez {
    let (from, to) = (Point::from(from), Point::from(to));
    let control = Point::new((from.x + to.x) / 2.0, 2.0 * middle - (from.y + to.y) / 2.0);
    QuadBez::new(from, control, to)
}

/// Returns the point of `circle` at `angle`
fn at(circle: Circle, angle: f64) -> Point {
    circle.center + Vec2::from_angle(angle.to_radians()) * circle.radius
}

/// Returns the arc of `circle` clockwise from the angle `from` to the angle
/// `to`
fn around(circle: Circle, from: f64, to: f64) -> Arc {
    let radii = (circle.radius, circle.radius);
    Arc::new(
        circle.center,
        radii,
        from.to_radians(),
        (to - from).to_radians(),
        0.0,
    )
}

/// Returns half an ellipse `depth` deep from `from` to `to`, which hangs
/// below the line between them when it runs from right to left
fn scallop(from: Point, to: Point, depth: f64) -> Arc {
    let chord = to - from;
    Arc::new(
        from.midpoint(to),
        (chord.hypot() / 2.0, depth),
        PI,
        PI,
        chord.atan2(),
    )
}

/// Shape painted onto the logo
#[derive(Debug, Clone)]
struct Layer {
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
    fn fill(outline: BezPath, color: &'static str) -> Self {
        Self {
            outline,
            paint: Paint::Fill(color),
            opacity: 1.0,
        }
    }

    fn stroke(outline: BezPath, color: &'static str, width: f64) -> Self {
        Self {
            paint: Paint::Stroke(color, width),
            ..Self::fill(outline, color)
        }
    }

    fn opacity(self, opacity: f64) -> Self {
        Self { opacity, ..self }
    }

    /// Returns the layer mirrored from left to right across the middle of
    /// the grid
    fn mirrored(self) -> Self {
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
fn rounded(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}
