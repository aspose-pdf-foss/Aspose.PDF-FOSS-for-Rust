//! `Aspose.Pdf.Annotations.FreeTextAnnotation` — a free-text
//! ("typewriter") annotation, per
//! `capabilities/free_text_annotation.yaml`.
//!
//! Extends [`MarkupAnnotation`]. The colour-taking `SetTextStyle`
//! overload (`System.Drawing.Color`) is dropped; `TextStyle` and
//! `Accept` ship.
//!
//! §12.5.6.6 Table 174 is written on save — `/DA`, `/Q`, `/DS`, `/IT`,
//! `/CL`, `/LE`, `/RD`.
//!
//! **Carried, not written.** Three properties have no entry in that
//! table to reach: `ending_style` (the table gives `/LE` as a single
//! name for the callout's first point — only a line annotation carries
//! an ending for each end), `rotate` (a rotation of the generated
//! appearance, which lives in that stream's matrix, and this crate
//! generates no appearance stream) and `text_style` (a parallel view of
//! the font, size and colour that `/DA` already encodes; `/DA` is
//! written from `default_appearance`, which the constructor derives
//! from the `DefaultAppearance` the annotation was built with).

use crate::public::annotation::sealed::HasBase;
use crate::public::annotation::{Annotation, AnnotationBase};
use crate::public::annotation_type::AnnotationType;
use crate::public::default_appearance::DefaultAppearance;
use crate::public::document::Document;
use crate::public::free_text_intent::FreeTextIntent;
use crate::public::justification::Justification;
use crate::public::line_ending::LineEnding;
use crate::public::markup_annotation::sealed::HasMarkupBase;
use crate::public::markup_annotation::{MarkupAnnotation, MarkupBase};
use crate::public::page::Page;
use crate::public::point::Point;
use crate::public::rectangle::Rectangle;
use crate::public::rich_text_font_styles::RichTextFontStyles;
use crate::public::rotation::Rotation;
use crate::public::text_style::TextStyle;
use crate::internal::pending::PENDING;

/// A free-text annotation rendered with a default appearance.
pub struct FreeTextAnnotation {
    base: AnnotationBase,
    markup: MarkupBase,
    default_appearance_obj: DefaultAppearance,
    default_appearance: String,
    default_style: String,
    starting_style: LineEnding,
    ending_style: LineEnding,
    justification: Justification,
    intent: FreeTextIntent,
    rotate: Rotation,
    callout: Vec<Point>,
    text_rectangle: Rectangle,
    text_style: TextStyle,
}

impl FreeTextAnnotation {
    fn build(base: AnnotationBase, appearance: DefaultAppearance) -> Self {
        let default_appearance = appearance.text();
        let default_style = format!("font: {}", appearance.font_name());
        Self {
            base,
            markup: MarkupBase::new(),
            default_appearance_obj: appearance,
            default_appearance,
            default_style,
            starting_style: LineEnding::None,
            ending_style: LineEnding::None,
            justification: Justification::Left,
            intent: FreeTextIntent::Undefined,
            rotate: Rotation::None,
            callout: Vec::new(),
            text_rectangle: Rectangle::trivial(),
            text_style: TextStyle::default(),
        }
    }

    /// Construct within a [`Document`] with a default appearance.
    pub fn new(_document: &Document, appearance: DefaultAppearance) -> Self {
        Self::build(AnnotationBase::default(), appearance)
    }

    /// Construct on a [`Page`] at the given rectangle with a default
    /// appearance.
    pub fn on_page(page: &Page, rect: Rectangle, appearance: DefaultAppearance) -> Self {
        let mut base = AnnotationBase::new(rect);
        base.page_index = page.number() as i32;
        Self::build(base, appearance)
    }

    /// Apply a font style to the character range `[start, end)`.
    /// Recorded for appearance regeneration (a no-op in the in-memory
    /// model).
    ///
    /// Not implemented — pending `annotation_appearance_streams`.
    /// Calling this panics.
    pub fn set_text_style_range(&mut self, _start: i32, _end: i32, _styles: RichTextFontStyles) {
        PENDING("annotation_appearance_streams")
    }

    /// The starting line-ending style (for callout lines).
    pub fn starting_style(&self) -> LineEnding {
        self.starting_style
    }
    /// Set the starting line-ending style.
    pub fn set_starting_style(&mut self, value: LineEnding) {
        self.starting_style = value;
    }

    /// The ending line-ending style.
    pub fn ending_style(&self) -> LineEnding {
        self.ending_style
    }
    /// Set the ending line-ending style.
    pub fn set_ending_style(&mut self, value: LineEnding) {
        self.ending_style = value;
    }

    /// The text justification.
    pub fn justification(&self) -> Justification {
        self.justification
    }
    /// Set the text justification.
    pub fn set_justification(&mut self, value: Justification) {
        self.justification = value;
    }

    /// The default-appearance (`/DA`) operator string.
    pub fn default_appearance(&self) -> &str {
        &self.default_appearance
    }
    /// Set the default-appearance string.
    pub fn set_default_appearance(&mut self, value: String) {
        self.default_appearance = value;
    }

    /// The default-appearance object the annotation was built with.
    pub fn default_appearance_object(&self) -> &DefaultAppearance {
        &self.default_appearance_obj
    }

    /// The free-text intent (plain / callout / typewriter).
    pub fn intent(&self) -> FreeTextIntent {
        self.intent
    }
    /// Set the free-text intent.
    pub fn set_intent(&mut self, value: FreeTextIntent) {
        self.intent = value;
    }

    /// The default rich-text style string.
    pub fn default_style(&self) -> &str {
        &self.default_style
    }
    /// Set the default rich-text style string.
    pub fn set_default_style(&mut self, value: String) {
        self.default_style = value;
    }

    /// The appearance rotation.
    pub fn rotate(&self) -> Rotation {
        self.rotate
    }
    /// Set the appearance rotation.
    pub fn set_rotate(&mut self, value: Rotation) {
        self.rotate = value;
    }

    /// The callout line points.
    pub fn callout(&self) -> &[Point] {
        &self.callout
    }
    /// Set the callout line points.
    pub fn set_callout(&mut self, value: Vec<Point>) {
        self.callout = value;
    }

    /// The inner text rectangle (the text region inset within
    /// `Rect`).
    pub fn text_rectangle(&self) -> Rectangle {
        self.text_rectangle
    }
    /// Set the inner text rectangle.
    pub fn set_text_rectangle(&mut self, value: Rectangle) {
        self.text_rectangle = value;
    }

    /// The text styling (font name / size / alignment).
    pub fn text_style(&self) -> &TextStyle {
        &self.text_style
    }
    /// Set the text styling.
    pub fn set_text_style(&mut self, value: TextStyle) {
        self.text_style = value;
    }
}

impl HasBase for FreeTextAnnotation {
    fn base(&self) -> &AnnotationBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut AnnotationBase {
        &mut self.base
    }
    fn markup_base(&self) -> Option<&crate::public::markup_annotation::MarkupBase> {
        Some(&self.markup)
    }
    fn pdf_entries(&self) -> Vec<(&'static str, crate::internal::annotation_io::Entry)> {
        use crate::internal::annotation_io::Entry;
        // §12.5.6.6 Table 174. /DA is the one entry the table marks
        // REQUIRED: it is the operator string a viewer sets the text
        // with, so without it there is nothing to draw the contents in.
        let mut out = Vec::new();
        if !self.default_appearance.is_empty() {
            out.push(("DA", Entry::Text(self.default_appearance.clone())));
        }
        if self.justification != Justification::Left {
            // /Q quadding: 0 left, 1 centred, 2 right.
            out.push(("Q", Entry::Int(self.justification as i64)));
        }
        if !self.default_style.is_empty() {
            out.push(("DS", Entry::Text(self.default_style.clone())));
        }
        if let Some(intent) = free_text_intent_name(self.intent) {
            out.push(("IT", Entry::Name(intent.to_string())));
        }
        // /CL is the callout line — two points, or three when it bends.
        // /LE is meaningful only with it, and is a single name for the
        // ending drawn at the line's first point, not the two-name
        // array a line annotation carries.
        if !self.callout.is_empty() {
            let flat = self.callout.iter().flat_map(|p| [p.x(), p.y()]).collect();
            out.push(("CL", Entry::Reals(flat)));
            if self.starting_style != LineEnding::None {
                out.push((
                    "LE",
                    Entry::Name(
                        crate::public::line_annotation::line_ending_name(self.starting_style)
                            .to_string(),
                    ),
                ));
            }
        }
        // /RD: the four differences between Rect and the rectangle the
        // text is laid out in, as left, bottom, right and top insets.
        // All-zero is the text filling Rect, which an absent /RD says.
        let tr = self.text_rectangle;
        if tr.llx() != 0.0 || tr.lly() != 0.0 || tr.urx() != 0.0 || tr.ury() != 0.0 {
            out.push(("RD", Entry::Reals(vec![tr.llx(), tr.lly(), tr.urx(), tr.ury()])));
        }
        out
    }
}

/// The name PDF gives a free-text annotation's intent (§12.5.6.6). An
/// undefined intent is the plain `/FreeText` default, which an absent
/// entry already means.
fn free_text_intent_name(i: FreeTextIntent) -> Option<&'static str> {
    match i {
        FreeTextIntent::Undefined => None,
        FreeTextIntent::FreeTextCallout => Some("FreeTextCallout"),
        FreeTextIntent::FreeTextTypeWriter => Some("FreeTextTypeWriter"),
    }
}

impl HasMarkupBase for FreeTextAnnotation {
    fn markup(&self) -> &MarkupBase {
        &self.markup
    }
    fn markup_mut(&mut self) -> &mut MarkupBase {
        &mut self.markup
    }
}

impl Annotation for FreeTextAnnotation {
    fn annotation_type(&self) -> AnnotationType {
        AnnotationType::FreeText
    }
}

impl MarkupAnnotation for FreeTextAnnotation {}
