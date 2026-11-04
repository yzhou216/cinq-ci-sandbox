// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Geometry that the shapes of the logo are declared with: arches, points
//! and arcs of circles, scallops, and the elements of paths

use std::f64::consts::PI;

use kurbo::{Arc, Circle, PathEl, Point, QuadBez, Vec2};

/// Returns the curve from `from` to `to` that passes `middle` down the
/// grid halfway along, arching up or sagging down
pub(super) fn arch(from: (f64, f64), middle: f64, to: (f64, f64)) -> QuadBez {
    let (from, to) = (Point::from(from), Point::from(to));
    let control = Point::new((from.x + to.x) / 2.0, 2.0 * middle - (from.y + to.y) / 2.0);
    QuadBez::new(from, control, to)
}

/// Returns the point of `circle` at `angle`
pub(super) fn at(circle: Circle, angle: f64) -> Point {
    circle.center + Vec2::from_angle(angle.to_radians()) * circle.radius
}

/// Returns the arc of `circle` clockwise from the angle `from` to the angle
/// `to`
pub(super) fn around(circle: Circle, from: f64, to: f64) -> Arc {
    Arc::new(
        circle.center,
        (circle.radius, circle.radius),
        from.to_radians(),
        (to - from).to_radians(),
        0.0,
    )
}

/// Returns half an ellipse `depth` deep from `from` to `to`, which hangs
/// below the line between them when it runs from right to left
pub(super) fn scallop(from: Point, to: Point, depth: f64) -> Arc {
    let chord = to - from;
    Arc::new(
        from.midpoint(to),
        (chord.hypot() / 2.0, depth),
        PI,
        PI,
        chord.atan2(),
    )
}

/// Returns the element of a path that starts a subpath at `start`
pub(super) fn move_to(start: impl Into<Point>) -> PathEl {
    PathEl::MoveTo(start.into())
}

/// Returns the element of a path that draws a line to `end`
pub(super) fn line_to(end: impl Into<Point>) -> PathEl {
    PathEl::LineTo(end.into())
}

/// Returns the element of a path that draws a cubic Bézier curve to `end`,
/// which bends toward the control points `first` and `second`
pub(super) fn curve_to<P: Into<Point>>(first: P, second: P, end: P) -> PathEl {
    PathEl::CurveTo(first.into(), second.into(), end.into())
}
