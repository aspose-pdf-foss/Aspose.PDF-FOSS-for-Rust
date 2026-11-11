//! `Aspose.Pdf.FloatingBox` — a positioned box with a size, a fill,
//! and child paragraphs, per `capabilities/floating_box.yaml`.
//!
//! A [`BaseParagraph`](crate::BaseParagraph) carrying a width / height, a background
//! colour, a repeat flag, a border, and a nested [`Paragraphs`]
//! collection, added to a page through the page's paragraph collection.
//!
//! **Carried, not rendered.** A floating box contributes no content: it
//! takes the default `render_content`, which answers nothing, so its
//! size, its background colour, its border and its children never
//! reach the page. The reason is the one the layout layer shares — see
//! the note at `RenderedParagraph` in
//! [`BaseParagraph`](crate::BaseParagraph) — sharpened by what a box
//! is: a [`Table`](crate::Table) renders because it carries its own
//! `left` and `top`, while a floating box is placed by the flow it sits
//! in, and a paragraph render is handed a page height and nothing else.
//! There is no origin to put the box, or its children, at.

use crate::public::base_paragraph::{impl_base_paragraph, ParagraphBase};
use crate::public::border_info::BorderInfo;
use crate::public::color::Color;
use crate::public::paragraphs::Paragraphs;

/// A positioned box containing child paragraphs.
#[derive(Debug)]
pub struct FloatingBox {
    base: ParagraphBase,
    width: f64,
    height: f64,
    is_need_repeating: bool,
    paragraphs: Paragraphs,
    border: BorderInfo,
    background_color: Color,
}

impl Default for FloatingBox {
    fn default() -> Self {
        Self {
            base: ParagraphBase::default(),
            width: 0.0,
            height: 0.0,
            is_need_repeating: false,
            paragraphs: Paragraphs::new(),
            border: BorderInfo::new(),
            background_color: Color::empty(),
        }
    }
}

impl FloatingBox {
    /// Construct an empty box (zero size).
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct a box with the given width and height, in points.
    pub fn with_dimensions(width: f32, height: f32) -> Self {
        Self { width: width as f64, height: height as f64, ..Self::default() }
    }

    /// The box width, in points.
    pub fn width(&self) -> f64 {
        self.width
    }
    /// Set the box width, in points.
    pub fn set_width(&mut self, value: f64) {
        self.width = value;
    }

    /// The box height, in points.
    pub fn height(&self) -> f64 {
        self.height
    }
    /// Set the box height, in points.
    pub fn set_height(&mut self, value: f64) {
        self.height = value;
    }

    /// Whether the box repeats across pages.
    pub fn is_need_repeating(&self) -> bool {
        self.is_need_repeating
    }
    /// Set whether the box repeats across pages.
    pub fn set_is_need_repeating(&mut self, value: bool) {
        self.is_need_repeating = value;
    }

    /// The box's child paragraphs.
    pub fn paragraphs(&self) -> &Paragraphs {
        &self.paragraphs
    }
    /// The box's child paragraphs, mutably.
    pub fn paragraphs_mut(&mut self) -> &mut Paragraphs {
        &mut self.paragraphs
    }
    /// Replace the box's child paragraphs.
    pub fn set_paragraphs(&mut self, value: Paragraphs) {
        self.paragraphs = value;
    }

    /// The box's border.
    pub fn border(&self) -> &BorderInfo {
        &self.border
    }
    /// Set the box's border.
    pub fn set_border(&mut self, value: BorderInfo) {
        self.border = value;
    }

    /// The box's background (fill) colour.
    pub fn background_color(&self) -> Color {
        self.background_color
    }
    /// Set the box's background (fill) colour.
    pub fn set_background_color(&mut self, value: Color) {
        self.background_color = value;
    }
}

impl crate::public::base_paragraph::sealed::Renderable for FloatingBox {}
impl_base_paragraph!(FloatingBox);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::public::base_paragraph::BaseParagraph;
    use crate::public::horizontal_alignment::HorizontalAlignment;

    #[test]
    fn dimensions_and_properties() {
        let mut b = FloatingBox::with_dimensions(200.0, 120.0);
        assert_eq!(b.width(), 200.0);
        assert_eq!(b.height(), 120.0);
        assert!(!b.is_need_repeating());
        b.set_is_need_repeating(true);
        b.set_background_color(Color::from_rgb(1.0, 0.0, 0.0));
        assert!(b.is_need_repeating());
        assert_eq!(b.background_color().a(), 1.0);
    }

    #[test]
    fn is_a_base_paragraph_with_child_paragraphs() {
        let mut b = FloatingBox::new();
        // BaseParagraph geometry (via the trait).
        b.set_horizontal_alignment(HorizontalAlignment::Center);
        b.set_z_index(3);
        assert_eq!(b.horizontal_alignment(), HorizontalAlignment::Center);
        assert_eq!(b.z_index(), 3);

        // Nested paragraphs (a FloatingBox is itself a BaseParagraph).
        b.paragraphs_mut().add(Box::new(FloatingBox::with_dimensions(1.0, 1.0)));
        assert_eq!(b.paragraphs().count(), 1);
    }
}
