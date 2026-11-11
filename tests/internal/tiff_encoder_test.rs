// Hand-authored gate driver — pixel_encode_roundtrip gate
// (byte-equality mode).
// Primitive: tiff_encoder
//
// For each `.tencfix` fixture under tests/fixtures/tiff_encoder/:
//   1. Parse the .tencfix wire format (per fixture_format: in
//      Feedstock/PDF/Spec/tiff_encoder.yaml v2).
//   2. Translate the wire compression / photometric codes
//      (private 0..3) to the encoder's option enum types.
//   3. Call tiff_encoder::encode once with the full pages list
//      — exercises chained-IFD multi-page output.
//   4. For each page index p, build a copy of the encoded TIFF
//      whose first-IFD-offset slot points at the p-th IFD in
//      the chain, then call tiff_decoder::decode on it. The
//      decoder reads only the first IFD; redirecting the slot
//      lets us decode every page with a one-shot decoder.
//   5. Assert the decoded RGBA8 equals the per-page input
//      RGBA8 byte-for-byte (every in-scope compression is
//      lossless; the .tencfix corpus is constructed so every
//      per-photometric coercion round-trips identity).
//
// tiff_encoder composes flate + lzw + runlength + ccitt;
// the integration-test crate pulls in those + tiff_decoder
// via `#[path]` at the crate root.

#[path = "../../src/internal/ccitt.rs"]
mod ccitt;

#[path = "../../src/internal/flate.rs"]
mod flate;

#[path = "../../src/internal/lzw.rs"]
mod lzw;

#[path = "../../src/internal/runlength.rs"]
mod runlength;

#[path = "../../src/internal/tiff_decoder.rs"]
mod tiff_decoder;

#[path = "../../src/internal/tiff_encoder.rs"]
mod tiff_encoder;

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
        .join("tiff_encoder")
}

#[test]
fn pixel_encode_roundtrip_over_corpus() {
    let dir = corpus_dir();
    assert!(dir.is_dir(),
            "corpus dir not found: {} \
             (set SPECDEV_CORPUS_DIR or vendor tests/fixtures/tiff_encoder/)",
            dir.display());

    let mut fixtures: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("read corpus dir")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("tencfix"))
        .collect();
    fixtures.sort();

    assert!(!fixtures.is_empty(),
            "no .tencfix fixtures under {}", dir.display());

    let mut checked = 0_usize;
    for path in &fixtures {
        eprintln!("TRY {}", path.display());
        let blob = fs::read(path).expect("read fixture");
        run_one(&blob, path);
        checked += 1;
    }
    eprintln!("tiff_encoder pixel_encode_roundtrip: checked={} failed=0",
              checked);
}

fn run_one(fixture: &[u8], path: &Path) {
    const HEADER: usize = 16;
    assert!(fixture.len() >= HEADER,
            "{}: .tencfix shorter than header", path.display());
    assert_eq!(&fixture[0..4], b"TENC",
               "{}: magic mismatch", path.display());
    let version = fixture[4];
    assert_eq!(version, 2,
               "{}: .tencfix version {} not supported (expected 2)",
               path.display(), version);
    let wire_comp = fixture[5];
    let wire_photo = fixture[6];
    let spp = fixture[7];
    let dpi = u32::from_le_bytes(fixture[8..12].try_into().unwrap());
    let cmap_bytes =
        u16::from_le_bytes(fixture[12..14].try_into().unwrap()) as usize;
    let page_count = fixture[14] as usize;
    let bps = fixture[15];
    assert!(page_count > 0, "{}: page_count is zero", path.display());
    assert!(matches!(bps, 1 | 4 | 8),
            "{}: bits_per_sample={} not in {{1,4,8}}", path.display(), bps);

    let mut cursor = HEADER;
    let color_map: Option<Vec<u16>> = if cmap_bytes != 0 {
        let entries = 3usize * (1usize << bps);
        assert_eq!(cmap_bytes, entries * 2,
                   "{}: cmap_byte_len mismatch", path.display());
        let mut cmap = Vec::with_capacity(entries);
        for i in 0..entries {
            cmap.push(u16::from_le_bytes(
                fixture[cursor + i * 2..cursor + i * 2 + 2]
                    .try_into().unwrap(),
            ));
        }
        cursor += cmap_bytes;
        Some(cmap)
    } else {
        None
    };

    let mut pages = Vec::with_capacity(page_count);
    let mut per_page_rgba: Vec<Vec<u8>> = Vec::with_capacity(page_count);
    for p in 0..page_count {
        assert!(cursor + 8 <= fixture.len(),
                "{}: truncated at page {} header", path.display(), p);
        let w = u32::from_le_bytes(
            fixture[cursor..cursor + 4].try_into().unwrap());
        let h = u32::from_le_bytes(
            fixture[cursor + 4..cursor + 8].try_into().unwrap());
        cursor += 8;
        let pixel_len = (w as usize) * (h as usize) * 4;
        assert!(cursor + pixel_len <= fixture.len(),
                "{}: truncated at page {} pixels", path.display(), p);
        let rgba = fixture[cursor..cursor + pixel_len].to_vec();
        cursor += pixel_len;
        pages.push(tiff_encoder::Page {
            width: w,
            height: h,
            rgba8: rgba.clone(),
        });
        per_page_rgba.push(rgba);
    }

    let options = tiff_encoder::Options {
        compression: wire_to_compression(wire_comp),
        photometric: wire_to_photometric(wire_photo),
        samples_per_pixel: spp,
        bits_per_sample: bps,
        dpi,
        color_map,
    };

    let encoded = tiff_encoder::encode(&pages, &options)
        .unwrap_or_else(|e| panic!("encode failed for {}: {}",
                                   path.display(), e));

    #[allow(clippy::needless_range_loop)] // p is a page index used as a value, not a slice cursor
    for p in 0..page_count {
        let single = if p == 0 {
            encoded.clone()
        } else {
            redirect_first_ifd(&encoded, p, path)
        };
        let decoded = tiff_decoder::decode(&single)
            .unwrap_or_else(|e| panic!(
                "decode failed for {} page {}: {}",
                path.display(), p, e));
        assert_eq!(
            decoded.pixels, per_page_rgba[p],
            "byte-equality failed for {} page {}", path.display(), p);
    }
}

fn wire_to_compression(code: u8) -> tiff_encoder::Compression {
    match code {
        0 => tiff_encoder::Compression::None,
        1 => tiff_encoder::Compression::Lzw,
        2 => tiff_encoder::Compression::Deflate,
        3 => tiff_encoder::Compression::PackBits,
        _ => panic!(".tencfix wire compression code {code} not in 0..3"),
    }
}

fn wire_to_photometric(code: u8) -> tiff_encoder::Photometric {
    match code {
        1 => tiff_encoder::Photometric::BlackIsZero,
        2 => tiff_encoder::Photometric::Rgb,
        3 => tiff_encoder::Photometric::Palette,
        _ => panic!(".tencfix wire photometric code {code} not in 1..3"),
    }
}

// Build a copy of `tiff` whose 8-byte header points the first-
// IFD-offset slot at the `page_index`-th IFD in the chain.
// tiff_decoder reads the first IFD only; the redirect lets us
// decode every page of a multi-page TIFF with a one-shot
// decoder.
fn redirect_first_ifd(tiff: &[u8], page_index: usize, path: &Path) -> Vec<u8> {
    assert!(tiff.len() >= 8, "{}: TIFF shorter than header", path.display());
    assert_eq!(&tiff[0..2], b"II",
               "{}: encoded TIFF not little-endian", path.display());
    let magic = u16::from_le_bytes([tiff[2], tiff[3]]);
    assert_eq!(magic, 42,
               "{}: encoded TIFF magic {} != 42", path.display(), magic);
    let mut ifd_offset =
        u32::from_le_bytes([tiff[4], tiff[5], tiff[6], tiff[7]]) as usize;

    for hop in 0..page_index {
        assert!(ifd_offset != 0,
                "{}: IFD chain shorter than expected (hop {})",
                path.display(), hop);
        assert!(ifd_offset + 2 <= tiff.len(),
                "{}: IFD offset {} runs past TIFF length",
                path.display(), ifd_offset);
        let entries = u16::from_le_bytes(
            tiff[ifd_offset..ifd_offset + 2].try_into().unwrap()) as usize;
        let next_slot = ifd_offset + 2 + entries * 12;
        assert!(next_slot + 4 <= tiff.len(),
                "{}: next-IFD-offset slot runs past TIFF length",
                path.display());
        ifd_offset = u32::from_le_bytes(
            tiff[next_slot..next_slot + 4].try_into().unwrap()) as usize;
    }
    assert!(ifd_offset != 0,
            "{}: IFD chain ended before page {}", path.display(), page_index);

    let mut out = tiff.to_vec();
    out[4..8].copy_from_slice(&(ifd_offset as u32).to_le_bytes());
    out
}
