//! `Aspose.Pdf.Forms.ScalingReason` — forms enum, per `capabilities/scaling_reason.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// `ScalingReason` — see the canonical Aspose.PDF documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum ScalingReason {
    /// `Always`.
    #[default]
    Always = 0,
    /// `IconIsBigger`.
    IconIsBigger = 1,
    /// `IconIsSmaller`.
    IconIsSmaller = 2,
    /// `Never`.
    Never = 3,
}
