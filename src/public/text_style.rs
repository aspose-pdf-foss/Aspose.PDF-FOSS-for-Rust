//! `Aspose.Pdf.Annotations.TextStyle` — the text styling applied to
//! a free-text annotation, per `capabilities/text_style.yaml`.
//!
//! The canonical `Color` property is typed `System.Drawing.Color`
//! and drops on the Rust surface (matching the C++ reference
//! subset); the font name, font size, and the two alignment
//! properties ship. The canonical type has no public constructor —
//! `Default` is the idiomatic Rust stand-in.

use crate::public::horizontal_alignment::HorizontalAlignment;
use crate::public::text_alignment::TextAlignment;
use std::fmt;

/// Text styling (font name, size, and alignment) for a free-text
/// annotation.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TextStyle {
    font_name: String,
    font_size: f64,
    alignment: TextAlignment,
    horizontal_alignment: HorizontalAlignment,
}

impl TextStyle {
    /// The font name.
    pub fn font_name(&self) -> &str {
        &self.font_name
    }
    /// Set the font name.
    pub fn set_font_name(&mut self, value: String) {
        self.font_name = value;
    }

    /// The font size, in points.
    pub fn font_size(&self) -> f64 {
        self.font_size
    }
    /// Set the font size, in points.
    pub fn set_font_size(&mut self, value: f64) {
        self.font_size = value;
    }

    /// The paragraph text alignment.
    pub fn alignment(&self) -> TextAlignment {
        self.alignment
    }
    /// Set the paragraph text alignment.
    pub fn set_alignment(&mut self, value: TextAlignment) {
        self.alignment = value;
    }

    /// The horizontal alignment.
    pub fn horizontal_alignment(&self) -> HorizontalAlignment {
        self.horizontal_alignment
    }
    /// Set the horizontal alignment.
    pub fn set_horizontal_alignment(&mut self, value: HorizontalAlignment) {
        self.horizontal_alignment = value;
    }
}

impl fmt::Display for TextStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.font_name, self.font_size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accessors_roundtrip() {
        let mut ts = TextStyle::default();
        ts.set_font_name("Times".into());
        ts.set_font_size(11.0);
        ts.set_alignment(TextAlignment::Center);
        ts.set_horizontal_alignment(HorizontalAlignment::Right);
        assert_eq!(ts.font_name(), "Times");
        assert_eq!(ts.font_size(), 11.0);
        assert_eq!(ts.alignment(), TextAlignment::Center);
        assert_eq!(ts.horizontal_alignment(), HorizontalAlignment::Right);
        assert_eq!(ts.to_string(), "Times 11");
    }
}
