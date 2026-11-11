//! `Aspose.Pdf.Facades.FormFieldFacade` — the appearance-attribute
//! carrier a `FormEditor` applies to a form field, per
//! `capabilities/form_field_facade.yaml`.
//!
//! A pure value type: border / font / alignment / caption / items
//! presets plus the canonical preset constants. The
//! `System.Drawing.Color` (`BorderColor` / `TextColor` /
//! `BackgroundColor` / `BackgroudColor`) and `System.Drawing.Rectangle`
//! (`Box`) properties cascade off the dropped translations list, matching
//! the C++ reference.
//!
//! **Applied by the editor.** A [`FormEditor`](crate::FormEditor) reads
//! the border width and style, the font and its size, and the check-box
//! style off this facade and sets them on the field it adds, where they
//! reach the file as the field's `/BS`, `/DA` and box style.
//!
//! **Carried, not written.** The rest describe a generated appearance
//! stream, which this crate does not emit: the caption and its items,
//! the rotation, the vertical and horizontal alignment, and the text
//! encoding a `/DA` font resource would carry.

use crate::public::encoding_type::EncodingType;
use crate::public::font_style::FontStyle;

/// Form-field appearance attributes.
#[derive(Debug, Clone, PartialEq)]
pub struct FormFieldFacade {
    border_style: i32,
    border_width: f32,
    font: FontStyle,
    custom_font: String,
    font_size: f32,
    text_encoding: EncodingType,
    alignment: i32,
    rotation: i32,
    caption: String,
    button_style: i32,
    position: Vec<f32>,
    page_number: i32,
    items: Vec<String>,
    export_items: Vec<Vec<String>>,
}

impl FormFieldFacade {
    // ---- Border width presets ----

    /// Undefined border width (canonical misspelling alias).
    pub const BORDER_WIDTH_UNDIFIED: f32 = -1.0;
    /// Undefined border width.
    pub const BORDER_WIDTH_UNDEFINED: f32 = -1.0;
    /// Thin (1 pt) border.
    pub const BORDER_WIDTH_THIN: f32 = 1.0;
    /// Medium (2 pt) border.
    pub const BORDER_WIDTH_MEDIUM: f32 = 2.0;
    /// Thick (3 pt) border.
    pub const BORDER_WIDTH_THICK: f32 = 3.0;

    // ---- Border style presets ----

    /// Solid border.
    pub const BORDER_STYLE_SOLID: i32 = 0;
    /// Dashed border.
    pub const BORDER_STYLE_DASHED: i32 = 1;
    /// Beveled border.
    pub const BORDER_STYLE_BEVELED: i32 = 2;
    /// Inset border.
    pub const BORDER_STYLE_INSET: i32 = 3;
    /// Underline border.
    pub const BORDER_STYLE_UNDERLINE: i32 = 4;
    /// Undefined border style.
    pub const BORDER_STYLE_UNDEFINED: i32 = 5;

    // ---- Alignment presets (horizontal + vertical share the space) ----

    /// Left-aligned.
    pub const ALIGN_LEFT: i32 = 0;
    /// Centre-aligned.
    pub const ALIGN_CENTER: i32 = 1;
    /// Right-aligned.
    pub const ALIGN_RIGHT: i32 = 2;
    /// Undefined alignment.
    pub const ALIGN_UNDEFINED: i32 = 3;
    /// Justified.
    pub const ALIGN_JUSTIFIED: i32 = 4;
    /// Top-aligned (vertical).
    pub const ALIGN_TOP: i32 = 0;
    /// Middle-aligned (vertical).
    pub const ALIGN_MIDDLE: i32 = 1;
    /// Bottom-aligned (vertical).
    pub const ALIGN_BOTTOM: i32 = 2;

    // ---- Check-box glyph presets (ZapfDingbats code points) ----

    /// Circle glyph.
    pub const CHECK_BOX_STYLE_CIRCLE: i32 = 108;
    /// Check glyph.
    pub const CHECK_BOX_STYLE_CHECK: i32 = 52;
    /// Cross glyph.
    pub const CHECK_BOX_STYLE_CROSS: i32 = 56;
    /// Diamond glyph.
    pub const CHECK_BOX_STYLE_DIAMOND: i32 = 117;
    /// Star glyph.
    pub const CHECK_BOX_STYLE_STAR: i32 = 72;
    /// Square glyph.
    pub const CHECK_BOX_STYLE_SQUARE: i32 = 110;
    /// Undefined glyph.
    pub const CHECK_BOX_STYLE_UNDEFINED: i32 = 32;

    /// Construct a facade with every attribute at its default.
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset every appearance attribute to its default.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// The border style preset.
    pub fn border_style(&self) -> i32 {
        self.border_style
    }
    /// Set the border style preset.
    pub fn set_border_style(&mut self, value: i32) {
        self.border_style = value;
    }

    /// The border width.
    pub fn border_width(&self) -> f32 {
        self.border_width
    }
    /// Set the border width.
    pub fn set_border_width(&mut self, value: f32) {
        self.border_width = value;
    }

    /// The built-in font.
    pub fn font(&self) -> FontStyle {
        self.font
    }
    /// Set the built-in font.
    pub fn set_font(&mut self, value: FontStyle) {
        self.font = value;
    }

    /// The custom-font name (empty when a built-in font is used).
    pub fn custom_font(&self) -> &str {
        &self.custom_font
    }
    /// Set the custom-font name.
    pub fn set_custom_font(&mut self, value: impl Into<String>) {
        self.custom_font = value.into();
    }

    /// The font size (0 = auto).
    pub fn font_size(&self) -> f32 {
        self.font_size
    }
    /// Set the font size.
    pub fn set_font_size(&mut self, value: f32) {
        self.font_size = value;
    }

    /// The text encoding.
    pub fn text_encoding(&self) -> EncodingType {
        self.text_encoding
    }
    /// Set the text encoding.
    pub fn set_text_encoding(&mut self, value: EncodingType) {
        self.text_encoding = value;
    }

    /// The alignment preset.
    pub fn alignment(&self) -> i32 {
        self.alignment
    }
    /// Set the alignment preset.
    pub fn set_alignment(&mut self, value: i32) {
        self.alignment = value;
    }

    /// The rotation, in degrees.
    pub fn rotation(&self) -> i32 {
        self.rotation
    }
    /// Set the rotation, in degrees.
    pub fn set_rotation(&mut self, value: i32) {
        self.rotation = value;
    }

    /// The button caption.
    pub fn caption(&self) -> &str {
        &self.caption
    }
    /// Set the button caption.
    pub fn set_caption(&mut self, value: impl Into<String>) {
        self.caption = value.into();
    }

    /// The check-box glyph preset.
    pub fn button_style(&self) -> i32 {
        self.button_style
    }
    /// Set the check-box glyph preset.
    pub fn set_button_style(&mut self, value: i32) {
        self.button_style = value;
    }

    /// The `[llx, lly, urx, ury]` position (empty when unset).
    pub fn position(&self) -> &[f32] {
        &self.position
    }
    /// Set the `[llx, lly, urx, ury]` position.
    pub fn set_position(&mut self, value: Vec<f32>) {
        self.position = value;
    }

    /// The 1-based page number.
    pub fn page_number(&self) -> i32 {
        self.page_number
    }
    /// Set the 1-based page number.
    pub fn set_page_number(&mut self, value: i32) {
        self.page_number = value;
    }

    /// The list/combo display items.
    pub fn items(&self) -> &[String] {
        &self.items
    }
    /// Set the list/combo display items.
    pub fn set_items(&mut self, value: Vec<String>) {
        self.items = value;
    }

    /// The list/combo `[display, export]` item pairs.
    pub fn export_items(&self) -> &[Vec<String>] {
        &self.export_items
    }
    /// Set the list/combo `[display, export]` item pairs.
    pub fn set_export_items(&mut self, value: Vec<Vec<String>>) {
        self.export_items = value;
    }
}

impl Default for FormFieldFacade {
    fn default() -> Self {
        Self {
            border_style: Self::BORDER_STYLE_UNDEFINED,
            border_width: Self::BORDER_WIDTH_UNDEFINED,
            font: FontStyle::Courier,
            custom_font: String::new(),
            font_size: 0.0,
            text_encoding: EncodingType::Identity_h,
            alignment: Self::ALIGN_UNDEFINED,
            rotation: 0,
            caption: String::new(),
            button_style: Self::CHECK_BOX_STYLE_UNDEFINED,
            position: Vec::new(),
            page_number: 1,
            items: Vec::new(),
            export_items: Vec::new(),
        }
    }
}

impl FormFieldFacade {
    /// The border style as a [`BorderStyle`], or `None` when the facade
    /// leaves it undefined.
    pub(crate) fn border_style_value(&self) -> Option<crate::public::border_style::BorderStyle> {
        use crate::public::border_style::BorderStyle;
        match self.border_style {
            Self::BORDER_STYLE_SOLID => Some(BorderStyle::Solid),
            Self::BORDER_STYLE_DASHED => Some(BorderStyle::Dashed),
            Self::BORDER_STYLE_BEVELED => Some(BorderStyle::Beveled),
            Self::BORDER_STYLE_INSET => Some(BorderStyle::Inset),
            Self::BORDER_STYLE_UNDERLINE => Some(BorderStyle::Underline),
            _ => None,
        }
    }

    /// The check-box style as a [`BoxStyle`]. The facade spells these as
    /// the ZapfDingbats character codes a `/MK /CA` caption would use
    /// (`4` is a check, `8` a cross, `l` a circle), which is why the
    /// constants are those numbers.
    pub(crate) fn box_style_value(&self) -> Option<crate::public::box_style::BoxStyle> {
        use crate::public::box_style::BoxStyle;
        match self.button_style {
            Self::CHECK_BOX_STYLE_CIRCLE => Some(BoxStyle::Circle),
            Self::CHECK_BOX_STYLE_CHECK => Some(BoxStyle::Check),
            Self::CHECK_BOX_STYLE_CROSS => Some(BoxStyle::Cross),
            Self::CHECK_BOX_STYLE_DIAMOND => Some(BoxStyle::Diamond),
            Self::CHECK_BOX_STYLE_STAR => Some(BoxStyle::Star),
            Self::CHECK_BOX_STYLE_SQUARE => Some(BoxStyle::Square),
            _ => None,
        }
    }

    /// The `/DA` font name and size this facade asks for, or `None`
    /// when it names neither. A custom font name wins over the
    /// [`FontStyle`] preset, which is what it is for.
    pub(crate) fn default_appearance_value(
        &self,
    ) -> Option<crate::public::default_appearance::DefaultAppearance> {
        let name = if !self.custom_font.is_empty() {
            self.custom_font.clone()
        } else {
            self.font.base_font_name()?.to_string()
        };
        if self.font_size <= 0.0 {
            return None;
        }
        let mut da = crate::public::default_appearance::DefaultAppearance::new();
        da.set_font_name(name);
        da.set_font_size(self.font_size as f64);
        Some(da)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_constants() {
        assert_eq!(FormFieldFacade::BORDER_WIDTH_UNDEFINED, -1.0);
        assert_eq!(FormFieldFacade::BORDER_WIDTH_UNDIFIED, -1.0);
        assert_eq!(FormFieldFacade::BORDER_WIDTH_THIN, 1.0);
        assert_eq!(FormFieldFacade::BORDER_WIDTH_MEDIUM, 2.0);
        assert_eq!(FormFieldFacade::BORDER_WIDTH_THICK, 3.0);

        assert_eq!(FormFieldFacade::BORDER_STYLE_SOLID, 0);
        assert_eq!(FormFieldFacade::BORDER_STYLE_UNDERLINE, 4);
        assert_eq!(FormFieldFacade::BORDER_STYLE_UNDEFINED, 5);

        assert_eq!(FormFieldFacade::ALIGN_LEFT, 0);
        assert_eq!(FormFieldFacade::ALIGN_JUSTIFIED, 4);
        assert_eq!(FormFieldFacade::ALIGN_TOP, 0);
        assert_eq!(FormFieldFacade::ALIGN_BOTTOM, 2);

        assert_eq!(FormFieldFacade::CHECK_BOX_STYLE_CIRCLE, 108);
        assert_eq!(FormFieldFacade::CHECK_BOX_STYLE_CHECK, 52);
        assert_eq!(FormFieldFacade::CHECK_BOX_STYLE_CROSS, 56);
        assert_eq!(FormFieldFacade::CHECK_BOX_STYLE_DIAMOND, 117);
        assert_eq!(FormFieldFacade::CHECK_BOX_STYLE_STAR, 72);
        assert_eq!(FormFieldFacade::CHECK_BOX_STYLE_SQUARE, 110);
        assert_eq!(FormFieldFacade::CHECK_BOX_STYLE_UNDEFINED, 32);
    }

    #[test]
    fn defaults() {
        let f = FormFieldFacade::new();
        assert_eq!(f.border_style(), FormFieldFacade::BORDER_STYLE_UNDEFINED);
        assert_eq!(f.border_width(), FormFieldFacade::BORDER_WIDTH_UNDEFINED);
        assert_eq!(f.font(), FontStyle::Courier);
        assert!(f.custom_font().is_empty());
        assert_eq!(f.font_size(), 0.0);
        assert_eq!(f.text_encoding(), EncodingType::Identity_h);
        assert_eq!(f.alignment(), FormFieldFacade::ALIGN_UNDEFINED);
        assert_eq!(f.rotation(), 0);
        assert!(f.caption().is_empty());
        assert_eq!(f.button_style(), FormFieldFacade::CHECK_BOX_STYLE_UNDEFINED);
        assert!(f.position().is_empty());
        assert_eq!(f.page_number(), 1);
        assert!(f.items().is_empty());
        assert!(f.export_items().is_empty());
    }

    #[test]
    fn property_roundtrip() {
        let mut f = FormFieldFacade::new();
        f.set_border_style(FormFieldFacade::BORDER_STYLE_BEVELED);
        f.set_border_width(FormFieldFacade::BORDER_WIDTH_THICK);
        f.set_font(FontStyle::CourierBold);
        f.set_custom_font("MyFont");
        f.set_font_size(12.5);
        f.set_text_encoding(EncodingType::Cp1250);
        f.set_alignment(FormFieldFacade::ALIGN_CENTER);
        f.set_rotation(90);
        f.set_caption("Submit");
        f.set_button_style(FormFieldFacade::CHECK_BOX_STYLE_STAR);
        f.set_position(vec![1.0, 2.0, 3.0, 4.0]);
        f.set_page_number(3);
        f.set_items(vec!["A".into(), "B".into()]);
        f.set_export_items(vec![vec!["x".into(), "1".into()], vec!["y".into(), "2".into()]]);

        assert_eq!(f.border_style(), FormFieldFacade::BORDER_STYLE_BEVELED);
        assert_eq!(f.border_width(), 3.0);
        assert_eq!(f.font(), FontStyle::CourierBold);
        assert_eq!(f.custom_font(), "MyFont");
        assert_eq!(f.font_size(), 12.5);
        assert_eq!(f.text_encoding(), EncodingType::Cp1250);
        assert_eq!(f.alignment(), FormFieldFacade::ALIGN_CENTER);
        assert_eq!(f.rotation(), 90);
        assert_eq!(f.caption(), "Submit");
        assert_eq!(f.button_style(), FormFieldFacade::CHECK_BOX_STYLE_STAR);
        assert_eq!(f.position(), &[1.0, 2.0, 3.0, 4.0]);
        assert_eq!(f.page_number(), 3);
        assert_eq!(f.items().len(), 2);
        assert_eq!(f.export_items().len(), 2);

        f.reset();
        assert_eq!(f.border_style(), FormFieldFacade::BORDER_STYLE_UNDEFINED);
        assert!(f.items().is_empty());
    }
}
