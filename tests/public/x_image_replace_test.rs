// Public-API smoke driver — `XImageCollection::replace` (ports the cpp
// x_image_replace_smoke_test). Replacing a loaded image's content keeps
// its resource name but swaps the XObject: after save / reopen the
// image's dimensions and bytes reflect the replacement. Out-of-range
// replace is a well-defined no-op (the count / contents are unchanged).

use aspose_pdf_foss::Document;
use std::path::{Path, PathBuf};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}
fn hello_world() -> PathBuf {
    fixtures().join("text_extractor/hello_world.pdf")
}
fn jpeg_32() -> Vec<u8> {
    std::fs::read(fixtures().join("dctdecode/gradient_32.jpg")).unwrap()
}
fn png_16() -> Vec<u8> {
    std::fs::read(fixtures().join("png_decoder/rgb_checker_16.png")).unwrap()
}

#[test]
fn replace_swaps_image_content() {
    let seeded = common::unique_temp("asp_rust_repl_seed.pdf");
    let out = common::unique_temp("asp_rust_repl_out.pdf");

    // Seed: a 32x32 JPEG (embedded verbatim as /DCTDecode).
    {
        let doc = Document::open(hello_world()).unwrap();
        doc.pages().get(1).unwrap().resources().images().add_stream(&jpeg_32());
        doc.save_to(&seeded).unwrap();
    }

    // Reopen, verify the JPEG, replace it with a 16x16 PNG, re-save.
    {
        let doc = Document::open(&seeded).unwrap();
        let __pg0 = doc.pages().get(1).unwrap();
        let images = __pg0.resources().images();
        assert_eq!(images.count(), 1);
        assert_eq!(images.get(0).unwrap().width(), 32);
        let raw = images.get(0).unwrap().get_raw_image_data();
        assert!(raw.len() >= 2);
        assert_eq!(raw[0], 0xFF); // JPEG SOI

        let name_before = images.names();
        images.replace(0, &png_16());
        // Replace keeps the count and the resource name.
        assert_eq!(images.count(), 1);
        assert_eq!(images.names(), name_before);
        doc.save_to(&out).unwrap();
    }

    // After reload the (same, single) image is now 16x16 and no longer
    // a JPEG: the PNG re-embeds as a decoded `/FlateDecode` RGBA raster
    // (16*16*4 bytes), not the original DCT-encoded blob.
    let doc = Document::open(&out).unwrap();
    let __pg1 = doc.pages().get(1).unwrap();
    let images = __pg1.resources().images();
    assert_eq!(images.count(), 1);
    assert_eq!(images.get(0).unwrap().width(), 16);
    assert_eq!(images.get(0).unwrap().height(), 16);
    let raw = images.get(0).unwrap().get_raw_image_data();
    // A DCT blob is the verbatim JPEG (a few hundred bytes, SOI = FF D8);
    // the replacement raster is exactly width*height*4 raw samples.
    assert_eq!(raw.len(), 16 * 16 * 4, "replacement is a decoded raster, not the JPEG");
    assert_ne!(&raw[..2.min(raw.len())], &[0xFF, 0xD8], "no longer a JPEG SOI");

    let _ = std::fs::remove_file(&seeded);
    let _ = std::fs::remove_file(&out);
}

#[test]
fn replace_in_memory_swaps_dimensions_and_name() {
    // Same swap, asserted at the in-memory collection level: the
    // resource name is preserved while the dimensions / bytes change.
    let doc = Document::open(hello_world()).unwrap();
    let __pg2 = doc.pages().get(1).unwrap();
    let images = __pg2.resources().images();
    let name = images.add_stream(&jpeg_32());
    assert_eq!(images.count(), 1);
    assert_eq!(images.get(0).unwrap().width(), 32);

    images.replace(0, &png_16());
    assert_eq!(images.count(), 1);
    assert_eq!(images.names(), vec![name]); // name kept
    assert_eq!(images.get(0).unwrap().width(), 16);
    assert_eq!(images.get(0).unwrap().height(), 16);
}

#[test]
fn replace_out_of_range_is_noop() {
    // C++ throws std::out_of_range; the Rust surface is a well-defined
    // no-op — an out-of-range index leaves the collection untouched.
    let doc = Document::open(hello_world()).unwrap();
    let __pg3 = doc.pages().get(1).unwrap();
    let images = __pg3.resources().images();
    assert_eq!(images.count(), 0);
    images.replace(0, &png_16()); // index past the end
    assert_eq!(images.count(), 0);

    let name = images.add_stream(&jpeg_32());
    images.replace(5, &png_16()); // still out of range
    assert_eq!(images.count(), 1);
    assert_eq!(images.names(), vec![name]);
    assert_eq!(images.get(0).unwrap().width(), 32); // unchanged
}

#[path = "../common/mod.rs"]
mod common;
