//! `Aspose.Pdf.Annotations.CaretAnnotation` — a caret (text-edit
//! insertion mark) annotation, per
//! `capabilities/caret_annotation.yaml`.

use crate::public::annotation::sealed::HasBase;
use crate::public::annotation::{Annotation, AnnotationBase};
use crate::public::annotation_type::AnnotationType;
use crate::public::caret_symbol::CaretSymbol;
use crate::public::document::Document;
use crate::public::markup_annotation::sealed::HasMarkupBase;
use crate::public::markup_annotation::{MarkupAnnotation, MarkupBase};
use crate::public::page::Page;
use crate::public::rectangle::Rectangle;

/// A caret annotation marking a text-insertion point.
pub struct CaretAnnotation {
    base: AnnotationBase,
    markup: MarkupBase,
    frame: Rectangle,
    symbol: CaretSymbol,
}

impl CaretAnnotation {
    /// Construct within a [`Document`].
    pub fn new(_document: &Document) -> Self {
        Self {
            base: AnnotationBase::default(),
            markup: MarkupBase::new(),
            frame: Rectangle::trivial(),
            symbol: CaretSymbol::None,
        }
    }

    /// Construct on a [`Page`] at the given rectangle.
    pub fn on_page(page: &Page, rect: Rectangle) -> Self {
        let mut base = AnnotationBase::new(rect);
        base.page_index = page.number() as i32;
        Self {
            base,
            markup: MarkupBase::new(),
            frame: Rectangle::trivial(),
            symbol: CaretSymbol::None,
        }
    }

    /// The padding frame between `Rect` and the drawn caret.
    pub fn frame(&self) -> Rectangle {
        self.frame
    }
    /// Set the padding frame.
    pub fn set_frame(&mut self, value: Rectangle) {
        self.frame = value;
    }

    /// The caret symbol drawn alongside the mark.
    pub fn symbol(&self) -> CaretSymbol {
        self.symbol
    }
    /// Set the caret symbol.
    pub fn set_symbol(&mut self, value: CaretSymbol) {
        self.symbol = value;
    }
}

impl HasBase for CaretAnnotation {
    fn base(&self) -> &AnnotationBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut AnnotationBase {
        &mut self.base
    }
    fn pdf_entries(&self) -> Vec<(&'static str, crate::internal::annotation_io::Entry)> {
        use crate::internal::annotation_io::Entry;
        // §12.5.6.11: /RD is the four differences between Rect and the
        // caret itself — left, bottom, right and top insets — and /Sy
        // the symbol drawn with it, where /P marks a new paragraph.
        let mut out = Vec::new();
        let f = self.frame;
        if f.llx() != 0.0 || f.lly() != 0.0 || f.urx() != 0.0 || f.ury() != 0.0 {
            out.push(("RD", Entry::Reals(vec![f.llx(), f.lly(), f.urx(), f.ury()])));
        }
        if self.symbol == CaretSymbol::Paragraph {
            out.push(("Sy", Entry::Name("P".to_string())));
        }
        out
    }
    fn markup_base(&self) -> Option<&crate::public::markup_annotation::MarkupBase> {
        Some(&self.markup)
    }
}
impl HasMarkupBase for CaretAnnotation {
    fn markup(&self) -> &MarkupBase {
        &self.markup
    }
    fn markup_mut(&mut self) -> &mut MarkupBase {
        &mut self.markup
    }
}
impl Annotation for CaretAnnotation {
    fn annotation_type(&self) -> AnnotationType {
        AnnotationType::Caret
    }
}
impl MarkupAnnotation for CaretAnnotation {}
