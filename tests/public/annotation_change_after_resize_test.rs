// Public-API smoke driver — Annotation.ChangeAfterResize.
//
// The base-trait method updates the annotation rectangle by the
// transform matrix (canonical "update parameters ... according to
// the matrix transform"). It is inherited by every annotation type.

use aspose_pdf_foss::{Annotation, Document, LineAnnotation, Matrix, Point, Rectangle, SquareAnnotation};

#[test]
fn change_after_resize_transforms_the_rectangle() {
    let doc = Document::new();
    let mut a = SquareAnnotation::new(&doc);
    a.set_rect(Rectangle::new(0.0, 0.0, 10.0, 20.0, false));
    // Scale ×2 then translate (5, 7): rect corners map through the matrix.
    a.change_after_resize(&Matrix::new(2.0, 0.0, 0.0, 2.0, 5.0, 7.0));
    let r = a.rect();
    assert_eq!((r.llx(), r.lly(), r.urx(), r.ury()), (5.0, 7.0, 25.0, 47.0));
}

#[test]
fn change_after_resize_is_available_on_all_annotations() {
    // The method lives on the base `Annotation` trait, so every
    // annotation type gets it; the identity transform is a no-op.
    let doc = Document::new();
    let mut line = LineAnnotation::new(&doc, Point::new(0.0, 0.0), Point::new(1.0, 1.0));
    let before = line.rect();
    line.change_after_resize(&Matrix::default());
    assert_eq!(line.rect(), before);
}
