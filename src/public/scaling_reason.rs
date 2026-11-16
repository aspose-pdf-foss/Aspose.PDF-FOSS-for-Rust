//! `Aspose.Pdf.Forms.ScalingReason` — forms enum, per `capabilities/scaling_reason.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// When a button icon is scaled to fit its field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum ScalingReason {
    /// Always scale the icon. Written as `A`.
    #[default]
    Always = 0,
    /// Scale only when the icon is larger than the field. Written as `B`.
    IconIsBigger = 1,
    /// Scale only when the icon is smaller than the field. Written as `S`.
    IconIsSmaller = 2,
    /// Never scale the icon. Written as `N`.
    Never = 3,
}
