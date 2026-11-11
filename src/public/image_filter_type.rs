//! `Aspose.Pdf.ImageFilterType` — the compression filter applied when
//! an image is added to a page's resources, per
//! `capabilities/image_filter_type.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// The compression filter applied when an image is added.
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum ImageFilterType {
    /// JPEG 2000 compression.
    #[default]
    Jpeg2000 = 0,
    /// Baseline JPEG (DCT) compression.
    Jpeg = 1,
    /// Flate (zlib) compression.
    Flate = 2,
    /// CCITT Group 3/4 fax compression.
    CCITTFax = 3,
}
