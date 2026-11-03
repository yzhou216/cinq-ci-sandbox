// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! App bundle for Darwin, which the build script lays out with all but the
//! program in it
//!
//! A bundle is a directory, cinq.app, that Finder shows and opens as one
//! program.  Its Info.plist names the program, and its icon is the logo
//! drawn at each size that Darwin shows icons at.  The program goes in
//! `Contents/MacOS`, which a package fills, since the build script runs
//! before the program is linked.  The program itself leaves this module out:
//! only the build script and the tests compile it.

#[cfg(test)]
mod tests;

use std::io::{self, Cursor};

use icns::{IconFamily, IconType, Image};
use resvg::{
    tiny_skia::{Pixmap, Transform},
    usvg::{Options, Tree},
};

use super::logo;

/// Kinds of image in the icon, one for each size that Darwin shows icons at,
/// at one and at two pixels to the point
const ICONS: [IconType; 10] = [
    IconType::RGB24_16x16,
    IconType::RGBA32_16x16_2x,
    IconType::RGB24_32x32,
    IconType::RGBA32_32x32_2x,
    IconType::RGB24_128x128,
    IconType::RGBA32_128x128_2x,
    IconType::RGBA32_256x256,
    IconType::RGBA32_256x256_2x,
    IconType::RGBA32_512x512,
    IconType::RGBA32_512x512_2x,
];

/// Returns Info.plist, which tells Darwin the name, identifier, and version
/// of cinq
pub fn info_plist() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key>
  <string>cinq</string>
  <key>CFBundleIconFile</key>
  <string>cinq</string>
  <key>CFBundleIdentifier</key>
  <string>io.yiyuzhou.cinq</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>CFBundleName</key>
  <string>cinq</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>{version}</string>
  <key>CFBundleVersion</key>
  <string>{version}</string>
  <key>NSHighResolutionCapable</key>
  <true/>
  <key>NSHumanReadableCopyright</key>
  <string>Copyright (C) 2026 Yiyu Zhou</string>
</dict>
</plist>
"#,
        version = env!("CARGO_PKG_VERSION")
    )
}

/// Returns the icon as an icns file, which holds the logo as each of
/// [`ICONS`]
pub fn icns() -> io::Result<Vec<u8>> {
    let logo = Tree::from_str(&logo::svg(), &Options::default()).map_err(io::Error::other)?;
    let mut icon = IconFamily::new();
    ICONS
        .into_iter()
        .try_for_each(|kind| icon.add_icon_with_type(&drawn(&logo, kind.pixel_width())?, kind))?;
    let mut file = Vec::new();
    icon.write(&mut file)?;
    Ok(file)
}

/// Returns `logo` drawn `pixels` across, as an image for the icon
fn drawn(logo: &Tree, pixels: u32) -> io::Result<Image> {
    let mut pixmap = Pixmap::new(pixels, pixels).expect("no icon is empty");
    let scale = pixels as f32 / logo.size().width();
    resvg::render(
        logo,
        Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    Image::read_png(Cursor::new(pixmap.encode_png().map_err(io::Error::other)?))
}
