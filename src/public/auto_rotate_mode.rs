//! `Aspose.Pdf.Facades.AutoRotateMode` — page auto-rotation mode, per
//! `capabilities/auto_rotate_mode.yaml`. Ordinals match canonical 26.4.0.

/// A page auto-rotation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum AutoRotateMode {
    /// No auto-rotation.
    #[default]
    None = 0,
    /// Rotate clockwise.
    ClockWise = 1,
    /// Rotate anti-clockwise.
    AntiClockWise = 2,
}
