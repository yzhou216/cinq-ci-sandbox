// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Times that the operator types

#[cfg(test)]
mod tests;

/// Time in seconds as the operator typed it, along with its value
///
/// Keeping the text lets the operator type intermediate input such as `1.`
/// without the field rewriting it.  The value is parsed once, when the text
/// is set, so playback and drawing read it without parsing again.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Seconds {
    text: String,
    /// Time that the text reads as, or none when it reads as no time
    value: Option<f32>,
}

impl Seconds {
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns the time that the text reads as, which is zero for text
    /// that reads as no time
    pub fn value(&self) -> f32 {
        self.value.unwrap_or(0.0)
    }

    /// Returns the time that the text reads as, or none for text that
    /// reads as no time, as empty text does
    pub(in crate::back) fn time(&self) -> Option<f32> {
        self.value
    }

    /// Returns whether the text reads as a time, as empty text does too
    pub fn is_valid(&self) -> bool {
        self.value.is_some() || self.text.trim().is_empty()
    }
}

impl From<String> for Seconds {
    /// Reads the text as seconds, or as minutes and seconds with a colon
    /// between them, as in `1:30`, which is how the cue list shows times,
    /// or as hours, minutes, and seconds.  Text that reads as no time, or
    /// as no finite time, such as `inf` or `1e40`, counts as zero, so that
    /// no cue waits forever.
    fn from(text: String) -> Self {
        let value = text
            .trim()
            .split(':')
            .map(|part| {
                part.trim()
                    .parse::<f32>()
                    .ok()
                    .filter(|part| part.is_finite() && *part >= 0.0)
            })
            .try_fold(0.0_f32, |total, part| part.map(|part| total * 60.0 + part))
            .filter(|value| value.is_finite());
        Self { text, value }
    }
}

impl From<f32> for Seconds {
    fn from(value: f32) -> Self {
        Self::from(value.to_string())
    }
}
