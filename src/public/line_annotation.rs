//! `Aspose.Pdf.Annotations.LineAnnotation` — a line annotation
//! (optionally a measurement/dimension line), per
//! `capabilities/line_annotation.yaml`.
//!
//! Extends [`MarkupAnnotation`]. The abstract-base `Accept` /
//! `ChangeAfterResize` members are dropped (see
//! `targets/rust/drops.yaml`).
//!
//! §12.5.6.7 Table 177 is written on save, `/Measure` included: the
//! [`Measure`] parameters reach the file as the measurement dictionary
//! §12.9 defines, so a dimension line carries the scale it is measured
//! against.
//!
//! [`Measure`]: crate::Measure

use crate::public::annotation::sealed::HasBase;
use crate::public::annotation::{Annotation, AnnotationBase};
use crate::public::annotation_type::AnnotationType;
use crate::public::caption_position::CaptionPosition;
use crate::public::color::Color;
use crate::public::document::Document;
use crate::public::line_ending::LineEnding;
use crate::public::line_intent::LineIntent;
use crate::public::markup_annotation::sealed::HasMarkupBase;
use crate::public::markup_annotation::{MarkupAnnotation, MarkupBase};
use crate::public::measure::Measure;
use crate::public::page::Page;
use crate::public::point::Point;
use crate::public::rectangle::Rectangle;

/// A line annotation between two endpoints.
pub struct LineAnnotation {
    base: AnnotationBase,
    markup: MarkupBase,
    starting: Point,
    ending: Point,
    starting_style: LineEnding,
    ending_style: LineEnding,
    interior_color: Color,
    leader_line: f64,
    leader_line_extension: f64,
    leader_line_offset: f64,
    show_caption: bool,
    caption_offset: Point,
    caption_position: CaptionPosition,
    intent: LineIntent,
    measure: Measure,
}

impl LineAnnotation {
    fn build(base: AnnotationBase, starting: Point, ending: Point) -> Self {
        Self {
            base,
            markup: MarkupBase::new(),
            starting,
            ending,
            starting_style: LineEnding::None,
            ending_style: LineEnding::None,
            interior_color: Color::empty(),
            leader_line: 0.0,
            leader_line_extension: 0.0,
            leader_line_offset: 0.0,
            show_caption: false,
            caption_offset: Point::new(0.0, 0.0),
            caption_position: CaptionPosition::Inline,
            intent: LineIntent::Undefined,
            measure: Measure::default(),
        }
    }

    /// Construct within a [`Document`] between two points.
    pub fn new(_document: &Document, starting: Point, ending: Point) -> Self {
        Self::build(AnnotationBase::default(), starting, ending)
    }

    /// Construct on a [`Page`] with a bounding rectangle and two
    /// endpoints.
    pub fn on_page(page: &Page, rect: Rectangle, starting: Point, ending: Point) -> Self {
        let mut base = AnnotationBase::new(rect);
        base.page_index = page.number() as i32;
        Self::build(base, starting, ending)
    }

    /// The line's starting point.
    pub fn starting(&self) -> Point {
        self.starting
    }
    /// Set the line's starting point.
    pub fn set_starting(&mut self, value: Point) {
        self.starting = value;
    }

    /// The line's ending point.
    pub fn ending(&self) -> Point {
        self.ending
    }
    /// Set the line's ending point.
    pub fn set_ending(&mut self, value: Point) {
        self.ending = value;
    }

    /// The line ending style at the start.
    pub fn starting_style(&self) -> LineEnding {
        self.starting_style
    }
    /// Set the start ending style.
    pub fn set_starting_style(&mut self, value: LineEnding) {
        self.starting_style = value;
    }

    /// The line ending style at the end.
    pub fn ending_style(&self) -> LineEnding {
        self.ending_style
    }
    /// Set the end ending style.
    pub fn set_ending_style(&mut self, value: LineEnding) {
        self.ending_style = value;
    }

    /// The interior fill colour of the line endings.
    pub fn interior_color(&self) -> Color {
        self.interior_color
    }
    /// Set the interior fill colour.
    pub fn set_interior_color(&mut self, value: Color) {
        self.interior_color = value;
    }

    /// The leader-line length.
    pub fn leader_line(&self) -> f64 {
        self.leader_line
    }
    /// Set the leader-line length.
    pub fn set_leader_line(&mut self, value: f64) {
        self.leader_line = value;
    }

    /// The leader-line extension length.
    pub fn leader_line_extension(&self) -> f64 {
        self.leader_line_extension
    }
    /// Set the leader-line extension length.
    pub fn set_leader_line_extension(&mut self, value: f64) {
        self.leader_line_extension = value;
    }

    /// The leader-line offset.
    pub fn leader_line_offset(&self) -> f64 {
        self.leader_line_offset
    }
    /// Set the leader-line offset.
    pub fn set_leader_line_offset(&mut self, value: f64) {
        self.leader_line_offset = value;
    }

    /// Whether the measurement caption is shown.
    pub fn show_caption(&self) -> bool {
        self.show_caption
    }
    /// Set whether the measurement caption is shown.
    pub fn set_show_caption(&mut self, value: bool) {
        self.show_caption = value;
    }

    /// The caption offset.
    pub fn caption_offset(&self) -> Point {
        self.caption_offset
    }
    /// Set the caption offset.
    pub fn set_caption_offset(&mut self, value: Point) {
        self.caption_offset = value;
    }

    /// The caption position relative to the line.
    pub fn caption_position(&self) -> CaptionPosition {
        self.caption_position
    }
    /// Set the caption position.
    pub fn set_caption_position(&mut self, value: CaptionPosition) {
        self.caption_position = value;
    }

    /// The line intent (plain / arrow / dimension).
    pub fn intent(&self) -> LineIntent {
        self.intent
    }
    /// Set the line intent.
    pub fn set_intent(&mut self, value: LineIntent) {
        self.intent = value;
    }
}

impl HasBase for LineAnnotation {
    fn base(&self) -> &AnnotationBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut AnnotationBase {
        &mut self.base
    }
    fn pdf_entries(&self) -> Vec<(&'static str, crate::internal::annotation_io::Entry)> {
        use crate::internal::annotation_io::Entry;
        // §12.5.6.7: /L is the line's two endpoints, /LE the names of
        // the styles drawn at each. Without /L a viewer has no line.
        let mut out = vec![(
            "L",
            Entry::Reals(vec![
                self.starting.x(),
                self.starting.y(),
                self.ending.x(),
                self.ending.y(),
            ]),
        )];
        if self.starting_style != LineEnding::None || self.ending_style != LineEnding::None {
            out.push((
                "LE",
                Entry::Names(vec![
                    line_ending_name(self.starting_style).to_string(),
                    line_ending_name(self.ending_style).to_string(),
                ]),
            ));
        }
        // The rest of Table 177. /IC fills the line endings — a
        // transparent colour is the absence of a fill, not black. The
        // leader lines (/LL, /LLE, /LLO) default to zero, which is what
        // "no leader line" is written as, so each is left out at zero.
        if !self.interior_color.is_empty() {
            let (r, g, b) = self.interior_color.rgb();
            out.push(("IC", Entry::Reals(vec![r, g, b])));
        }
        if self.leader_line != 0.0 {
            out.push(("LL", Entry::Real(self.leader_line)));
        }
        if self.leader_line_extension != 0.0 {
            out.push(("LLE", Entry::Real(self.leader_line_extension)));
        }
        if self.leader_line_offset != 0.0 {
            out.push(("LLO", Entry::Real(self.leader_line_offset)));
        }
        // /CP and /CO are meaningful only when /Cap is true (§12.5.6.7),
        // so they follow the caption rather than standing on their own.
        if self.show_caption {
            out.push(("Cap", Entry::Bool(true)));
            if self.caption_position == CaptionPosition::Top {
                out.push(("CP", Entry::Name("Top".to_string())));
            }
            if self.caption_offset.x() != 0.0 || self.caption_offset.y() != 0.0 {
                out.push((
                    "CO",
                    Entry::Reals(vec![self.caption_offset.x(), self.caption_offset.y()]),
                ));
            }
        }
        if let Some(name) = line_intent_name(self.intent) {
            out.push(("IT", Entry::Name(name.to_string())));
        }
        // §12.9's measurement parameters, as the plain dictionary
        // Table 261 defines — written only when they say something.
        if let Some(measure) = crate::public::measure::measure_entries(&self.measure) {
            out.push(("Measure", measure));
        }
        out
    }
    fn markup_base(&self) -> Option<&crate::public::markup_annotation::MarkupBase> {
        Some(&self.markup)
    }
}

impl HasMarkupBase for LineAnnotation {
    fn markup(&self) -> &MarkupBase {
        &self.markup
    }
    fn markup_mut(&mut self) -> &mut MarkupBase {
        &mut self.markup
    }
}

impl LineAnnotation {
    /// The measurement parameters of this dimension line (canonical
    /// `Measure`).
    pub fn measure(&self) -> &Measure {
        &self.measure
    }
    /// Set the measurement parameters.
    pub fn set_measure(&mut self, value: Measure) {
        self.measure = value;
    }
}

impl Annotation for LineAnnotation {
    fn annotation_type(&self) -> AnnotationType {
        AnnotationType::Line
    }
}

impl MarkupAnnotation for LineAnnotation {}

/// The name PDF gives a line ending (§12.5.6.7 Table 176).
pub(crate) fn line_ending_name(e: LineEnding) -> &'static str {
    match e {
        LineEnding::None => "None",
        LineEnding::Square => "Square",
        LineEnding::Circle => "Circle",
        LineEnding::Diamond => "Diamond",
        LineEnding::OpenArrow => "OpenArrow",
        LineEnding::ClosedArrow => "ClosedArrow",
        LineEnding::Butt => "Butt",
        LineEnding::ROpenArrow => "ROpenArrow",
        LineEnding::RClosedArrow => "RClosedArrow",
        LineEnding::Slash => "Slash",
    }
}

/// The name PDF gives a line annotation's intent (§12.5.6.7). An
/// undefined intent is the file saying nothing, so it has no name.
pub(crate) fn line_intent_name(i: LineIntent) -> Option<&'static str> {
    match i {
        LineIntent::Undefined => None,
        LineIntent::LineArrow => Some("LineArrow"),
        LineIntent::LineDimension => Some("LineDimension"),
    }
}
