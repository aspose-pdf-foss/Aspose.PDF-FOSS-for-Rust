//! `Aspose.Pdf.SvgLoadOptions` — options for importing an SVG, per
//! `capabilities/svg_load_options.yaml`.
//!
//! Passed to [`Document::open_svg`](crate::Document::open_svg).
//! Extends [`LoadOptions`].
//!
//! **Neither is consulted, and the two are unconsulted for different
//! reasons.**
//!
//! [`adjust_page_size`](SvgLoadOptions::adjust_page_size) defaults to
//! `true` and `true` is what the import already does: the page is sized
//! from the SVG's own width and height, falling back to US Letter when
//! it declares none. What is missing is the `false` case — keeping a
//! fixed page size and fitting the drawing into it.
//!
//! [`disable_font_license_verifications`](SvgLoadOptions::disable_font_license_verifications)
//! asks for a check to be skipped, and the question does not arise: the
//! import renders the SVG's shapes and no text, so no font is embedded
//! and there is no licence to verify.

use crate::public::load_options::LoadOptions;

/// Options controlling SVG import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SvgLoadOptions {
    disable_font_license_verifications: bool,
    adjust_page_size: bool,
}

impl Default for SvgLoadOptions {
    fn default() -> Self {
        Self {
            disable_font_license_verifications: false,
            adjust_page_size: true,
        }
    }
}

impl SvgLoadOptions {
    /// Construct default SVG load options (page sized to the SVG).
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the imported page is resized to the SVG's dimensions.
    pub fn adjust_page_size(&self) -> bool {
        self.adjust_page_size
    }
    /// Set whether the imported page is resized to the SVG.
    pub fn set_adjust_page_size(&mut self, value: bool) {
        self.adjust_page_size = value;
    }
}

impl LoadOptions for SvgLoadOptions {
    fn disable_font_license_verifications(&self) -> bool {
        self.disable_font_license_verifications
    }
    fn set_disable_font_license_verifications(&mut self, value: bool) {
        self.disable_font_license_verifications = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_accessors() {
        let mut o = SvgLoadOptions::new();
        assert!(o.adjust_page_size());
        assert!(!o.disable_font_license_verifications());
        o.set_adjust_page_size(false);
        o.set_disable_font_license_verifications(true);
        assert!(!o.adjust_page_size());
        assert!(o.disable_font_license_verifications());
    }
}
