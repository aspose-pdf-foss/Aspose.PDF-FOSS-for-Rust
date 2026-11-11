// Hand-authored gate driver — pixel_encode_roundtrip gate
// (byte-equality mode).
// Primitive: png_encoder
//
// For each `.pencfix` fixture under tests/fixtures/png_encoder/:
//   1. Parse the .pencfix wire format (per fixture_format: in
//      Feedstock/PDF/Spec/png_encoder.yaml v1).
//   2. Build the encoder Options POD (color_type maps directly
//      to the IHDR byte value 0/2/6 — no translation).
//   3. Call png_encoder::encode with the page's RGBA8.
//   4. Call png_decoder::decode on the encoded PNG.
//   5. Assert the decoded RGBA8 equals the input RGBA8 byte-
//      for-byte. PNG is lossless on every in-scope colour type;
//      the .pencfix corpus is constructed so per-colour-type
//      coercions round-trip identity (alpha widened to 0xFF on
//      RGB / Grayscale read, R=G=B widened from Y on Grayscale
//      read).
//
// png_encoder composes flate; the integration-test crate also
// pulls in png_decoder for the round-trip read.

#[path = "../../src/internal/flate.rs"]
mod flate;

#[path = "../../src/internal/png_decoder.rs"]
mod png_decoder;

#[path = "../../src/internal/png_encoder.rs"]
mod png_encoder;

use std::fs;
use std::path::{Path, PathBuf};

fn corpus_dir() -> PathBuf {
    if let Ok(env_dir) = std::env::var("SPECDEV_CORPUS_DIR") {
        if !env_dir.is_empty() {
            return PathBuf::from(env_dir);
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("png_encoder")
}

#[test]
fn pixel_encode_roundtrip_over_corpus() {
    let dir = corpus_dir();
    assert!(dir.is_dir(),
            "corpus dir not found: {} \
             (set SPECDEV_CORPUS_DIR or vendor tests/fixtures/png_encoder/)",
            dir.display());

    let mut fixtures: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("read corpus dir")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("pencfix"))
        .collect();
    fixtures.sort();

    assert!(!fixtures.is_empty(),
            "no .pencfix fixtures under {}", dir.display());

    let mut checked = 0_usize;
    for path in &fixtures {
        eprintln!("TRY {}", path.display());
        let blob = fs::read(path).expect("read fixture");
        run_one(&blob, path);
        checked += 1;
    }
    eprintln!("png_encoder pixel_encode_roundtrip: checked={} failed=0",
              checked);
}

fn run_one(fixture: &[u8], path: &Path) {
    const HEADER: usize = 8;
    assert!(fixture.len() >= HEADER,
            "{}: .pencfix shorter than header", path.display());
    assert_eq!(&fixture[0..4], b"PENC",
               "{}: magic mismatch", path.display());
    let version = fixture[4];
    assert_eq!(version, 1,
               "{}: .pencfix version {} not supported (expected 1)",
               path.display(), version);
    let color_type_byte = fixture[5];
    let color_type = match color_type_byte {
        0 => png_encoder::ColorType::Grayscale,
        2 => png_encoder::ColorType::Rgb,
        6 => png_encoder::ColorType::Rgba,
        other => panic!("{}: unknown color_type byte {}",
                        path.display(), other),
    };

    let cursor = HEADER;
    assert!(fixture.len() >= cursor + 8,
            "{}: truncated at page header", path.display());
    let width = u32::from_le_bytes(
        fixture[cursor..cursor + 4].try_into().unwrap());
    let height = u32::from_le_bytes(
        fixture[cursor + 4..cursor + 8].try_into().unwrap());
    let pixel_len = (width as usize) * (height as usize) * 4;
    let pixels_start = cursor + 8;
    assert!(fixture.len() >= pixels_start + pixel_len,
            "{}: truncated at page pixels", path.display());
    let rgba = &fixture[pixels_start..pixels_start + pixel_len];

    let options = png_encoder::Options { color_type };
    let encoded = png_encoder::encode(width, height, rgba, &options)
        .unwrap_or_else(|e| panic!("encode failed for {}: {}",
                                   path.display(), e));
    let decoded = png_decoder::decode(&encoded)
        .unwrap_or_else(|e| panic!("decode failed for {}: {}",
                                   path.display(), e));
    assert_eq!(
        decoded.pixels, rgba,
        "byte-equality failed for {}", path.display());
}
