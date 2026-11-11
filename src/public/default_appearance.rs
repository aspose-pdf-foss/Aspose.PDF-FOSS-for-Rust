//! `Aspose.Pdf.Annotations.DefaultAppearance` — the default text
//! appearance (font + size) of a variable-text annotation, per
//! `capabilities/default_appearance.yaml`.
//!
//! Ships the default constructor and the font-name / size / resource
//! -name properties plus the read-only assembled `text` string. The
//! `TextColor` (`System.Drawing.Color`) and the colour / `Font`
//! constructors, and the `Font` getter (`Aspose.Pdf.Text.Font`,
//! text/fonts cluster), are dropped — matching the C++ reference
//! subset (see `targets/rust/{translations,drops}.yaml`).

/// The default appearance (`/DA`) of a variable-text annotation.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DefaultAppearance {
    font_size: f64,
    font_name: String,
    font_resource_name: String,
}

impl DefaultAppearance {
    /// Construct a default (empty) appearance.
    pub fn new() -> Self {
        Self::default()
    }

    /// The font size, in points.
    pub fn font_size(&self) -> f64 {
        self.font_size
    }
    /// Set the font size, in points.
    pub fn set_font_size(&mut self, value: f64) {
        self.font_size = value;
    }

    /// The font name (e.g. `Helvetica`).
    pub fn font_name(&self) -> &str {
        &self.font_name
    }
    /// Set the font name.
    pub fn set_font_name(&mut self, value: String) {
        self.font_name = value;
    }

    /// The font resource name (the `/Font` resource key, e.g. `Helv`).
    pub fn font_resource_name(&self) -> &str {
        &self.font_resource_name
    }
    /// Set the font resource name.
    pub fn set_font_resource_name(&mut self, value: String) {
        self.font_resource_name = value;
    }

    /// The assembled default-appearance operator string (e.g.
    /// `/Helv 12 Tf 0 g`), or an empty string when no font is set.
    pub fn text(&self) -> String {
        let name = if !self.font_resource_name.is_empty() {
            &self.font_resource_name
        } else {
            &self.font_name
        };
        if name.is_empty() {
            String::new()
        } else {
            format!("/{name} {} Tf 0 g", self.font_size)
        }
    }
}
