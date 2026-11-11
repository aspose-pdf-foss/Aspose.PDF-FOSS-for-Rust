// Public-API smoke driver — Page geometry (Rect / MediaBox / CropBox /
// Rotate / GetPageRect / SetPageSize), ported from the C++ peer's
// page_geometry_smoke_test.cpp and page_boxes_smoke_test.cpp.
//
// Reads resolve from the parsed page tree; writes stage on a shared
// interior-mutable geometry handle and persist through Document::save
// via an incremental /MediaBox / /CropBox / /Rotate rewrite. An
// untouched document resaves verbatim.

use std::path::PathBuf;

use aspose_pdf_foss::{Document, Rectangle, Rotation};

fn fixture(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(rel)
}

const HELLO_WORLD: &str = "text_extractor/hello_world.pdf";

fn open_hello() -> Document {
    Document::open(fixture(HELLO_WORLD)).expect("open hello_world.pdf")
}

fn approx(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-4
}

#[test]
fn read_defaults() {
    let doc = open_hello();
    let page = doc.pages().get(1).expect("page 1");
    assert!(approx(page.rect().width(), 612.0), "rect width");
    assert!(approx(page.rect().height(), 792.0), "rect height");
    assert!(approx(page.media_box().width(), 612.0), "media_box width");
    assert_eq!(page.rotate(), Rotation::None, "rotate default");
    assert!(
        approx(page.get_page_rect(true).height(), 792.0),
        "get_page_rect height"
    );
    // CropBox defaults to MediaBox.
    assert!(approx(page.crop_box().width(), 612.0), "crop_box width");
}

#[test]
fn set_get_in_session() {
    let doc = open_hello();
    doc.pages().get(1).unwrap().set_page_size(300.0, 400.0);
    let page = doc.pages().get(1).unwrap();
    assert!(approx(page.rect().width(), 300.0), "resized width");
    assert!(approx(page.rect().height(), 400.0), "resized height");

    doc.pages()
        .get(1)
        .unwrap()
        .set_rect(Rectangle::new(10.0, 20.0, 210.0, 320.0, false));
    let p2 = doc.pages().get(1).unwrap();
    assert!(approx(p2.rect().llx(), 10.0), "rect llx");
    assert!(approx(p2.rect().width(), 200.0), "rect width");
    assert!(approx(p2.rect().height(), 300.0), "rect height");
}

#[test]
fn set_page_size_save_reload() {
    let bytes = {
        let doc = open_hello();
        doc.pages().get(1).unwrap().set_page_size(360.0, 480.0);
        let mut buf: Vec<u8> = Vec::new();
        doc.save_to_writer(&mut buf).expect("save");
        buf
    };
    let re = Document::from_bytes(bytes).expect("reopen");
    let page = re.pages().get(1).unwrap();
    assert!(approx(page.rect().width(), 360.0), "reloaded width");
    assert!(approx(page.rect().height(), 480.0), "reloaded height");
}

#[test]
fn rotate_save_reload() {
    let bytes = {
        let doc = open_hello();
        doc.pages().get(1).unwrap().set_rotate(Rotation::On90);
        let mut buf: Vec<u8> = Vec::new();
        doc.save_to_writer(&mut buf).expect("save");
        buf
    };
    let re = Document::from_bytes(bytes).expect("reopen");
    assert_eq!(re.pages().get(1).unwrap().rotate(), Rotation::On90);
}

#[test]
fn no_edit_saves_verbatim() {
    let doc = open_hello();
    let mut buf: Vec<u8> = Vec::new();
    doc.save_to_writer(&mut buf).expect("save");
    // No geometry (or other) edit → save short-circuits to a verbatim
    // copy of the source bytes.
    let original = std::fs::read(fixture(HELLO_WORLD)).expect("read fixture");
    assert_eq!(buf, original, "verbatim save");
}

#[test]
fn crop_box_set_and_save_round_trip() {
    let bytes = {
        let doc = open_hello();
        doc.pages()
            .get(1)
            .unwrap()
            .set_crop_box(Rectangle::new(10.0, 20.0, 200.0, 300.0, false));
        let page = doc.pages().get(1).unwrap();
        assert!(approx(page.crop_box().llx(), 10.0), "crop llx in session");
        // MediaBox is untouched by a CropBox-only edit.
        assert!(
            approx(page.media_box().width(), 612.0),
            "media_box unchanged"
        );
        let mut buf: Vec<u8> = Vec::new();
        doc.save_to_writer(&mut buf).expect("save");
        buf
    };
    let re = Document::from_bytes(bytes).expect("reopen");
    let page = re.pages().get(1).unwrap();
    assert!(approx(page.crop_box().llx(), 10.0), "reloaded crop llx");
    assert!(approx(page.crop_box().ury(), 300.0), "reloaded crop ury");
    // Effective page rect follows the CropBox.
    assert!(approx(page.rect().width(), 190.0), "reloaded rect width");
}

#[test]
fn get_page_rect_considers_rotation() {
    let doc = open_hello();
    let page = doc.pages().get(1).unwrap();
    page.set_rotate(Rotation::On90);
    // With rotation considered, a 90° turn swaps width/height.
    let rotated = page.get_page_rect(true);
    assert!(approx(rotated.width(), 792.0), "rotated width");
    assert!(approx(rotated.height(), 612.0), "rotated height");
    // Without rotation, the unrotated crop box is returned verbatim.
    let upright = page.get_page_rect(false);
    assert!(approx(upright.width(), 612.0), "upright width");
    assert!(approx(upright.height(), 792.0), "upright height");
}

/// §14.11.2: a page that names none of the finishing boxes reports the
/// `/CropBox` for all three, which is the default the specification
/// gives them — not an empty rectangle and not the `/MediaBox`.
#[test]
fn the_finishing_boxes_default_to_the_crop_box() {
    let doc = open_hello();
    let page = doc.pages().get(1).unwrap();
    page.set_crop_box(Rectangle::new(5.0, 6.0, 300.0, 400.0, false));
    for (name, got) in [
        ("art", page.art_box()),
        ("bleed", page.bleed_box()),
        ("trim", page.trim_box()),
    ] {
        assert!(approx(got.llx(), 5.0), "{name} llx follows the crop box");
        assert!(approx(got.urx(), 300.0), "{name} urx follows the crop box");
    }
}

#[test]
fn the_finishing_boxes_survive_a_save_and_reload() {
    let bytes = {
        let doc = open_hello();
        let page = doc.pages().get(1).unwrap();
        page.set_art_box(Rectangle::new(10.0, 11.0, 210.0, 311.0, false));
        page.set_bleed_box(Rectangle::new(12.0, 13.0, 212.0, 313.0, false));
        page.set_trim_box(Rectangle::new(14.0, 15.0, 214.0, 315.0, false));
        // Each is independent — setting one must not move the others.
        assert!(approx(page.art_box().llx(), 10.0));
        assert!(approx(page.bleed_box().lly(), 13.0));
        assert!(approx(page.trim_box().urx(), 214.0));
        let mut buf: Vec<u8> = Vec::new();
        doc.save_to_writer(&mut buf).expect("save");
        buf
    };
    let re = Document::from_bytes(bytes).expect("reopen");
    let page = re.pages().get(1).unwrap();
    assert!(approx(page.art_box().llx(), 10.0), "art box reloaded");
    assert!(approx(page.art_box().ury(), 311.0));
    assert!(approx(page.bleed_box().llx(), 12.0), "bleed box reloaded");
    assert!(approx(page.trim_box().lly(), 15.0), "trim box reloaded");
    // The crop box is untouched by a finishing-box edit.
    assert!(approx(page.crop_box().width(), 612.0), "crop box unchanged");
}

#[test]
fn a_page_that_never_set_a_finishing_box_writes_none() {
    // An absent box stays absent: reopening a page whose only edit was
    // the crop box must still report the crop box for all three, which
    // it could not do if the writer had materialised copies.
    let bytes = {
        let doc = open_hello();
        doc.pages()
            .get(1)
            .unwrap()
            .set_crop_box(Rectangle::new(1.0, 2.0, 101.0, 202.0, false));
        let mut buf: Vec<u8> = Vec::new();
        doc.save_to_writer(&mut buf).expect("save");
        buf
    };
    assert!(
        !String::from_utf8_lossy(&bytes).contains("/ArtBox"),
        "no /ArtBox should have been written"
    );
    let re = Document::from_bytes(bytes).expect("reopen");
    let page = re.pages().get(1).unwrap();
    assert!(approx(page.trim_box().urx(), 101.0), "still follows the crop box");
}
