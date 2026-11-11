// Public-API driver — Page.Resources.Images save-through.
//
// Exercises the live per-page image handle: a JPEG added through
// Page.resources().images() embeds as a /DCTDecode image XObject in the
// page's /Resources /XObject and round-trips through save / reopen
// (count, resource name, dimensions). An untouched page resaves
// verbatim.

use aspose_pdf_foss::{ColorType, Document, ImageFilterType};
use std::path::{Path, PathBuf};

fn two_pages() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/text_extractor/two_pages.pdf")
}

/// A minimal baseline-DCT JPEG header (SOF0) carrying `width` / `height`
/// — enough for the dimension probe; embedded verbatim under
/// `/DCTDecode`.
fn jpeg(width: u8, height: u8) -> Vec<u8> {
    vec![0xFF, 0xD8, 0xFF, 0xC0, 0x00, 0x11, 0x08, 0x00, height, 0x00, width]
}

#[test]
fn save_reopen_round_trip() {
    let out = common::unique_temp("aspose_rust_ximage.pdf");
    let img = jpeg(40, 30);
    let name;
    {
        let doc = Document::open(two_pages()).unwrap();
        assert_eq!(doc.pages().get(1).unwrap().resources().images().count(), 0);
        name = doc.pages().get(1).unwrap().resources().images().add_stream(&img);
        doc.save_to(&out).unwrap();
    }

    let saved = std::fs::read(&out).unwrap();
    assert!(
        saved.windows(b"/DCTDecode".len()).any(|w| w == b"/DCTDecode"),
        "the image XObject must be written with a /DCTDecode filter"
    );
    assert!(
        saved.windows(img.len()).any(|w| w == img.as_slice()),
        "the JPEG bytes must appear verbatim in the saved PDF"
    );

    let doc = Document::open(&out).unwrap();
    let page = doc.pages().get(1).unwrap();
    let images = page.resources().images();
    assert_eq!(images.count(), 1);
    assert_eq!(images.names(), vec![name.clone()]);
    let by_name = images.get_by_name(&name).unwrap();
    assert_eq!(by_name.width(), 40);
    assert_eq!(by_name.height(), 30);

    let _ = std::fs::remove_file(&out);
}

#[test]
fn second_page_resources_are_independent() {
    // Adding to page 1 must not touch page 2's resources.
    let out = common::unique_temp("aspose_rust_ximage_p1.pdf");
    {
        let doc = Document::open(two_pages()).unwrap();
        doc.pages().get(1).unwrap().resources().images().add_stream(&jpeg(8, 8));
        doc.save_to(&out).unwrap();
    }
    let doc = Document::open(&out).unwrap();
    assert_eq!(doc.pages().get(1).unwrap().resources().images().count(), 1);
    assert_eq!(doc.pages().get(2).unwrap().resources().images().count(), 0);
    let _ = std::fs::remove_file(&out);
}

#[test]
fn verbatim_resave_keeps_no_images() {
    let out = common::unique_temp("aspose_rust_ximage_verbatim.pdf");
    let original = std::fs::read(two_pages()).unwrap();
    {
        let doc = Document::open(two_pages()).unwrap();
        let __pg0 = doc.pages().get(1).unwrap();
        let _ = __pg0.resources().images().count();
        doc.save_to(&out).unwrap();
    }
    let resaved = std::fs::read(&out).unwrap();
    assert_eq!(resaved, original, "untouched page resources must resave verbatim");
    let _ = std::fs::remove_file(&out);
}

/// A grayscale baseline JPEG: the SOF0 declares a single component, so
/// it embeds under `/DeviceGray`.
fn gray_jpeg(width: u8, height: u8) -> Vec<u8> {
    vec![0xFF, 0xD8, 0xFF, 0xC0, 0x00, 0x11, 0x08, 0x00, height, 0x00, width, 0x01]
}

#[test]
fn filter_and_color_type_survive_round_trip() {
    // A colour JPEG (default 3 components → /DeviceRGB) and a grayscale
    // JPEG (1 component → /DeviceGray) added to a page must report their
    // filter and colour classification after save / reopen — read back
    // from the written image XObject dictionary.
    let out = common::unique_temp("aspose_rust_ximage_classify.pdf");
    let (rgb_name, gray_name);
    {
        let doc = Document::open(two_pages()).unwrap();
        let page = doc.pages().get(1).unwrap();
        let images = page.resources().images();
        rgb_name = images.add_stream(&jpeg(20, 10));
        gray_name = images.add_stream(&gray_jpeg(12, 8));
        doc.save_to(&out).unwrap();
    }

    let doc = Document::open(&out).unwrap();
    let page = doc.pages().get(1).unwrap();
    let images = page.resources().images();

    let rgb = images.get_by_name(&rgb_name).unwrap();
    assert_eq!(rgb.filter_type(), ImageFilterType::Jpeg);
    assert_eq!(rgb.get_color_type(), ColorType::Rgb);

    let gray = images.get_by_name(&gray_name).unwrap();
    assert_eq!(gray.filter_type(), ImageFilterType::Jpeg);
    assert_eq!(gray.get_color_type(), ColorType::Grayscale);

    let _ = std::fs::remove_file(&out);
}

#[path = "../common/mod.rs"]
mod common;
