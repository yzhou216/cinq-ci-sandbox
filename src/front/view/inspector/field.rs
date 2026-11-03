// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Fields of the inspector, each a labelled input for one setting

use iced::{
    Element,
    widget::{row, text_input},
};
use strum::{IntoStaticStr, VariantArray};

use crate::{
    back::{Advance, Cue, Edit, Kind},
    front::{
        App, Message,
        view::{NAME, SPACING, button, dropdown, format, theme},
    },
};

/// Inspector field, one per row of a form, which goes by its name, apart
/// from the target of an audio cue, which is its file
#[derive(Debug, Clone, Copy, IntoStaticStr)]
pub(super) enum Field {
    Number,
    Name,
    Target,
    Notes,
    #[strum(serialize = "Pre-wait")]
    PreWait,
    Duration,
    #[strum(serialize = "Post-wait")]
    PostWait,
    Then,
    Start,
    End,
}

impl Field {
    pub(super) fn label(self, cue: &Cue) -> &'static str {
        match self {
            Field::Target if cue.kind == Kind::Audio => "File",
            _ => self.into(),
        }
    }

    /// Returns the input for this field.  It is disabled when editing is
    /// locked or when the value does not apply to the cue, such as the
    /// post-wait of an auto-follow cue.  Values that come from elsewhere,
    /// such as how long an audio cue plays, show in a disabled field too.
    /// The sound file of an audio cue can be typed, or chosen in the
    /// desktop's file chooser, and a time that reads as no time stands out.
    pub(super) fn input<'a>(self, app: &'a App, cue: &'a Cue) -> Element<'a, Message> {
        let id = cue.id;
        let editable = app.editable();
        let field =
            |value: &str, applies: bool, edit: fn(String) -> Edit| -> Element<'a, Message> {
                let input = text_input(self.placeholder(cue), value)
                    .on_input_maybe(
                        (editable && applies).then_some(move |text| Message::Edit(id, edit(text))),
                    )
                    .style(theme::field(self.valid(cue)));
                match self {
                    Field::Name => input.id(NAME).into(),
                    _ => input.into(),
                }
            };
        match self {
            Field::Number => field(&cue.number, true, Edit::Number),
            Field::Name => field(&cue.name, true, Edit::Name),
            Field::Target => match cue.kind {
                Kind::Audio => row![
                    field(&cue.file, true, Edit::File),
                    button("Choose").on_press_maybe(editable.then_some(Message::ChooseSound(id))),
                ]
                .spacing(SPACING)
                .into(),
                Kind::Fade | Kind::Stop if editable => target_menu(app, cue),
                Kind::Fade | Kind::Stop => shown(
                    &app.workspace
                        .target(cue)
                        .map_or_else(String::new, Cue::to_string),
                ),
                Kind::Wait | Kind::Memo => shown(""),
            },
            Field::Notes => field(&cue.notes, true, Edit::Notes),
            Field::PreWait => field(cue.pre_wait.text(), true, |text| Edit::PreWait(text.into())),
            Field::Duration if cue.kind.has_duration() => {
                field(cue.duration.text(), true, |text| {
                    Edit::Duration(text.into())
                })
            }
            Field::Duration => shown(&format::action(cue)),
            Field::PostWait => field(
                cue.post_wait.text(),
                cue.advance != Advance::Follow,
                |text| Edit::PostWait(text.into()),
            ),
            Field::Then if editable => dropdown(
                Advance::VARIANTS,
                Some(cue.advance),
                move |advance| Message::Edit(id, Edit::Advance(advance)),
                &app.config.keymap,
            )
            .into(),
            Field::Then => shown(&cue.advance.to_string()),
            Field::Start => field(cue.start.text(), true, |text| Edit::Start(text.into())),
            Field::End => field(cue.end.text(), true, |text| Edit::End(text.into())),
        }
    }

    /// Returns whether the text of the field reads as a value, which only
    /// the text of a time can fail to
    fn valid(self, cue: &Cue) -> bool {
        match self {
            Field::PreWait => cue.pre_wait.is_valid(),
            Field::Duration => cue.duration.is_valid(),
            Field::PostWait => cue.post_wait.is_valid(),
            Field::Start => cue.start.is_valid(),
            Field::End => cue.end.is_valid(),
            _ => true,
        }
    }

    /// Returns the hint an empty field shows
    fn placeholder(self, cue: &Cue) -> &'static str {
        match self {
            Field::Target if cue.kind == Kind::Audio => "path of a sound file",
            Field::PreWait | Field::Duration | Field::PostWait | Field::Start => "0",
            Field::End => "end of file",
            _ => "",
        }
    }
}

/// Shows a value in a disabled field, so it lines up with editable ones
fn shown<'a>(value: &str) -> Element<'a, Message> {
    text_input("", value).into()
}

/// Menu of the cues that a fade or stop cue can act on, each shown by its
/// number and name
fn target_menu<'a>(app: &'a App, cue: &'a Cue) -> Element<'a, Message> {
    let id = cue.id;
    let targets: Vec<&Cue> = app
        .workspace
        .cues()
        .filter(|other| cue.can_target(other))
        .collect();
    let selected = targets
        .iter()
        .find(|target| Some(target.id) == cue.target)
        .copied();
    dropdown(
        targets,
        selected,
        move |target: &Cue| Message::Edit(id, Edit::Target(Some(target.id))),
        &app.config.keymap,
    )
    .placeholder("choose a cue")
    .into()
}
