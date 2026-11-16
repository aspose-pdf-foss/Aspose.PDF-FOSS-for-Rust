//! `Aspose.Pdf.Annotations.HighlightingMode` — annotation enum, per `capabilities/highlighting_mode.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// The visual effect shown while a link or widget annotation is being activated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum HighlightingMode {
    /// No visual effect while the annotation is activated. Written as `N`.
    #[default]
    None = 0,
    /// Invert the annotation's rectangle. This is the format's own default, so it writes no entry.
    Invert = 1,
    /// Invert the annotation's border. Written as `O`.
    Outline = 2,
    /// Draw the annotation's down appearance, as if pressed. Written as `P`.
    Push = 3,
    /// Toggle between the down and the normal appearance. Written as `T`.
    Toggle = 4,
}
