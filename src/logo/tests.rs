// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the logo: the geometry that declares it, and the SVG that it is
//! written as

use super::*;

/// Returns whether `point` lies within a hundredth of `x` and `y`
fn near(point: Point, x: f64, y: f64) -> bool {
    (point.x - x).abs() < 0.01 && (point.y - y).abs() < 0.01
}

#[test]
fn angles_go_clockwise_from_the_right_as_the_grid_points_down() {
    let circle = Circle::new((10.0, 10.0), 2.0);
    assert!(near(at(circle, 0.0), 12.0, 10.0));
    assert!(near(at(circle, 90.0), 10.0, 12.0));
    assert!(near(at(circle, -90.0), 10.0, 8.0));
}

#[test]
fn arch_runs_between_its_ends_through_its_middle_height() {
    let arch = arch((14.0, 30.0), 12.0, (114.0, 30.0));
    assert!(near(arch.eval(0.0), 14.0, 30.0));
    assert!(near(arch.eval(0.5), 64.0, 12.0));
    assert!(near(arch.eval(1.0), 114.0, 30.0));
}

#[test]
fn scallop_hangs_below_a_line_that_runs_from_right_to_left() {
    let bounds = scallop(Point::new(20.0, 10.0), Point::new(0.0, 10.0), 6.0)
        .to_path(TOLERANCE)
        .bounding_box();
    assert!((bounds.x0 - 0.0).abs() < 0.01 && (bounds.x1 - 20.0).abs() < 0.01);
    assert!((bounds.y0 - 10.0).abs() < 0.01 && (bounds.y1 - 16.0).abs() < 0.01);
}

#[test]
fn widths_from_arithmetic_are_written_to_two_decimal_places() {
    assert_eq!(rounded(PEN * NAME_SCALE).to_string(), "2.4");
}

#[test]
fn logo_is_one_svg_document_of_paths_on_its_grid() {
    let svg = svg();
    let lines: Vec<&str> = svg.lines().collect();
    assert_eq!(
        lines.first(),
        Some(&r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 128">"#)
    );
    assert_eq!(lines.last(), Some(&"</svg>"));
    assert!(
        lines[1..lines.len() - 1]
            .iter()
            .all(|line| line.starts_with(r#"  <path d="M"#) && line.ends_with("/>")),
        "{svg}"
    );
}
