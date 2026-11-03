// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! App bundle of cinq for macOS, which the build script lays out for builds
//! for macOS, with all but the program in it
//!
//! A bundle is a directory, cinq.app, that macOS shows and opens as one
//! app.  Its Info.plist tells macOS the name, identifier, and version of the
//! app, and its icon is the logo drawn at each size that macOS shows icons
//! at.  The program goes in `Contents/MacOS`, which the build leaves for a
//! package to fill, since a build script runs before the program is linked.
//! The program itself leaves this module out: only the build script and the
//! tests compile it.

#[cfg(test)]
mod tests;

use std::{
    fmt::{self, Display},
    io,
};

use icns::{IconFamily, IconType, Image, PixelFormat};
use resvg::{
    tiny_skia::{Pixmap, Transform},
    usvg::{Options, Tree},
};

use super::logo;

/// Identifier of the app, in reverse DNS form
const IDENTIFIER: &str = "io.yiyuzhou.cinq";

/// Notice that Finder shows with the app, as `--version` prints it
const COPYRIGHT: &str = "Copyright (C) 2026 Yiyu Zhou";

/// Sizes that macOS shows icons at, in points, each of which the icon holds
/// at one and at two pixels to the point
const SIZES: [u32; 5] = [16, 32, 128, 256, 512];

/// Value of a key in Info.plist, whose text needs no escaping in XML
#[derive(Debug, Clone, Copy)]
enum Value<'a> {
    Text(&'a str),
    True,
}

impl Display for Value<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text(text) => write!(f, "<string>{text}</string>"),
            Self::True => write!(f, "<true/>"),
        }
    }
}

/// Returns Info.plist, which tells macOS what the app is, giving `minimum`
/// as the oldest release of macOS that it runs on when the build names one
pub fn info_plist(minimum: Option<&str>) -> String {
    let name = env!("CARGO_PKG_NAME");
    let version = env!("CARGO_PKG_VERSION");
    let keys: String = [
        ("CFBundleExecutable", Some(Value::Text(name))),
        ("CFBundleIconFile", Some(Value::Text(name))),
        ("CFBundleIdentifier", Some(Value::Text(IDENTIFIER))),
        ("CFBundleInfoDictionaryVersion", Some(Value::Text("6.0"))),
        ("CFBundleName", Some(Value::Text(name))),
        ("CFBundlePackageType", Some(Value::Text("APPL"))),
        ("CFBundleShortVersionString", Some(Value::Text(version))),
        ("CFBundleVersion", Some(Value::Text(version))),
        ("LSMinimumSystemVersion", minimum.map(Value::Text)),
        ("NSHighResolutionCapable", Some(Value::True)),
        ("NSHumanReadableCopyright", Some(Value::Text(COPYRIGHT))),
    ]
    .into_iter()
    .filter_map(|(key, value)| value.map(|value| format!("\t<key>{key}</key>\n\t{value}\n")))
    .collect();
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \
         \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\">\n<dict>\n{keys}</dict>\n</plist>\n"
    )
}

/// Returns the icon of the app as an icns file: the logo drawn at each of
/// [`SIZES`], at one and at two pixels to the point
pub fn icns() -> io::Result<Vec<u8>> {
    let logo = Tree::from_str(&logo::svg(), &Options::default()).map_err(io::Error::other)?;
    let icon = SIZES
        .into_iter()
        .flat_map(|points| [1, 2].map(|density| (points * density, density)))
        .try_fold(IconFamily::new(), |mut icon, (pixels, density)| {
            let kind = IconType::from_pixel_size_and_density(pixels, pixels, density)
                .expect("macOS has an icon of each size");
            icon.add_icon_with_type(&drawn(&logo, pixels)?, kind)?;
            Ok::<_, io::Error>(icon)
        })?;
    let mut file = Vec::new();
    icon.write(&mut file)?;
    Ok(file)
}

/// Returns `logo` drawn `pixels` across, as an image for the icon
fn drawn(logo: &Tree, pixels: u32) -> io::Result<Image> {
    let mut pixmap = Pixmap::new(pixels, pixels).expect("the icon has no empty size");
    let scale = pixels as f32 / logo.size().width();
    resvg::render(
        logo,
        Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    // Colors, which tiny-skia keeps multiplied by their alpha and icns takes
    // as they are
    let rgba = pixmap
        .pixels()
        .iter()
        .flat_map(|pixel| {
            let color = pixel.demultiply();
            [color.red(), color.green(), color.blue(), color.alpha()]
        })
        .collect();
    Image::from_data(PixelFormat::RGBA, pixels, pixels, rgba)
}
