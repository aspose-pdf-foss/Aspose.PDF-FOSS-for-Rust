//! `Aspose.Pdf.FileEncoding` — embedded-file payload encoding
//! selector, per `capabilities/file_encoding.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0
//! `Aspose.Pdf.FileEncoding` enum.

/// Encoding applied to an embedded file's payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum FileEncoding {
    /// Store the payload uncompressed.
    #[default]
    None = 0,
    /// Compress the payload with the Zip (Flate) filter.
    Zip = 1,
}
