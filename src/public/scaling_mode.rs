//! `Aspose.Pdf.Forms.ScalingMode` — forms enum, per `capabilities/scaling_mode.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// Whether a button icon keeps its proportions when scaled to its field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum ScalingMode {
    /// Keep the icon's aspect ratio. Written as `P`.
    #[default]
    Proportional = 0,
    /// Stretch the icon to fill the field, ignoring its aspect ratio. Written as `A`.
    Anamorphic = 1,
}
