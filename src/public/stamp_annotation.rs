//! `Aspose.Pdf.Annotations.StampAnnotation` — a rubber-stamp
//! annotation, per `capabilities/stamp_annotation.yaml`.
//!
//! Extends [`MarkupAnnotation`]. The canonical `Image`
//! (`System.IO.Stream`) maps to owned bytes (read via `image`,
//! written via `set_image`).
//!
//! **Carried, not written.** `/Name` reaches the file, so a viewer
//! draws the named stamp. A custom `image` does not: a stamp image is
//! an appearance stream (`/AP /N`), a second object holding the image
//! XObject, and the annotation writer emits one dictionary per
//! annotation. The bytes stay in memory and the named stamp is drawn
//! instead.

use crate::public::annotation::sealed::HasBase;
use crate::public::annotation::{Annotation, AnnotationBase};
use crate::public::annotation_type::AnnotationType;
use crate::public::document::Document;
use crate::public::markup_annotation::sealed::HasMarkupBase;
use crate::public::markup_annotation::{MarkupAnnotation, MarkupBase};
use crate::public::page::Page;
use crate::public::rectangle::Rectangle;
use crate::public::stamp_icon::StampIcon;

/// A rubber-stamp annotation.
pub struct StampAnnotation {
    base: AnnotationBase,
    markup: MarkupBase,
    icon: StampIcon,
    image: Vec<u8>,
}

impl StampAnnotation {
    /// Construct within a [`Document`].
    pub fn new(_document: &Document) -> Self {
        Self {
            base: AnnotationBase::default(),
            markup: MarkupBase::new(),
            icon: StampIcon::Draft,
            image: Vec::new(),
        }
    }

    /// Construct on a [`Page`] at the given rectangle.
    pub fn on_page(page: &Page, rect: Rectangle) -> Self {
        let mut base = AnnotationBase::new(rect);
        base.page_index = page.number() as i32;
        Self {
            base,
            markup: MarkupBase::new(),
            icon: StampIcon::Draft,
            image: Vec::new(),
        }
    }

    /// The stamp icon.
    pub fn icon(&self) -> StampIcon {
        self.icon
    }
    /// Set the stamp icon.
    pub fn set_icon(&mut self, value: StampIcon) {
        self.icon = value;
    }

    /// The custom stamp image bytes (empty when none is set).
    ///
    /// Carried, not written: a stamp draws from its appearance stream,
    /// so a custom image has to be an image XObject inside one, and an
    /// annotation here is written as a dictionary with no `/AP`. The
    /// named [`icon`](StampAnnotation::icon) does reach the file as
    /// `/Name`, which is the stamp a viewer draws from its own set.
    pub fn image(&self) -> Vec<u8> {
        self.image.clone()
    }
    /// Set the custom stamp image bytes.
    pub fn set_image(&mut self, image: &[u8]) {
        self.image = image.to_vec();
    }
}

impl HasBase for StampAnnotation {
    fn base(&self) -> &AnnotationBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut AnnotationBase {
        &mut self.base
    }
    fn pdf_entries(&self) -> Vec<(&'static str, crate::internal::annotation_io::Entry)> {
        use crate::internal::annotation_io::Entry;
        // §12.5.6.12: /Name is the stamp a viewer draws. /Draft is the
        // default, which an absent entry already means.
        if self.icon != StampIcon::Draft {
            vec![("Name", Entry::Name(stamp_icon_name(self.icon).to_string()))]
        } else {
            Vec::new()
        }
    }
    fn markup_base(&self) -> Option<&crate::public::markup_annotation::MarkupBase> {
        Some(&self.markup)
    }
}
impl HasMarkupBase for StampAnnotation {
    fn markup(&self) -> &MarkupBase {
        &self.markup
    }
    fn markup_mut(&mut self) -> &mut MarkupBase {
        &mut self.markup
    }
}
impl Annotation for StampAnnotation {
    fn annotation_type(&self) -> AnnotationType {
        AnnotationType::Stamp
    }
}
impl MarkupAnnotation for StampAnnotation {}

/// The name PDF gives a rubber stamp (§12.5.6.12 Table 181).
fn stamp_icon_name(i: StampIcon) -> &'static str {
    match i {
        StampIcon::Draft => "Draft",
        StampIcon::Approved => "Approved",
        StampIcon::Experimental => "Experimental",
        StampIcon::NotApproved => "NotApproved",
        StampIcon::AsIs => "AsIs",
        StampIcon::Expired => "Expired",
        StampIcon::NotForPublicRelease => "NotForPublicRelease",
        StampIcon::Confidential => "Confidential",
        StampIcon::Final => "Final",
        StampIcon::Sold => "Sold",
        StampIcon::Departmental => "Departmental",
        StampIcon::ForComment => "ForComment",
        StampIcon::ForPublicRelease => "ForPublicRelease",
        StampIcon::TopSecret => "TopSecret",
    }
}
