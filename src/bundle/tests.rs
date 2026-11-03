// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the app bundle: the version that Info.plist gives, and the icon

use super::*;

#[test]
fn info_plist_gives_the_version_of_the_package() {
    let version = concat!("<string>", env!("CARGO_PKG_VERSION"), "</string>");
    assert_eq!(info_plist().matches(version).count(), 2);
}

#[test]
fn icon_holds_the_logo_as_each_kind_of_image() {
    let icon = IconFamily::read(Cursor::new(icns().expect("the logo draws")))
        .expect("the icon reads back");
    assert_eq!(icon.available_icons(), ICONS);
}
