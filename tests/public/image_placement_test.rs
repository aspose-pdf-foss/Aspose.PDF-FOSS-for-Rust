// Public-API driver — ImagePlacementAbsorber walks a page's drawn
// images and reports each placement's geometry.
//
// The fixture `page_renderer/image_xobject.pdf` draws a 4×4 image with
// `q 100 0 0 50 50 15 cm /Im0 Do` on a 200×80 page, so the placement is
// fully determined: matrix [100 0 0 50 50 15], rectangle (50,15)-(150,65),
// zero rotation, and a resolution of 4·72/100 × 4·72/50 = 3 × 6 DPI.

use aspose_pdf_foss::{BlendMode, Document, ImageFilterType, ImagePlacementAbsorber};
use std::path::{Path, PathBuf};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}
fn image_xobject() -> PathBuf {
    fixtures().join("page_renderer/image_xobject.pdf")
}
fn hello_world() -> PathBuf {
    fixtures().join("text_extractor/hello_world.pdf")
}

#[test]
fn visit_document_finds_the_drawn_image() {
    let doc = Document::open(image_xobject()).unwrap();
    let mut a = ImagePlacementAbsorber::new();
    a.visit_document(&doc).unwrap();
    assert_eq!(a.image_placements().count(), 1);
}

#[test]
fn placement_rectangle_matches_the_cm_matrix() {
    let doc = Document::open(image_xobject()).unwrap();
    let mut a = ImagePlacementAbsorber::new();
    a.visit_document(&doc).unwrap();
    let p = a.image_placements().get(1).unwrap();
    let r = p.rectangle();
    assert!((r.llx() - 50.0).abs() < 1e-6, "llx {}", r.llx());
    assert!((r.lly() - 15.0).abs() < 1e-6, "lly {}", r.lly());
    assert!((r.urx() - 150.0).abs() < 1e-6, "urx {}", r.urx());
    assert!((r.ury() - 65.0).abs() < 1e-6, "ury {}", r.ury());
    assert!((r.width() - 100.0).abs() < 1e-6);
    assert!((r.height() - 50.0).abs() < 1e-6);
}

#[test]
fn placement_matrix_is_the_ctm() {
    let doc = Document::open(image_xobject()).unwrap();
    let mut a = ImagePlacementAbsorber::new();
    a.visit_document(&doc).unwrap();
    let m = a.image_placements().get(1).unwrap().matrix();
    assert_eq!((m.a(), m.b(), m.c(), m.d(), m.e(), m.f()), (100.0, 0.0, 0.0, 50.0, 50.0, 15.0));
}

#[test]
fn axis_aligned_placement_has_zero_rotation() {
    let doc = Document::open(image_xobject()).unwrap();
    let mut a = ImagePlacementAbsorber::new();
    a.visit_document(&doc).unwrap();
    assert!(a.image_placements().get(1).unwrap().rotation().abs() < 1e-4);
}

#[test]
fn resolution_is_pixels_over_placed_size() {
    let doc = Document::open(image_xobject()).unwrap();
    let mut a = ImagePlacementAbsorber::new();
    a.visit_document(&doc).unwrap();
    let res = a.image_placements().get(1).unwrap().resolution();
    assert_eq!(res.x(), 3); // 4 px / (100/72) in
    assert_eq!(res.y(), 6); // 4 px / (50/72) in
}

#[test]
fn placement_exposes_image_and_page() {
    let doc = Document::open(image_xobject()).unwrap();
    let mut a = ImagePlacementAbsorber::new();
    a.visit_document(&doc).unwrap();
    let p = a.image_placements().get(1).unwrap();
    assert_eq!(p.image().width(), 4);
    assert_eq!(p.image().height(), 4);
    assert!(!p.image().name().is_empty());
    // The placement's page is page 1 of the document.
    assert_eq!(p.page(), &doc.pages().get(1).unwrap());
}

#[test]
fn placement_exposes_compositing_parameters() {
    let doc = Document::open(image_xobject()).unwrap();
    let mut a = ImagePlacementAbsorber::new();
    a.visit_document(&doc).unwrap();
    let p = a.image_placements().get(1).unwrap();
    let c = p.compositing_parameters();
    // No `gs`/`/BM` in the content → the default Normal blend mode.
    assert_eq!(c.blend_mode(), BlendMode::Normal);
    // The image XObject is `/Filter /DCTDecode` → baseline JPEG.
    assert_eq!(c.filter_type(), ImageFilterType::Jpeg);
    // The image carries neither `/SMask` nor `/Mask`.
    assert!(!c.is_masked());
}

#[test]
fn visit_page_matches_visit_document() {
    let doc = Document::open(image_xobject()).unwrap();
    let page = doc.pages().get(1).unwrap();
    let mut a = ImagePlacementAbsorber::new();
    a.visit_page(&page).unwrap();
    assert_eq!(a.image_placements().count(), 1);
}

#[test]
fn document_without_drawn_images_yields_none() {
    let doc = Document::open(hello_world()).unwrap();
    let mut a = ImagePlacementAbsorber::new();
    a.visit_document(&doc).unwrap();
    assert_eq!(a.image_placements().count(), 0);
}

#[test]
fn placements_accumulate_across_visits() {
    let doc = Document::open(image_xobject()).unwrap();
    let mut a = ImagePlacementAbsorber::new();
    a.visit_document(&doc).unwrap();
    a.visit_document(&doc).unwrap();
    assert_eq!(a.image_placements().count(), 2);
}

#[test]
fn read_only_mode_flag_round_trips() {
    let mut a = ImagePlacementAbsorber::new();
    assert!(!a.is_read_only_mode());
    a.set_is_read_only_mode(true);
    assert!(a.is_read_only_mode());
}

#[test]
fn out_of_range_index_is_none() {
    let doc = Document::open(image_xobject()).unwrap();
    let mut a = ImagePlacementAbsorber::new();
    a.visit_document(&doc).unwrap();
    assert!(a.image_placements().get(0).is_none());
    assert!(a.image_placements().get(999).is_none());
}
