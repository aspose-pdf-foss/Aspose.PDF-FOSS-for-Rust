//! `Aspose.Pdf.Annotations.TextAnnotation` — a "sticky note"
//! annotation, per `capabilities/text_annotation.yaml`.

use crate::public::annotation::sealed::HasBase;
use crate::public::annotation_state::AnnotationState;
use crate::public::annotation_state_model::AnnotationStateModel;
use crate::public::annotation::{Annotation, AnnotationBase};
use crate::public::annotation_type::AnnotationType;
use crate::public::document::Document;
use crate::public::markup_annotation::sealed::HasMarkupBase;
use crate::public::markup_annotation::{MarkupAnnotation, MarkupBase};
use crate::public::page::Page;
use crate::public::rectangle::Rectangle;
use crate::public::text_icon::TextIcon;

/// A text ("sticky note") annotation.
pub struct TextAnnotation {
    base: AnnotationBase,
    markup: MarkupBase,
    open: bool,
    icon: TextIcon,
}

impl TextAnnotation {
    /// Construct within a [`Document`]. The document is accepted for
    /// canonical parity (annotation/document association).
    pub fn new(_document: &Document) -> Self {
        Self {
            base: AnnotationBase::default(),
            markup: MarkupBase::new(),
            open: false,
            icon: TextIcon::Note,
        }
    }

    /// Construct on a [`Page`] at the given rectangle.
    pub fn on_page(page: &Page, rect: Rectangle) -> Self {
        let mut base = AnnotationBase::new(rect);
        base.page_index = page.number() as i32;
        Self {
            base,
            markup: MarkupBase::new(),
            open: false,
            icon: TextIcon::Note,
        }
    }

    /// Whether the note pops open by default.
    pub fn open(&self) -> bool {
        self.open
    }
    /// Set whether the note pops open by default.
    pub fn set_open(&mut self, value: bool) {
        self.open = value;
    }

    /// The note icon.
    pub fn icon(&self) -> TextIcon {
        self.icon
    }
    /// Set the note icon.
    pub fn set_icon(&mut self, value: TextIcon) {
        self.icon = value;
    }
}

impl HasBase for TextAnnotation {
    fn base(&self) -> &AnnotationBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut AnnotationBase {
        &mut self.base
    }
    fn pdf_entries(&self) -> Vec<(&'static str, crate::internal::annotation_io::Entry)> {
        use crate::internal::annotation_io::Entry;
        // §12.5.6.4 Table 172: /Name is the icon a viewer draws for the
        // note — the annotation has no other visible form — and /Open
        // says whether its window starts open. Both default to the
        // values an absent entry already means (/Note, closed).
        let mut out = Vec::new();
        if self.open {
            out.push(("Open", Entry::Bool(true)));
        }
        if self.icon != TextIcon::Note {
            out.push(("Name", Entry::Name(text_icon_name(self.icon).to_string())));
        }
        // The same table carries the review state. /StateModel is
        // required whenever /State is present, so the two are written
        // together or not at all, and an undefined model means the
        // annotation was never given a state.
        if let (Some(model), Some(state)) = (
            state_model_name(self.markup.state_model),
            state_name(self.markup.state),
        ) {
            out.push(("StateModel", Entry::Name(model.to_string())));
            out.push(("State", Entry::Name(state.to_string())));
        }
        out
    }
    fn markup_base(&self) -> Option<&crate::public::markup_annotation::MarkupBase> {
        Some(&self.markup)
    }
}

impl HasMarkupBase for TextAnnotation {
    fn markup(&self) -> &MarkupBase {
        &self.markup
    }
    fn markup_mut(&mut self) -> &mut MarkupBase {
        &mut self.markup
    }
}

impl Annotation for TextAnnotation {
    fn annotation_type(&self) -> AnnotationType {
        AnnotationType::Text
    }
}

impl MarkupAnnotation for TextAnnotation {}

/// The name PDF gives a text annotation's icon (§12.5.6.4). The first
/// seven are the standard set; the rest are the names viewers added for
/// the same purpose, which the table allows.
fn text_icon_name(i: TextIcon) -> &'static str {
    match i {
        TextIcon::Note => "Note",
        TextIcon::Comment => "Comment",
        TextIcon::Key => "Key",
        TextIcon::Help => "Help",
        TextIcon::NewParagraph => "NewParagraph",
        TextIcon::Paragraph => "Paragraph",
        TextIcon::Insert => "Insert",
        TextIcon::Check => "Check",
        TextIcon::Cross => "Cross",
        TextIcon::Circle => "Circle",
        TextIcon::Star => "Star",
    }
}

/// The name PDF gives a review-state model (§12.5.6.4 Table 172).
fn state_model_name(m: AnnotationStateModel) -> Option<&'static str> {
    match m {
        AnnotationStateModel::Undefined => None,
        AnnotationStateModel::Marked => Some("Marked"),
        AnnotationStateModel::Review => Some("Review"),
    }
}

/// The name PDF gives a review state. `Undefined` is the absence of a
/// state; `None` is the state literally named "None", which the Review
/// model lists alongside Accepted and the rest.
fn state_name(s: AnnotationState) -> Option<&'static str> {
    match s {
        AnnotationState::Undefined => None,
        AnnotationState::Marked => Some("Marked"),
        AnnotationState::Unmarked => Some("Unmarked"),
        AnnotationState::Accepted => Some("Accepted"),
        AnnotationState::Rejected => Some("Rejected"),
        AnnotationState::Cancelled => Some("Cancelled"),
        AnnotationState::Completed => Some("Completed"),
        AnnotationState::None => Some("None"),
    }
}
