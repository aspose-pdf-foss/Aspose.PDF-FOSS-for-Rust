// Hand-authored gate driver — pixel_encode_roundtrip gate.
// Primitive: bmp_encoder
// Freeze gate: byte-equality round-trip — for each `.bencfix`
// fixture the driver parses (color_type, page) out of the wire
// format, calls bmp_encoder::encode, runs bmp_decoder::decode on
// the produced BMP, and asserts the decoded RGBA buffer equals
// the input RGBA buffer byte-for-byte. BMP is lossless on every
// in-scope (bit-depth, compression) pair, so byte-identical
// agreement is the bar.
//
// .bencfix wire format (per fixture_format: in
// Feedstock/PDF/Spec/bmp_encoder.yaml):
//
//   Header (8 bytes):
//     [0..3]   magic       : "BENC"
//     [4]      version     : 1
//     [5]      color_type  : 24 (Bgr) or 32 (Bgra)
//     [6..7]   reserved    : zero
//
//   Page block:
//     [0..3]   width       : uint32 LE
//     [4..7]   height      : uint32 LE
//     [8..]    rgba8       : width*height*4 bytes, R G B A,
//                            row-major top-to-bottom

#[path = "../../src/internal/bmp_encoder.rs"]
mod bmp_encoder;

#[path = "../../src/internal/bmp_decoder.rs"]
mod bmp_decoder;

use std::fs;
use std::path::{Path, PathBuf};

use bmp_encoder::{encode, ColorType, Options};

const BENCFIX_MAGIC: &[u8; 4] = b"BENC";
const BENCFIX_HEADER_SIZE: usize = 8;

fn corpus_dir() -> PathBuf {
    if let Ok(env_dir) = std::env::var("SPECDEV_CORPUS_DIR") {
        if !env_dir.is_empty() {
            return PathBuf::from(env_dir);
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("bmp_encoder")
}

struct Parsed {
    color_type: ColorType,
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

fn read_u32_le(buf: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([buf[off], buf[off + 1], buf[off + 2], buf[off + 3]])
}

fn parse_bencfix(path: &Path, bytes: &[u8]) -> Parsed {
    assert!(bytes.len() >= BENCFIX_HEADER_SIZE,
        "{}: shorter than 8-byte header", path.display());
    assert_eq!(&bytes[..4], BENCFIX_MAGIC,
        "{}: magic mismatch", path.display());
    assert_eq!(bytes[4], 1,
        "{}: unsupported version {}", path.display(), bytes[4]);
    let color_type = match bytes[5] {
        24 => ColorType::Bgr,
        32 => ColorType::Bgra,
        other => panic!("{}: wire color_type {} not in {{24, 32}}",
                        path.display(), other),
    };
    // bytes[6..8] reserved — ignored.

    let cursor = BENCFIX_HEADER_SIZE;
    assert!(bytes.len() >= cursor + 8,
        "{}: truncated at page header", path.display());
    let width = read_u32_le(bytes, cursor);
    let height = read_u32_le(bytes, cursor + 4);
    let pixel_off = cursor + 8;
    let pixel_len = (width as usize) * (height as usize) * 4;
    assert_eq!(bytes.len() - pixel_off, pixel_len,
        "{}: page declares {} pixel bytes but file has {}",
        path.display(), pixel_len, bytes.len() - pixel_off);
    let rgba = bytes[pixel_off..pixel_off + pixel_len].to_vec();
    Parsed { color_type, width, height, rgba }
}

fn run_one(path: &Path) -> bool {
    let bytes = fs::read(path).expect("read .bencfix");
    let parsed = parse_bencfix(path, &bytes);

    let encoded = match encode(
        parsed.width, parsed.height, &parsed.rgba,
        Options { color_type: Some(parsed.color_type) },
    ) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("MISMATCH {}: encode error {:?}",
                      path.display(), e);
            return false;
        }
    };

    let decoded = match bmp_decoder::decode(&encoded) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("MISMATCH {}: decode error {:?}",
                      path.display(), e);
            return false;
        }
    };

    if decoded.width as u32 != parsed.width
        || decoded.height as u32 != parsed.height
    {
        eprintln!(
            "MISMATCH {}: dim mismatch encoded={}x{} decoded={}x{}",
            path.display(),
            parsed.width, parsed.height,
            decoded.width, decoded.height);
        return false;
    }

    if decoded.pixels != parsed.rgba {
        let n = decoded.pixels.len().min(parsed.rgba.len());
        let mut first_diff = None;
        for i in 0..n {
            if decoded.pixels[i] != parsed.rgba[i] {
                first_diff = Some((i, decoded.pixels[i], parsed.rgba[i]));
                break;
            }
        }
        eprintln!(
            "MISMATCH {}: pixel buffers differ (lengths {} vs {}) \
             first diff at {:?}",
            path.display(),
            decoded.pixels.len(), parsed.rgba.len(),
            first_diff);
        return false;
    }

    true
}

#[test]
fn pixel_encode_roundtrip_over_corpus() {
    let dir = corpus_dir();
    assert!(dir.is_dir(),
            "corpus dir not found: {} \
             (set SPECDEV_CORPUS_DIR or vendor tests/fixtures/bmp_encoder/)",
            dir.display());

    let mut fixtures: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("read corpus dir")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .filter(|p| p.extension().and_then(|s| s.to_str())
                     == Some("bencfix"))
        .collect();
    fixtures.sort();

    assert!(!fixtures.is_empty(),
            "no .bencfix fixtures under {}", dir.display());

    let mut checked = 0_usize;
    let mut failed = 0_usize;
    for path in &fixtures {
        eprintln!("TRY {}", path.display());
        if !run_one(path) {
            failed += 1;
        }
        checked += 1;
    }
    eprintln!("bmp_encoder pixel_encode_roundtrip: \
               checked={} failed={}", checked, failed);
    assert_eq!(failed, 0, "{} fixture mismatches", failed);
}
