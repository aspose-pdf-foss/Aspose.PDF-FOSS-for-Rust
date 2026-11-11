//! `Aspose.Pdf.Text.TextEditOptions.ClippingPathsProcessingMode` — how
//! existing clipping paths are handled while editing text, per
//! `capabilities/clipping_paths_processing_mode.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// How existing clipping paths are handled while editing text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum ClippingPathsProcessingMode {
    /// Leave clipping paths unchanged.
    #[default]
    KeepIntact = 0,
    /// Expand clipping paths to fit the edited text.
    Expand = 1,
    /// Remove clipping paths.
    Remove = 2,
}
