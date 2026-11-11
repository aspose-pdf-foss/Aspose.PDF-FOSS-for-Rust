//! `Aspose.Pdf.Annotations.MarkupAnnotation` — the intermediate
//! base for annotations that carry author markup (title, subject,
//! opacity, review state), per `capabilities/markup_annotation.yaml`.
//!
//! Modelled as the [`MarkupAnnotation`] trait (extending
//! [`Annotation`]) backed by a shared
//! `MarkupBase` field carrier, mirroring the annotation-base
//! sealed pattern. The canonical `InReplyTo` (recursive `Annotation`
//! ref) and `Popup`
//! (`PopupAnnotation`, A-4) members are dropped, as is the abstract
//! `.ctor` (the Rust trait has no constructor); see
//! `targets/rust/drops.yaml`.
//!
//! The markup entries §12.5.6.2 gives every subtype — `/T`, `/Subj`,
//! `/RC`, `/RT`, `/CA` — are written on save.
//!
//! **Carried, not written on most subtypes.** The review state lives
//! here because the canonical surface puts it on the markup base, but
//! `/State` and `/StateModel` are entries of the *text* annotation's
//! table (§12.5.6.4), so they are written for
//! [`TextAnnotation`](crate::TextAnnotation) and carried without an
//! entry elsewhere. `creation_date` is written as `/CreationDate`, and
//! only when it was given: [`DateTime`] formats a PDF date and answers
//! nothing for a date nobody set, which is what keeps an annotation
//! from being stamped with a time the caller never stated.
//!
//! [`DateTime`]: crate::DateTime

use crate::public::annotation::Annotation;
use crate::public::annotation_state::AnnotationState;
use crate::public::annotation_state_model::AnnotationStateModel;
use crate::public::date_time::DateTime;
use crate::public::reply_type::ReplyType;

/// Shared markup fields. Crate-internal — the Rust mechanism for
/// sharing the markup base's properties across concrete markup
/// annotations.
pub(crate) struct MarkupBase {
    pub(crate) title: String,
    pub(crate) rich_text: String,
    pub(crate) subject: String,
    pub(crate) opacity: f64,
    pub(crate) reply_type: ReplyType,
    pub(crate) state: AnnotationState,
    pub(crate) state_model: AnnotationStateModel,
    pub(crate) creation_date: DateTime,
}

impl MarkupBase {
    /// A markup base with the PDF defaults (opacity `1.0`, reply
    /// type `Reply`, no review state).
    pub(crate) fn new() -> Self {
        Self {
            title: String::new(),
            rich_text: String::new(),
            subject: String::new(),
            opacity: 1.0,
            reply_type: ReplyType::Reply,
            state: AnnotationState::None,
            state_model: AnnotationStateModel::Undefined,
            creation_date: DateTime::default(),
        }
    }
}

impl Default for MarkupBase {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) mod sealed {
    use super::MarkupBase;

    /// Markup-base access for concrete markup annotations.
    pub(crate) trait HasMarkupBase {
        /// The shared markup base.
        fn markup(&self) -> &MarkupBase;
        /// The shared markup base, mutably.
        fn markup_mut(&mut self) -> &mut MarkupBase;
    }
}

/// An annotation that carries author markup (title / subject /
/// opacity / review state).
///
/// Concrete markup annotations implement the crate-private markup
/// accessor; the property accessors below are trait defaults over
/// the shared `MarkupBase`.
#[allow(private_bounds)]
pub trait MarkupAnnotation: Annotation + sealed::HasMarkupBase {
    /// The creation date (canonical `CreationDate`).
    fn creation_date(&self) -> DateTime {
        self.markup().creation_date
    }
    /// Set the creation date.
    fn set_creation_date(&mut self, value: DateTime) {
        self.markup_mut().creation_date = value;
    }

    /// The markup title (author).
    fn title(&self) -> &str {
        &self.markup().title
    }
    /// Set the markup title.
    fn set_title(&mut self, value: String) {
        self.markup_mut().title = value;
    }

    /// The rich-text contents.
    fn rich_text(&self) -> &str {
        &self.markup().rich_text
    }
    /// Set the rich-text contents.
    fn set_rich_text(&mut self, value: String) {
        self.markup_mut().rich_text = value;
    }

    /// The subject.
    fn subject(&self) -> &str {
        &self.markup().subject
    }
    /// Set the subject.
    fn set_subject(&mut self, value: String) {
        self.markup_mut().subject = value;
    }

    /// The opacity (`0.0`–`1.0`).
    fn opacity(&self) -> f64 {
        self.markup().opacity
    }
    /// Set the opacity.
    fn set_opacity(&mut self, value: f64) {
        self.markup_mut().opacity = value;
    }

    /// The reply relationship.
    fn reply_type(&self) -> ReplyType {
        self.markup().reply_type
    }
    /// Set the reply relationship.
    fn set_reply_type(&mut self, value: ReplyType) {
        self.markup_mut().reply_type = value;
    }

    /// Clear the review state.
    fn clear_state(&mut self) {
        let m = self.markup_mut();
        m.state = AnnotationState::None;
        m.state_model = AnnotationStateModel::Undefined;
    }

    /// Set the review state, attributed to `author`. The author is
    /// accepted for canonical parity.
    fn set_review_state(&mut self, state: AnnotationState, _author: String) {
        let m = self.markup_mut();
        m.state = state;
        m.state_model = AnnotationStateModel::Review;
    }

    /// Set the review state without an author attribution.
    fn set_review_state_value(&mut self, state: AnnotationState) {
        let m = self.markup_mut();
        m.state = state;
        m.state_model = AnnotationStateModel::Review;
    }

    /// Set the marked state (`true` → marked, `false` → unmarked).
    fn set_marked_state(&mut self, marked: bool) {
        let m = self.markup_mut();
        m.state = if marked {
            AnnotationState::Marked
        } else {
            AnnotationState::Unmarked
        };
        m.state_model = AnnotationStateModel::Marked;
    }

    /// The current review state.
    fn get_state(&self) -> AnnotationState {
        self.markup().state
    }

    /// The current state model.
    fn get_state_model(&self) -> AnnotationStateModel {
        self.markup().state_model
    }
}
