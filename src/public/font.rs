//! `Aspose.Pdf.Text.Font` — a font usable for drawing text, per
//! `capabilities/font.yaml`.
//!
//! A `Font` is obtained from
//! [`FontRepository`](crate::FontRepository) — it has no public
//! constructor. A name-resolved font (`find_font`) carries just a
//! name and is not embedded; a file-opened font (`open_font`)
//! carries the embeddable font program and is embedded by default.

/// A font descriptor for text drawing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Font {
    font_name: String,
    base_font: String,
    is_embedded: bool,
    is_subset: bool,
    /// The embeddable font program (the raw TrueType bytes), when
    /// this font was opened from a file. Consumed by the text
    /// drawing / save pipeline to embed a `FontFile2`. `None` for
    /// name-resolved fonts.
    pub(crate) program: Option<Vec<u8>>,
}

impl Font {
    /// A name-resolved font (the `find_font` result): a `base_font`
    /// equal to the name, not embedded, no program.
    pub(crate) fn resolved(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            font_name: name.clone(),
            base_font: name,
            is_embedded: false,
            is_subset: false,
            program: None,
        }
    }

    /// A font opened from an embeddable program (the `open_font`
    /// result): embedded by default, carrying the font bytes.
    pub(crate) fn from_program(name: impl Into<String>, program: Vec<u8>) -> Self {
        let name = name.into();
        Self {
            font_name: name.clone(),
            base_font: name,
            is_embedded: true,
            is_subset: false,
            program: Some(program),
        }
    }

    /// The font name.
    pub fn font_name(&self) -> &str {
        &self.font_name
    }

    /// The PostScript base font name.
    pub fn base_font(&self) -> &str {
        &self.base_font
    }

    /// Whether the font program is embedded into the document.
    pub fn is_embedded(&self) -> bool {
        self.is_embedded
    }
    /// Set whether the font program is embedded into the document.
    pub fn set_is_embedded(&mut self, value: bool) {
        self.is_embedded = value;
    }

    /// Whether only the used glyphs are embedded (subsetting).
    pub fn is_subset(&self) -> bool {
        self.is_subset
    }
    /// Set whether only the used glyphs are embedded (subsetting).
    pub fn set_is_subset(&mut self, value: bool) {
        self.is_subset = value;
    }
}
