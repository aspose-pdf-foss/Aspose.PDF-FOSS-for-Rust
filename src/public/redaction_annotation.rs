//! `Aspose.Pdf.Annotations.RedactionAnnotation` — a redaction
//! annotation, per `capabilities/redaction_annotation.yaml`.
//!
//! Extends [`MarkupAnnotation`]. `Redact` burns the redacted region into
//! the page: when the annotation was constructed on a page, it queues a
//! redaction request on the page's shared edit buffer so the document's
//! next save removes every text-show whose origin falls inside the
//! rectangle. A `Document`-bound annotation (no page) holds no edit
//! handle, so `redact` is a safe no-op.

use crate::public::annotation::sealed::HasBase;
use crate::public::annotation::{Annotation, AnnotationBase};
use crate::public::annotation_type::AnnotationType;
use crate::public::color::Color;
use crate::public::document::Document;
use crate::public::horizontal_alignment::HorizontalAlignment;
use crate::public::markup_annotation::sealed::HasMarkupBase;
use crate::public::markup_annotation::{MarkupAnnotation, MarkupBase};
use crate::public::page::Page;
use crate::public::point::Point;
use crate::public::rectangle::Rectangle;

/// A redaction annotation marking a region to be removed.
pub struct RedactionAnnotation {
    base: AnnotationBase,
    markup: MarkupBase,
    quad_point: Vec<Point>,
    default_appearance: String,
    fill_color: Color,
    border_color: Color,
    overlay_text: String,
    font_size: f32,
    repeat: bool,
    text_alignment: HorizontalAlignment,
    /// The page binding captured at `on_page` construction: the page's
    /// leaf object id plus a clone of its shared edit buffer. `None` for
    /// a `Document`-bound annotation, which makes `redact` a no-op.
    page_binding: Option<(u32, crate::public::page_edits::EditBuffer)>,
}

impl RedactionAnnotation {
    fn build(
        base: AnnotationBase,
        page_binding: Option<(u32, crate::public::page_edits::EditBuffer)>,
    ) -> Self {
        Self {
            base,
            markup: MarkupBase::new(),
            quad_point: Vec::new(),
            default_appearance: String::new(),
            fill_color: Color::black(),
            border_color: Color::empty(),
            overlay_text: String::new(),
            font_size: 0.0,
            repeat: false,
            text_alignment: HorizontalAlignment::None,
            page_binding,
        }
    }

    /// Construct within a [`Document`].
    pub fn new(_document: &Document) -> Self {
        Self::build(AnnotationBase::default(), None)
    }

    /// Construct on a [`Page`] at the given rectangle.
    pub fn on_page(page: &Page, rect: Rectangle) -> Self {
        let mut base = AnnotationBase::new(rect);
        base.page_index = page.number() as i32;
        let binding = (page.leaf_id(), std::rc::Rc::clone(&page.edits));
        Self::build(base, Some(binding))
    }

    /// Burn the redaction into the page content. When the annotation was
    /// constructed on a page, queues a redaction request on the page's
    /// shared edit buffer so the document's next save removes every
    /// text-show whose origin falls inside the rectangle. A
    /// `Document`-bound annotation (no page) is a safe no-op.
    pub fn redact(&mut self) {
        if let Some((leaf_id, edits)) = &self.page_binding {
            let r = self.base.rect;
            edits.borrow_mut().redactions.push(
                crate::public::page_edits::RedactionRequest {
                    leaf_id: *leaf_id,
                    rect: [r.llx(), r.lly(), r.urx(), r.ury()],
                },
            );
        }
    }

    /// The marked quadrilateral regions.
    pub fn quad_point(&self) -> &[Point] {
        &self.quad_point
    }
    /// Set the marked quadrilateral regions.
    pub fn set_quad_point(&mut self, value: Vec<Point>) {
        self.quad_point = value;
    }

    /// The overlay-text default-appearance string.
    pub fn default_appearance(&self) -> &str {
        &self.default_appearance
    }
    /// Set the overlay-text default-appearance string.
    pub fn set_default_appearance(&mut self, value: String) {
        self.default_appearance = value;
    }

    /// The fill colour of the redacted region.
    pub fn fill_color(&self) -> Color {
        self.fill_color
    }
    /// Set the fill colour.
    pub fn set_fill_color(&mut self, value: Color) {
        self.fill_color = value;
    }

    /// The border colour.
    pub fn border_color(&self) -> Color {
        self.border_color
    }
    /// Set the border colour.
    pub fn set_border_color(&mut self, value: Color) {
        self.border_color = value;
    }

    /// The overlay text drawn over the redacted region.
    pub fn overlay_text(&self) -> &str {
        &self.overlay_text
    }
    /// Set the overlay text.
    pub fn set_overlay_text(&mut self, value: String) {
        self.overlay_text = value;
    }

    /// The overlay-text font size, in points.
    pub fn font_size(&self) -> f32 {
        self.font_size
    }
    /// Set the overlay-text font size.
    pub fn set_font_size(&mut self, value: f32) {
        self.font_size = value;
    }

    /// Whether the overlay text repeats to fill the region.
    pub fn repeat(&self) -> bool {
        self.repeat
    }
    /// Set whether the overlay text repeats.
    pub fn set_repeat(&mut self, value: bool) {
        self.repeat = value;
    }

    /// The overlay-text alignment.
    pub fn text_alignment(&self) -> HorizontalAlignment {
        self.text_alignment
    }
    /// Set the overlay-text alignment.
    pub fn set_text_alignment(&mut self, value: HorizontalAlignment) {
        self.text_alignment = value;
    }
}

impl HasBase for RedactionAnnotation {
    fn base(&self) -> &AnnotationBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut AnnotationBase {
        &mut self.base
    }
    fn pdf_entries(&self) -> Vec<(&'static str, crate::internal::annotation_io::Entry)> {
        use crate::internal::annotation_io::Entry;
        let mut out = Vec::new();
        if !self.quad_point.is_empty() {
            let flat = self.quad_point.iter().flat_map(|p| [p.x(), p.y()]).collect();
            out.push(("QuadPoints", Entry::Reals(flat)));
        }
        if !self.overlay_text.is_empty() {
            out.push(("OverlayText", Entry::Text(self.overlay_text.clone())));
        }
        out
    }
    fn markup_base(&self) -> Option<&crate::public::markup_annotation::MarkupBase> {
        Some(&self.markup)
    }
}
impl HasMarkupBase for RedactionAnnotation {
    fn markup(&self) -> &MarkupBase {
        &self.markup
    }
    fn markup_mut(&mut self) -> &mut MarkupBase {
        &mut self.markup
    }
}
impl Annotation for RedactionAnnotation {
    fn annotation_type(&self) -> AnnotationType {
        AnnotationType::Redaction
    }
}
impl MarkupAnnotation for RedactionAnnotation {}
