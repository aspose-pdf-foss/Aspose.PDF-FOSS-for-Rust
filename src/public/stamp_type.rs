//! `Aspose.Pdf.Facades.StampType` — stamp content kind, per
//! `capabilities/stamp_type.yaml`. Ordinals match canonical 26.4.0.

/// A stamp content kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum StampType {
    /// A form (vector) stamp.
    #[default]
    Form = 0,
    /// An image stamp.
    Image = 1,
}
