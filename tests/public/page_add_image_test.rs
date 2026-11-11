// Public-API driver — Page.add_image(bytes, rect, bbox, autoAdjust).
//
// The canonical Page.AddImage(Stream, Rectangle, …) overload places an
// in-memory JPEG on the page; it embeds as an image XObject that survives
// a save / reopen as a page resource. Unsupported image bytes return an
// error (the canonical void overload signals failure by throwing). Ports
// the C++ page_add_image_stream_smoke_test.

use aspose_pdf_foss::{Document, Rectangle};
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

// Page.add_image(bytes, rect) embeds the image; it survives save/reload
// as a page resource with positive dimensions.
#[test]
fn places_image_from_bytes() {
    let out = common::unique_temp("aspose_rust_addimg_stream.pdf");
    let jpeg = jpeg_fixture();
    assert!(jpeg.len() >= 2);
    {
        let doc = Document::open(hello_world()).unwrap();
        doc.pages()
            .get(1)
            .unwrap()
            .add_image(&jpeg, Rectangle::new(100.0, 100.0, 300.0, 300.0, true), None, true)
            .unwrap();
        doc.save_to(&out).unwrap();
    }

    let doc = Document::open(&out).unwrap();
    let page = doc.pages().get(1).unwrap();
    let images = page.resources().images();
    assert_eq!(images.count(), 1);
    assert!(images.get(0).unwrap().width() > 0);
    assert!(images.get(0).unwrap().height() > 0);

    let _ = std::fs::remove_file(&out);
}

// The bbox + autoAdjustRectangle overload arguments are accepted and the
// image still lands as a resource.
#[test]
fn places_image_with_bbox_clip() {
    let out = common::unique_temp("aspose_rust_addimg_bbox.pdf");
    let jpeg = jpeg_fixture();
    {
        let doc = Document::open(hello_world()).unwrap();
        doc.pages()
            .get(1)
            .unwrap()
            .add_image(
                &jpeg,
                Rectangle::new(100.0, 100.0, 300.0, 300.0, true),
                Some(Rectangle::new(110.0, 110.0, 200.0, 200.0, true)),
                false,
            )
            .unwrap();
        doc.save_to(&out).unwrap();
    }

    let doc = Document::open(&out).unwrap();
    assert_eq!(doc.pages().get(1).unwrap().resources().images().count(), 1);

    let _ = std::fs::remove_file(&out);
}

// Unsupported image bytes return an error.
#[test]
fn errors_on_unsupported_bytes() {
    let doc = Document::open(hello_world()).unwrap();
    let junk = vec![0u8; 32];
    let result = doc
        .pages()
        .get(1)
        .unwrap()
        .add_image(&junk, Rectangle::new(0.0, 0.0, 10.0, 10.0, true), None, true);
    assert!(result.is_err());
}

#[path = "../common/mod.rs"]
mod common;
