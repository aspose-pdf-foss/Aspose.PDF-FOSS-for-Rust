//! `Aspose.Pdf.ExtractImageMode` — which images an extractor collects,
//! per `capabilities/extract_image_mode.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// Which images an extractor collects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum ExtractImageMode {
    /// Every image defined in the page resources.
    #[default]
    DefinedInResources = 0,
    /// Only images actually drawn by the page content.
    ActuallyUsed = 1,
}
