//! `Aspose.Pdf.PageLabel` — the labelling rule for a run of pages,
//! per `capabilities/page_label.yaml`.
//!
//! A default page label starts numbering at `1` with Arabic
//! numerals and no prefix.

use crate::public::numbering_style::NumberingStyle;

/// A page-label rule: the starting value, numbering style, and
/// prefix applied to a run of pages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageLabel {
    starting_value: i32,
    numbering_style: NumberingStyle,
    prefix: String,
}

impl Default for PageLabel {
    fn default() -> Self {
        Self {
            starting_value: 1,
            numbering_style: NumberingStyle::NumeralsArabic,
            prefix: String::new(),
        }
    }
}

impl PageLabel {
    /// Construct a default page label (start `1`, Arabic numerals,
    /// no prefix).
    pub fn new() -> Self {
        Self::default()
    }

    /// The value the first page in the run is numbered with.
    pub fn starting_value(&self) -> i32 {
        self.starting_value
    }
    /// Set the starting value.
    pub fn set_starting_value(&mut self, value: i32) {
        self.starting_value = value;
    }

    /// The numbering style.
    pub fn numbering_style(&self) -> NumberingStyle {
        self.numbering_style
    }
    /// Set the numbering style.
    pub fn set_numbering_style(&mut self, value: NumberingStyle) {
        self.numbering_style = value;
    }

    /// The label prefix prepended before the numeral.
    pub fn prefix(&self) -> &str {
        &self.prefix
    }
    /// Set the label prefix.
    pub fn set_prefix(&mut self, value: String) {
        self.prefix = value;
    }
}
