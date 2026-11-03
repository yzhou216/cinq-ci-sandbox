// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the app bundle: what Info.plist tells macOS, and the icon at
//! each size that macOS shows icons at

use std::io::Cursor;

use super::*;

/// Returns the value that `plist` gives `key`, on the line after the key
fn value<'a>(plist: &'a str, key: &str) -> Option<&'a str> {
    let line = format!("\t<key>{key}</key>");
    plist
        .lines()
        .skip_while(|other| *other != line)
        .nth(1)
        .map(str::trim)
}

#[test]
fn info_plist_names_the_program_its_icon_and_its_version() {
    let plist = info_plist(None);
    let name = concat!("<string>", env!("CARGO_PKG_NAME"), "</string>");
    let version = concat!("<string>", env!("CARGO_PKG_VERSION"), "</string>");
    assert_eq!(value(&plist, "CFBundleExecutable"), Some(name));
    assert_eq!(value(&plist, "CFBundleIconFile"), Some(name));
    assert_eq!(
        value(&plist, "CFBundleIdentifier"),
        Some("<string>io.yiyuzhou.cinq</string>")
    );
    assert_eq!(value(&plist, "CFBundleShortVersionString"), Some(version));
    assert_eq!(value(&plist, "CFBundleVersion"), Some(version));
}

#[test]
fn info_plist_gives_the_oldest_macos_only_when_the_build_names_one() {
    assert_eq!(value(&info_plist(None), "LSMinimumSystemVersion"), None);
    assert_eq!(
        value(&info_plist(Some("11.0")), "LSMinimumSystemVersion"),
        Some("<string>11.0</string>")
    );
}

#[test]
fn icon_holds_the_logo_at_each_size_and_density() {
    let icon = IconFamily::read(Cursor::new(icns().expect("the logo draws")))
        .expect("the icon reads back");
    let held = icon.available_icons();
    let missing: Vec<(u32, u32)> = SIZES
        .into_iter()
        .flat_map(|points| [(points, 1), (points * 2, 2)])
        .filter(|&(pixels, density)| {
            IconType::from_pixel_size_and_density(pixels, pixels, density)
                .is_none_or(|kind| !held.contains(&kind))
        })
        .collect();
    assert!(missing.is_empty(), "the icon lacks {missing:?}");
}
