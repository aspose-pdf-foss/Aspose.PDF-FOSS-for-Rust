//! `Aspose.Pdf.Forms.ScalingMode` — forms enum, per `capabilities/scaling_mode.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// `ScalingMode` — see the canonical Aspose.PDF documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum ScalingMode {
    /// `Proportional`.
    #[default]
    Proportional = 0,
    /// `Anamorphic`.
    Anamorphic = 1,
}
