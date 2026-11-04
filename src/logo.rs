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
//! [`geometry`] holds the helpers that the shapes are declared with, and
//! [`layer`] paints each shape as a path of the document.  The program
//! itself leaves this module out: only the build script and the tests
//! compile it.

mod geometry;
mod layer;
#[cfg(test)]
mod tests;

use kurbo::{Affine, BezPath, Circle, Ellipse, ParamCurve, PathEl, Point, QuadBez, Shape};

use geometry::{arch, around, at, curve_to, line_to, move_to, scallop};
use layer::Layer;

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
/// Digit 5
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
    let stage = top_arch().path_elements(TOLERANCE).chain([
        line_to((116.0, 110.0)),
        line_to((12.0, 110.0)),
        PathEl::ClosePath,
    ]);
    Layer::fill(stage.collect(), PLUM)
}

/// Beam of the spotlight, from the valance down to the floor
fn beam() -> Layer {
    let beam = [
        move_to((57.0, 28.0)),
        line_to((71.0, 28.0)),
        line_to((92.0, 104.0)),
        line_to((36.0, 104.0)),
        PathEl::ClosePath,
    ];
    Layer::fill(beam.into_iter().collect(), GOLD).opacity(0.14)
}

/// Pool of light that the spotlight casts on the floor
fn pool() -> Layer {
    let pool = Ellipse::new((64.0, 101.0), (26.0, 5.0), 0.0);
    Layer::fill(pool.to_path(TOLERANCE), GOLD).opacity(0.3)
}

/// Digit 5, center stage: a top bar, a stem, and a round bowl, drawn as one
/// stroke
fn five() -> Layer {
    let bowl = Circle::new((63.5, 76.0), 14.0);
    let five = [
        move_to((76.5, 46.0)),
        line_to((51.5, 46.0)),
        line_to(at(bowl, -160.0)),
    ]
    .into_iter()
    .chain(around(bowl, -160.0, 145.0).append_iter(TOLERANCE));
    Layer::stroke(five.collect(), CREAM, 9.0)
}

/// Left curtain, drawn back by its cord and flaring toward the floor,
/// which the right one mirrors
fn curtain() -> Layer {
    let curtain = [
        move_to((14.0, 38.0)),
        line_to((40.0, 38.0)),
        // Inner edge, sweeping in to the cord, then falling to the floor
        curve_to((36.0, 58.0), (28.0, 72.0), (22.0, 82.0)),
        curve_to((26.0, 92.0), (30.0, 102.0), (33.0, 110.0)),
        line_to((5.0, 110.0)),
        // Outer edge
        curve_to((9.0, 86.0), (12.0, 60.0), (14.0, 38.0)),
        PathEl::ClosePath,
    ];
    Layer::fill(curtain.into_iter().collect(), RED)
}

/// Folds of the left curtain, which gather at its cord
fn folds() -> Layer {
    let folds = [
        move_to((31.0, 42.0)),
        curve_to((28.0, 58.0), (22.0, 71.0), (16.0, 81.0)),
        move_to((16.0, 85.0)),
        curve_to((19.0, 94.0), (21.0, 102.0), (23.0, 109.0)),
    ];
    Layer::stroke(folds.into_iter().collect(), DARK_RED, 2.5)
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
    let scallops = ends
        .windows(2)
        .flat_map(|pair| scallop(pair[0], pair[1], 6.0).append_iter(TOLERANCE));
    let valance = top
        .path_elements(TOLERANCE)
        .chain([line_to(lower.p2)])
        .chain(scallops)
        .chain([PathEl::ClosePath]);
    Layer::fill(valance.collect(), GOLD)
}

/// Front of the stage, curving toward the audience
fn stage_front() -> Layer {
    let top = arch((3.0, 104.0), 108.0, (125.0, 104.0));
    let bottom = arch((123.0, 114.0), 127.0, (5.0, 114.0));
    let front = top.path_elements(TOLERANCE).chain([
        line_to(bottom.p0),
        PathEl::QuadTo(bottom.p1, bottom.p2),
        PathEl::ClosePath,
    ]);
    Layer::fill(front.collect(), GOLD)
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
    let letters: BezPath = around(c, 40.0, 320.0)
        .path_elements(TOLERANCE)
        .chain([
            move_to((i, 0.0)),
            line_to((i, 10.0)),
            move_to((n, 10.0)),
            line_to((n, 0.0)),
            move_to((n, 4.5)),
            curve_to((n, 1.6), (n + 2.0, 0.0), (n + 4.5, 0.0)),
            curve_to((n + 7.0, 0.0), (n + 9.0, 1.6), (n + 9.0, 4.5)),
            line_to((n + 9.0, 10.0)),
        ])
        .chain(q.path_elements(TOLERANCE))
        .chain([move_to((tail, 0.0)), line_to((tail, 13.0))])
        .collect();
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
