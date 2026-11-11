//! `Aspose.Pdf.Facades.FieldType` — the kind of form field a
//! `FormEditor` creates, per `capabilities/field_type.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0
//! `Aspose.Pdf.Facades.FieldType` enum.

/// A form-field kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum FieldType {
    /// Single-line text field.
    #[default]
    Text = 0,
    /// Combo-box (editable drop-down).
    ComboBox = 1,
    /// List-box.
    ListBox = 2,
    /// Radio-button group.
    Radio = 3,
    /// Check-box.
    CheckBox = 4,
    /// Push-button.
    PushButton = 5,
    /// Multi-line text field.
    MultiLineText = 6,
    /// Barcode field.
    Barcode = 7,
    /// Invalid / unrecognised name or type.
    InvalidNameOrType = 8,
    /// Signature field.
    Signature = 9,
    /// Image (button) field.
    Image = 10,
    /// Numeric text field.
    Numeric = 11,
    /// Date-time field.
    DateTime = 12,
}
