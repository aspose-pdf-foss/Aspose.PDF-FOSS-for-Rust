//! `Aspose.Pdf.Devices.FormPresentationMode` — selects how
//! AcroForm widgets are presented during rendering.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0
//! `Aspose.Pdf.Devices.FormPresentationMode` enum:
//! `Production=0`, `Editor=1`. `Production` is the canonical
//! engine default — it preserves the widget's stored appearance
//! stream as authored. `Editor` regenerates appearance from the
//! field model.
//!
//! The Rust v1 raster pipeline does not yet honour either mode
//! at encode time (AcroForm rendering is deferred); this enum
//! ships so the carrier surface on
//! [`ImageDevice`](crate::ImageDevice) and
//! [`TiffDevice`](crate::TiffDevice) can round-trip.

/// Selects how AcroForm widgets are presented during rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
#[derive(Default)]
pub enum FormPresentationMode {
    /// Preserve each widget's stored appearance stream as authored.
    /// Canonical engine default.
    #[default]
    Production = 0,
    /// Regenerate widget appearance from the field model.
    Editor = 1,
}

