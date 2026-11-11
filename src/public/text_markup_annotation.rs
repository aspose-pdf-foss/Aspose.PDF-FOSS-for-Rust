//! `Aspose.Pdf.Annotations.TextMarkupAnnotation` — the intermediate
//! base for text-markup annotations (highlight / underline /
//! strike-out / squiggly), per
//! `capabilities/text_markup_annotation.yaml`.
//!
//! Modelled as the [`TextMarkupAnnotation`] trait (extending
//! [`MarkupAnnotation`]) over a shared `TextMarkupBase`.
//! `ChangeAfterResize` (`Aspose.Pdf.Matrix`) is dropped.

use crate::public::markup_annotation::MarkupAnnotation;
use crate::public::point::Point;
use crate::public::text_fragment::TextFragment;
use crate::public::text_fragment_collection::TextFragmentCollection;

/// Shared text-markup fields. Crate-internal.
pub(crate) struct TextMarkupBase {
    pub(crate) quad_points: Vec<Point>,
    pub(crate) marked_text: String,
}

impl TextMarkupBase {
    pub(crate) fn new() -> Self {
        Self {
            quad_points: Vec::new(),
            marked_text: String::new(),
        }
    }
}

impl Default for TextMarkupBase {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) mod sealed {
    use super::TextMarkupBase;

    /// Text-markup-base access for concrete text-markup annotations.
    pub(crate) trait HasTextMarkupBase {
        /// The shared text-markup base.
        fn text_markup(&self) -> &TextMarkupBase;
        /// The shared text-markup base, mutably.
        fn text_markup_mut(&mut self) -> &mut TextMarkupBase;
    }
}

/// A text-markup annotation (highlight / underline / strike-out /
/// squiggly) defined by its quadrilateral regions.
#[allow(private_bounds)]
pub trait TextMarkupAnnotation: MarkupAnnotation + sealed::HasTextMarkupBase {
    /// The marked quadrilaterals (groups of four points per marked
    /// region).
    fn quad_points(&self) -> &[Point] {
        &self.text_markup().quad_points
    }
    /// Set the marked quadrilaterals.
    fn set_quad_points(&mut self, value: Vec<Point>) {
        self.text_markup_mut().quad_points = value;
    }

    /// The marked text covered by the annotation.
    fn get_marked_text(&self) -> String {
        self.text_markup().marked_text.clone()
    }

    /// The marked text as a [`TextFragmentCollection`]. Mirrors
    /// [`get_marked_text`](Self::get_marked_text): the marked run is
    /// exposed as fragments (empty text yields an empty collection).
    fn get_marked_text_fragments(&self) -> TextFragmentCollection {
        let mut fragments = TextFragmentCollection::default();
        let marked = &self.text_markup().marked_text;
        if !marked.is_empty() {
            fragments.push(TextFragment::with_text(marked.clone()));
        }
        fragments
    }
}
