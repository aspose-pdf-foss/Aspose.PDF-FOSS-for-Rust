//! `Aspose.Pdf.Text.TextReplaceOptions.Scope` — how many matches a
//! text replacement affects, per `capabilities/scope.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum
//! (canonical names `REPLACE_FIRST` / `REPLACE_ALL`).

/// How many matches a text replacement affects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum Scope {
    /// Replace only the first match.
    #[default]
    ReplaceFirst = 0,
    /// Replace every match.
    ReplaceAll = 1,
}
