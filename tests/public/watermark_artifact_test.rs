// Public-API driver — WatermarkArtifact + Page.Artifacts() render-on-save.
//
// A WatermarkArtifact added through Page.artifacts() renders into the
// page on save: a text watermark as a positioned text run (extractable
// after a save / reopen round-trip via TextAbsorber); an image watermark
// as an embedded image XObject (surfaces in Page.resources().images()).
// Also covers the collection's add / count / delete semantics and the
// default Artifact property values. Ports the C++
// watermark_artifact_smoke_test.

use aspose_pdf_foss::{Artifact, Document, Point, TextAbsorber, TextState, WatermarkArtifact};
use std::path::{Path, PathBuf};

fn hello_world() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/text_extractor/hello_world.pdf")
}

fn jpeg_fixture() -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/dctdecode/gradient_32.jpg"),
    )
    .unwrap()
}

// A text WatermarkArtifact renders as page text that survives a round-trip.
#[test]
fn text_watermark_renders_on_save() {
    let out = common::unique_temp("aspose_rust_wm_text.pdf");
    {
        let doc = Document::open(hello_world()).unwrap();
        let mut wm = WatermarkArtifact::new();
        let mut ts = TextState::default();
        ts.set_font_size(36.0);
        wm.set_text_and_state("CONFIDENTIAL", ts);
        wm.set_position(Point::new(150.0, 400.0));
        wm.set_rotation(45.0);
        wm.set_opacity(0.4);
        wm.set_is_background(true);

        let page = doc.pages().get(1).unwrap();
        page.artifacts().add(wm);
        assert_eq!(page.artifacts().count(), 1);
        doc.save_to(&out).unwrap();
    }

    let doc = Document::open(&out).unwrap();
    let mut a = TextAbsorber::new();
    a.visit_document(&doc).unwrap();
    assert!(a.text().contains("CONFIDENTIAL"), "got: {:?}", a.text());

    let _ = std::fs::remove_file(&out);
}

// An image WatermarkArtifact embeds an image XObject on the page.
#[test]
fn image_watermark_embeds_xobject() {
    let out = common::unique_temp("aspose_rust_wm_img.pdf");
    let jpeg = jpeg_fixture();
    assert!(jpeg.len() >= 2);
    {
        let doc = Document::open(hello_world()).unwrap();
        let mut wm = WatermarkArtifact::new();
        wm.set_image_bytes(&jpeg);
        wm.set_position(Point::new(100.0, 200.0));
        wm.set_opacity(0.5);

        let page = doc.pages().get(1).unwrap();
        page.artifacts().add(wm);
        doc.save_to(&out).unwrap();
    }

    let doc = Document::open(&out).unwrap();
    assert_eq!(doc.pages().get(1).unwrap().resources().images().count(), 1);

    let _ = std::fs::remove_file(&out);
}

// Collection semantics: add / count / delete (1-based index).
#[test]
fn collection_add_and_delete() {
    let doc = Document::open(hello_world()).unwrap();
    let mut a = WatermarkArtifact::new();
    a.set_text("alpha");
    let mut b = WatermarkArtifact::new();
    b.set_text("beta");
    let page = doc.pages().get(1).unwrap();
    let artifacts = page.artifacts();
    artifacts.add(a);
    artifacts.add(b);
    assert_eq!(artifacts.count(), 2);
    artifacts.delete_at(1);
    assert_eq!(artifacts.count(), 1);
    // The first (alpha) was removed; beta is now at index 1.
    assert_eq!(artifacts.get(1).unwrap().text(), "beta");
    // Out-of-range lookups yield None (the Rust analogue of the
    // canonical indexer's out_of_range throw).
    assert!(artifacts.get(2).is_none());
}

// Default Artifact property values.
#[test]
fn default_properties() {
    let mut wm = WatermarkArtifact::new();
    assert_eq!(wm.opacity(), 1.0);
    assert_eq!(wm.rotation(), 0.0);
    assert!(!wm.is_background());
    assert!(wm.text().is_empty());

    wm.set_is_background(true);
    wm.set_rotation(90.0);
    assert!(wm.is_background());
    assert_eq!(wm.rotation(), 90.0);

    // Into<Artifact> carries the inherited state.
    let a: Artifact = wm.into();
    assert!(a.is_background());
    assert_eq!(a.rotation(), 90.0);
}

#[path = "../common/mod.rs"]
mod common;

// Rotation was carried on the artifact and never applied: a watermark
// set to 45 degrees painted flat. It turns about the artifact's own
// position, so the anchor stays put.
#[test]
fn a_rotated_watermark_turns_about_its_position() {
    let out = common::unique_temp("aspose_rust_wm_rot.pdf");
    {
        let doc = Document::open(hello_world()).unwrap();
        let mut wm = WatermarkArtifact::new();
        let mut ts = TextState::default();
        ts.set_font_size(24.0);
        wm.set_text_and_state("TURNED", ts);
        wm.set_position(Point::new(100.0, 200.0));
        wm.set_rotation(90.0);
        doc.pages().get(1).unwrap().artifacts().add(wm);
        doc.save_to(&out).unwrap();
    }
    let content = String::from_utf8_lossy(&std::fs::read(&out).unwrap()).into_owned();

    // A quarter turn about (100, 200): cos 0, sin 1, and the pivot
    // stays where it was — e = 100 - 0 + 200 = 300, f = 200 - 100 - 0.
    assert!(
        content.contains("0 1 -1 0 300 100 cm"),
        "no rotation matrix: {content:?}"
    );
    let _ = std::fs::remove_file(&out);
}

#[test]
fn an_unrotated_watermark_emits_no_matrix() {
    let out = common::unique_temp("aspose_rust_wm_norot.pdf");
    {
        let doc = Document::open(hello_world()).unwrap();
        let mut wm = WatermarkArtifact::new();
        wm.set_text("FLAT");
        wm.set_position(Point::new(100.0, 200.0));
        doc.pages().get(1).unwrap().artifacts().add(wm);
        doc.save_to(&out).unwrap();
    }
    let content = String::from_utf8_lossy(&std::fs::read(&out).unwrap()).into_owned();
    assert!(content.contains("(FLAT) Tj"), "the watermark is missing");
    assert!(!content.contains(" cm\n"), "an unrotated artifact must not emit a matrix");
    let _ = std::fs::remove_file(&out);
}
