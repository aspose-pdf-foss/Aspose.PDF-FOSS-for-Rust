//! `Aspose.Pdf.Forms.Option` — a single entry of a choice field, per
//! `capabilities/option.yaml`.
//!
//! Note: the canonical type name `Option` shadows the standard
//! library's [`Option`](::core::option::Option) when glob-imported.
//! Within this crate the form option is aliased where it would
//! collide; downstream callers can `use aspose_pdf_foss::Option`
//! explicitly.

/// One selectable entry of a choice (combo / list / radio) field.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Option {
    value: String,
    name: String,
    selected: bool,
    index: i32,
}

impl Option {
    /// Construct an empty option.
    pub fn new() -> Self {
        Self::default()
    }

    /// The export value.
    pub fn value(&self) -> &str {
        &self.value
    }
    /// Set the export value.
    pub fn set_value(&mut self, value: String) {
        self.value = value;
    }

    /// The display name.
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Set the display name.
    pub fn set_name(&mut self, value: String) {
        self.name = value;
    }

    /// Whether the option is selected.
    pub fn selected(&self) -> bool {
        self.selected
    }
    /// Set whether the option is selected.
    pub fn set_selected(&mut self, value: bool) {
        self.selected = value;
    }

    /// The 0-based index of the option within its collection.
    pub fn index(&self) -> i32 {
        self.index
    }

    pub(crate) fn set_index(&mut self, value: i32) {
        self.index = value;
    }
}
