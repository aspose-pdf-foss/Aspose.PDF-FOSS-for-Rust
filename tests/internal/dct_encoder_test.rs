// Hand-authored gate driver — pixel_encode_roundtrip gate (PSNR mode).
// Primitive: dct_encoder
//
// For each `.dencfix` fixture under tests/fixtures/dct_encoder/:
//   1. Parse the .dencfix wire format (per fixture_format: in
//      Feedstock/PDF/Spec/dct_encoder.yaml v1).
//   2. Build the encoder Options POD (translate the 1=Grayscale /
//      2=Rgb wire byte to dct_encoder::Photometric).
//   3. Call dct_encoder::encode with the page's RGBA8.
//   4. Call dctdecode::decode on the encoded JPEG.
//   5. Assert the decoded RGBA8 has the same length as the input
//      (a structural disagreement byte-eq nor PSNR can absorb)
//      and that the channel-independent PSNR is ≥ 30 dB.
//
// 30 dB floor is conservative — a typical baseline JPEG at q=75
// produces 35-45 dB on natural imagery; the headroom absorbs
// Tier-2 spec-recall noise without becoming a green-on-broken
// rubber stamp (a wrong-direction DCT, swapped quant tables, or
// bit-reversed Huffman code falls well below 30 dB).

#[path = "../../src/internal/dct_encoder.rs"]
mod dct_encoder;

#[path = "../../src/internal/dctdecode.rs"]
mod dctdecode;

use std::fs;
use std::path::{Path, PathBuf};

const PSNR_MIN: f64 = 30.0;

fn corpus_dir() -> PathBuf {
    if let Ok(env_dir) = std::env::var("SPECDEV_CORPUS_DIR") {
        if !env_dir.is_empty() {
            return PathBuf::from(env_dir);
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("dct_encoder")
}

fn psnr(a: &[u8], b: &[u8]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut sse: f64 = 0.0;
    for (x, y) in a.iter().zip(b.iter()) {
        let d = *x as f64 - *y as f64;
        sse += d * d;
    }
    if sse == 0.0 {
        return f64::INFINITY;
    }
    let mse = sse / a.len() as f64;
    10.0 * (255.0 * 255.0 / mse).log10()
}

#[test]
fn pixel_encode_roundtrip_over_corpus() {
    let dir = corpus_dir();
    assert!(
        dir.is_dir(),
        "corpus dir not found: {} \
         (set SPECDEV_CORPUS_DIR or vendor tests/fixtures/dct_encoder/)",
        dir.display()
    );

    let mut fixtures: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("read corpus dir")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("dencfix"))
        .collect();
    fixtures.sort();
    assert!(
        !fixtures.is_empty(),
        "no .dencfix fixtures under {}", dir.display()
    );

    let mut checked = 0_usize;
    for path in &fixtures {
        eprintln!("TRY {}", path.display());
        let blob = fs::read(path).expect("read fixture");
        run_one(&blob, path);
        checked += 1;
    }
    eprintln!(
        "dct_encoder pixel_encode_roundtrip: checked={} failed=0",
        checked
    );
}

fn run_one(fixture: &[u8], path: &Path) {
    // Header (16 bytes; per fixture_format in dct_encoder.yaml).
    const HEADER: usize = 16;
    assert!(
        fixture.len() >= HEADER,
        "{}: .dencfix shorter than header", path.display()
    );
    assert_eq!(
        &fixture[0..4],
        b"DENC",
        "{}: magic mismatch", path.display()
    );
    let version = fixture[4];
    assert_eq!(
        version, 1,
        "{}: .dencfix version {} not supported (expected 1)",
        path.display(), version
    );
    let quality = fixture[5];
    let photometric_byte = fixture[6];
    let photometric = match photometric_byte {
        1 => dct_encoder::Photometric::Grayscale,
        2 => dct_encoder::Photometric::Rgb,
        other => panic!(
            "{}: unknown photometric byte {}", path.display(), other
        ),
    };
    let reserved0 = fixture[7];
    assert_eq!(
        reserved0, 0,
        "{}: reserved0 must be 0 (got {})", path.display(), reserved0
    );
    let dpi = u32::from_le_bytes(fixture[8..12].try_into().unwrap());
    let reserved1 = &fixture[12..16];
    assert!(
        reserved1.iter().all(|&b| b == 0),
        "{}: reserved1 must be zero", path.display()
    );

    // Page block: width, height, RGBA8.
    let mut cursor = HEADER;
    assert!(
        fixture.len() >= cursor + 8,
        "{}: truncated at page header", path.display()
    );
    let width = u32::from_le_bytes(
        fixture[cursor..cursor + 4].try_into().unwrap(),
    );
    let height = u32::from_le_bytes(
        fixture[cursor + 4..cursor + 8].try_into().unwrap(),
    );
    cursor += 8;
    let pixel_len = (width as usize) * (height as usize) * 4;
    assert!(
        fixture.len() >= cursor + pixel_len,
        "{}: truncated at page pixels", path.display()
    );
    let rgba = &fixture[cursor..cursor + pixel_len];

    let options = dct_encoder::Options {
        quality,
        photometric,
        dpi,
    };
    let encoded = dct_encoder::encode(width, height, rgba, &options)
        .unwrap_or_else(|e| panic!("encode failed for {}: {}", path.display(), e));
    let decoded = dctdecode::decode(&encoded)
        .unwrap_or_else(|e| panic!("decode failed for {}: {}", path.display(), e));

    assert_eq!(
        decoded.pixels.len(),
        rgba.len(),
        "pixel-buffer length mismatch for {} (decoded {} vs expected {})",
        path.display(), decoded.pixels.len(), rgba.len()
    );
    let p = psnr(&decoded.pixels, rgba);
    assert!(
        p >= PSNR_MIN,
        "PSNR {p:.2} dB below floor {PSNR_MIN} dB for {}",
        path.display()
    );
    eprintln!(
        "  OK  {}  PSNR={:.2} dB",
        path.file_name().unwrap().to_string_lossy(),
        p
    );
}
