// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Colors of modus-vivendi, the dark theme of the Modus themes for Emacs
//!
//! Every value below is a named color of modus-vivendi, which keeps the
//! background pure black.  Every color is opaque, and the theme assigns them
//! to the roles that iced's own widget styles draw with, so the widgets keep
//! their default look.  The roles carry the meanings common to interfaces:
//! blue marks what comes next or is chosen, green what runs, yellow what
//! needs a look, and red what failed or stops everything.
//!
//! The exceptions are [`button`], since iced draws disabled buttons half
//! transparent, so buttons get a style of their own, which [`go`] and
//! [`panic`](fn@panic) paint green and red, [`field`], since iced has no
//! style for a field that holds a mistake, and [`LINK`] and [`SELECTION`],
//! since iced has no role for links or for the selected row of a table.

use iced::{
    Color, color,
    theme::{
        Palette, Theme,
        palette::{Background, Danger, Extended, Pair, Primary, Secondary, Success, Warning},
    },
    widget::{
        button::{Status, Style},
        text_input,
    },
};

const BG_MAIN: Color = color!(0x000000);
const BG_DIM: Color = color!(0x1e1e1e);
const BG_INACTIVE: Color = color!(0x303030);
const BG_ACTIVE: Color = color!(0x535353);
const BORDER: Color = color!(0x646464);
const BORDER_MODE_LINE_ACTIVE: Color = color!(0x959595);
const FG_DIM: Color = color!(0x989898);
const FG_MAIN: Color = color!(0xffffff);
const BG_HOVER: Color = color!(0x45605e);
const BG_HL_LINE: Color = color!(0x2f3849);
const BG_COMPLETION: Color = color!(0x2f447f);
const BLUE: Color = color!(0x2fafff);
const BLUE_COOLER: Color = color!(0x00bcff);
const BLUE_WARMER: Color = color!(0x79a8ff);
const GREEN: Color = color!(0x44bc44);
const YELLOW_WARMER: Color = color!(0xfec43f);
const RED: Color = color!(0xff5f59);
const BG_GREEN_SUBTLE: Color = color!(0x00422a);
const BG_GREEN_INTENSE: Color = color!(0x2f822f);
const BG_YELLOW_SUBTLE: Color = color!(0x4a4000);
const BG_YELLOW_INTENSE: Color = color!(0x7a6100);
const BG_RED_SUBTLE: Color = color!(0x620f2a);
const BG_RED_INTENSE: Color = color!(0x9d1f1f);

/// Color of links, which modus-vivendi gives to `fg-link`
pub const LINK: Color = BLUE_WARMER;

/// Background of the selected row of a table, which modus-vivendi gives to
/// `bg-hl-line`, the line of the cursor, and on which every color of text
/// stays legible
pub const SELECTION: Color = BG_HL_LINE;

/// Colors of the roles of iced's palette, which iced's widget styles draw
/// with, and which spans of rich text take as they are
pub const PALETTE: Palette = Palette {
    background: BG_MAIN,
    text: FG_MAIN,
    primary: BLUE,
    success: GREEN,
    warning: YELLOW_WARMER,
    danger: RED,
};

/// Returns the theme of the whole interface
pub fn modus_vivendi() -> Theme {
    Theme::custom_with_fn("Modus Vivendi", PALETTE, |_| extended())
}

/// Assigns colors to the roles of iced's widget styles.  Drop-down lists,
/// read-only fields, menus, and scroll rails sit on the dim background,
/// while borders, rules, and the empty part of bars and sliders use the
/// border gray.  Blue marks what is filled, chosen, or focused.
/// Placeholders and disabled values use the dim foreground, which iced
/// takes from the secondary color.
fn extended() -> Extended {
    let pair = |color, text| Pair { color, text };
    Extended {
        background: Background {
            base: pair(BG_MAIN, FG_MAIN),
            weakest: pair(BG_DIM, FG_MAIN),
            weaker: pair(BG_DIM, FG_MAIN),
            weak: pair(BG_DIM, FG_MAIN),
            neutral: pair(BG_INACTIVE, FG_MAIN),
            strong: pair(BORDER, FG_MAIN),
            stronger: pair(BORDER_MODE_LINE_ACTIVE, BG_MAIN),
            strongest: pair(FG_DIM, BG_MAIN),
        },
        primary: Primary {
            weak: pair(BG_COMPLETION, FG_MAIN),
            base: pair(BLUE, BG_MAIN),
            strong: pair(BLUE_COOLER, BG_MAIN),
        },
        secondary: Secondary {
            weak: pair(FG_DIM, BG_MAIN),
            base: pair(FG_DIM, BG_MAIN),
            strong: pair(FG_DIM, BG_MAIN),
        },
        success: Success {
            weak: pair(BG_GREEN_SUBTLE, FG_MAIN),
            base: pair(BG_GREEN_INTENSE, FG_MAIN),
            strong: pair(BG_GREEN_INTENSE, FG_MAIN),
        },
        warning: Warning {
            weak: pair(BG_YELLOW_SUBTLE, FG_MAIN),
            base: pair(BG_YELLOW_INTENSE, FG_MAIN),
            strong: pair(BG_YELLOW_INTENSE, FG_MAIN),
        },
        danger: Danger {
            weak: pair(BG_RED_SUBTLE, FG_MAIN),
            base: pair(BG_RED_INTENSE, FG_MAIN),
            strong: pair(BG_RED_INTENSE, FG_MAIN),
        },
        is_dark: true,
    }
}

/// Draws a button in modus-vivendi's button colors
pub fn button(theme: &Theme, status: Status) -> Style {
    painted(theme, status, BG_ACTIVE)
}

/// Draws the GO button on modus-vivendi's intense green, the color of going
/// ahead
pub fn go(theme: &Theme, status: Status) -> Style {
    painted(theme, status, BG_GREEN_INTENSE)
}

/// Draws the Panic button on modus-vivendi's intense red, the color of
/// stopping
pub fn panic(theme: &Theme, status: Status) -> Style {
    painted(theme, status, BG_RED_INTENSE)
}

/// Returns the style of a text field: iced's default, or [`invalid`] while
/// its text reads as no valid value
pub fn field(valid: bool) -> fn(&Theme, text_input::Status) -> text_input::Style {
    if valid { text_input::default } else { invalid }
}

/// Draws a text field in iced's default style, with the border in red, the
/// color of failure, so that a mistake shows as it is typed
fn invalid(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let style = text_input::default(theme, status);
    text_input::Style {
        border: style.border.color(RED),
        ..style
    }
}

/// Draws a button on `background`, on the hover background while hovered,
/// and in the dim colors of an inactive button while disabled.  Apart from
/// color it is iced's default style.
fn painted(theme: &Theme, status: Status, background: Color) -> Style {
    let (background, text) = match status {
        Status::Active | Status::Pressed => (background, FG_MAIN),
        Status::Hovered => (BG_HOVER, FG_MAIN),
        Status::Disabled => (BG_DIM, FG_DIM),
    };
    Style {
        background: Some(background.into()),
        text_color: text,
        ..iced::widget::button::primary(theme, Status::Active)
    }
}
