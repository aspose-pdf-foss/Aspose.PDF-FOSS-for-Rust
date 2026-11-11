//! `Aspose.Pdf.Annotations.AnnotationSelector` — a visitor over
//! concrete annotation types, per
//! `capabilities/annotation_selector.yaml`.
//!
//! Each shipped concrete annotation that the canonical selector
//! visits exposes a `visit_<type>` method (the canonical
//! `Visit(ConcreteType)` overload). Visiting an annotation —
//! directly, or via [`Annotation::accept`]
//! / [`AnnotationCollection::accept`](crate::AnnotationCollection::accept)
//! — records its [`AnnotationType`] into [`selected`](AnnotationSelector::selected).
//! The `Visit` overload for the not-yet-shipped `PDF3DAnnotation`
//! (deferred) is dropped. The canonical selector carries no
//! `Visit(SoundAnnotation)` overload.

use crate::public::annotation::Annotation;
use crate::public::annotation_type::AnnotationType;
use crate::public::rich_media_annotation::RichMediaAnnotation;
use crate::public::widget_annotation::WidgetAnnotation;
use crate::public::text_annotation::TextAnnotation;
use crate::public::link_annotation::LinkAnnotation;
use crate::public::square_annotation::SquareAnnotation;
use crate::public::circle_annotation::CircleAnnotation;
use crate::public::line_annotation::LineAnnotation;
use crate::public::free_text_annotation::FreeTextAnnotation;
use crate::public::polygon_annotation::PolygonAnnotation;
use crate::public::polyline_annotation::PolylineAnnotation;
use crate::public::highlight_annotation::HighlightAnnotation;
use crate::public::underline_annotation::UnderlineAnnotation;
use crate::public::strike_out_annotation::StrikeOutAnnotation;
use crate::public::squiggly_annotation::SquigglyAnnotation;
use crate::public::ink_annotation::InkAnnotation;
use crate::public::stamp_annotation::StampAnnotation;
use crate::public::popup_annotation::PopupAnnotation;
use crate::public::caret_annotation::CaretAnnotation;
use crate::public::file_attachment_annotation::FileAttachmentAnnotation;
use crate::public::movie_annotation::MovieAnnotation;
use crate::public::screen_annotation::ScreenAnnotation;
use crate::public::redaction_annotation::RedactionAnnotation;
use crate::public::watermark_annotation::WatermarkAnnotation;
use crate::public::color_bar_annotation::ColorBarAnnotation;
use crate::public::trim_mark_annotation::TrimMarkAnnotation;
use crate::public::bleed_mark_annotation::BleedMarkAnnotation;
use crate::public::registration_mark_annotation::RegistrationMarkAnnotation;
use crate::public::page_information_annotation::PageInformationAnnotation;

/// A visitor that records the annotations it visits.
#[derive(Default)]
pub struct AnnotationSelector {
    selected: Vec<AnnotationType>,
}

impl AnnotationSelector {
    /// Construct an empty selector.
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct a selector seeded for a specific annotation. The
    /// annotation is accepted for canonical parity; selection is
    /// recorded as annotations are visited.
    pub fn for_annotation(_annotation: &dyn Annotation) -> Self {
        Self::default()
    }

    /// The types of the annotations visited so far, in visit order.
    pub fn selected(&self) -> &[AnnotationType] {
        &self.selected
    }

    pub(crate) fn record_visit(&mut self, annotation_type: AnnotationType) {
        self.selected.push(annotation_type);
    }

    /// Visit a [`TextAnnotation`].
    pub fn visit_text_annotation(&mut self, annotation: &TextAnnotation) {
        self.record_visit(annotation.annotation_type());
    }
    /// Visit a [`LinkAnnotation`].
    pub fn visit_link_annotation(&mut self, annotation: &LinkAnnotation) {
        self.record_visit(annotation.annotation_type());
    }
    /// Visit a [`SquareAnnotation`].
    pub fn visit_square_annotation(&mut self, annotation: &SquareAnnotation) {
        self.record_visit(annotation.annotation_type());
    }
    /// Visit a [`CircleAnnotation`].
    pub fn visit_circle_annotation(&mut self, annotation: &CircleAnnotation) {
        self.record_visit(annotation.annotation_type());
    }
    /// Visit a [`LineAnnotation`].
    pub fn visit_line_annotation(&mut self, annotation: &LineAnnotation) {
        self.record_visit(annotation.annotation_type());
    }
    /// Visit a [`FreeTextAnnotation`].
    pub fn visit_free_text_annotation(&mut self, annotation: &FreeTextAnnotation) {
        self.record_visit(annotation.annotation_type());
    }
    /// Visit a [`PolygonAnnotation`].
    pub fn visit_polygon_annotation(&mut self, annotation: &PolygonAnnotation) {
        self.record_visit(annotation.annotation_type());
    }
    /// Visit a [`PolylineAnnotation`].
    pub fn visit_polyline_annotation(&mut self, annotation: &PolylineAnnotation) {
        self.record_visit(annotation.annotation_type());
    }
    /// Visit a [`HighlightAnnotation`].
    pub fn visit_highlight_annotation(&mut self, annotation: &HighlightAnnotation) {
        self.record_visit(annotation.annotation_type());
    }
    /// Visit a [`UnderlineAnnotation`].
    pub fn visit_underline_annotation(&mut self, annotation: &UnderlineAnnotation) {
        self.record_visit(annotation.annotation_type());
    }
    /// Visit a [`StrikeOutAnnotation`].
    pub fn visit_strike_out_annotation(&mut self, annotation: &StrikeOutAnnotation) {
        self.record_visit(annotation.annotation_type());
    }
    /// Visit a [`SquigglyAnnotation`].
    pub fn visit_squiggly_annotation(&mut self, annotation: &SquigglyAnnotation) {
        self.record_visit(annotation.annotation_type());
    }
    /// Visit a [`InkAnnotation`].
    pub fn visit_ink_annotation(&mut self, annotation: &InkAnnotation) {
        self.record_visit(annotation.annotation_type());
    }
    /// Visit a [`StampAnnotation`].
    pub fn visit_stamp_annotation(&mut self, annotation: &StampAnnotation) {
        self.record_visit(annotation.annotation_type());
    }
    /// Visit a [`PopupAnnotation`].
    pub fn visit_popup_annotation(&mut self, annotation: &PopupAnnotation) {
        self.record_visit(annotation.annotation_type());
    }
    /// Visit a [`CaretAnnotation`].
    pub fn visit_caret_annotation(&mut self, annotation: &CaretAnnotation) {
        self.record_visit(annotation.annotation_type());
    }
    /// Visit a [`FileAttachmentAnnotation`].
    pub fn visit_file_attachment_annotation(&mut self, annotation: &FileAttachmentAnnotation) {
        self.record_visit(annotation.annotation_type());
    }
    /// Visit a [`MovieAnnotation`].
    pub fn visit_movie_annotation(&mut self, annotation: &MovieAnnotation) {
        self.record_visit(annotation.annotation_type());
    }
    /// Visit a [`ScreenAnnotation`].
    pub fn visit_screen_annotation(&mut self, annotation: &ScreenAnnotation) {
        self.record_visit(annotation.annotation_type());
    }
    /// Visit a [`RedactionAnnotation`].
    pub fn visit_redaction_annotation(&mut self, annotation: &RedactionAnnotation) {
        self.record_visit(annotation.annotation_type());
    }
    /// Visit a [`WatermarkAnnotation`].
    pub fn visit_watermark_annotation(&mut self, annotation: &WatermarkAnnotation) {
        self.record_visit(annotation.annotation_type());
    }
    /// Visit a [`ColorBarAnnotation`].
    pub fn visit_color_bar_annotation(&mut self, annotation: &ColorBarAnnotation) {
        self.record_visit(annotation.annotation_type());
    }
    /// Visit a [`TrimMarkAnnotation`].
    pub fn visit_trim_mark_annotation(&mut self, annotation: &TrimMarkAnnotation) {
        self.record_visit(annotation.annotation_type());
    }
    /// Visit a [`BleedMarkAnnotation`].
    pub fn visit_bleed_mark_annotation(&mut self, annotation: &BleedMarkAnnotation) {
        self.record_visit(annotation.annotation_type());
    }
    /// Visit a [`RegistrationMarkAnnotation`].
    pub fn visit_registration_mark_annotation(&mut self, annotation: &RegistrationMarkAnnotation) {
        self.record_visit(annotation.annotation_type());
    }
    /// Visit a [`PageInformationAnnotation`].
    pub fn visit_page_information_annotation(&mut self, annotation: &PageInformationAnnotation) {
        self.record_visit(annotation.annotation_type());
    }
    /// Visit a [`WidgetAnnotation`].
    pub fn visit_widget_annotation(&mut self, annotation: &WidgetAnnotation) {
        self.record_visit(annotation.annotation_type());
    }
    /// Visit a [`RichMediaAnnotation`].
    pub fn visit_rich_media_annotation(&mut self, annotation: &RichMediaAnnotation) {
        self.record_visit(annotation.annotation_type());
    }
}
