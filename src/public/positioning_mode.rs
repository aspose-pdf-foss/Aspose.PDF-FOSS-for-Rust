//! `Aspose.Pdf.Facades.PositioningMode` — text-positioning mode, per
//! `capabilities/positioning_mode.yaml`. Ordinals match canonical 26.4.0.

/// A text-positioning mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum PositioningMode {
    /// Legacy positioning.
    #[default]
    Legacy = 0,
    /// Modern line-spacing positioning.
    ModernLineSpacing = 1,
    /// The current default positioning.
    Current = 2,
}
