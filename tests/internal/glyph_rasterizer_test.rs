// Hand-authored gate driver — lossy_oracle_agreement gate.
// Primitive: glyph_rasterizer
// Freeze gate: for each `Feedstock/PDF/Fixtures/glyph_rasterizer/*.bin`
// fixture X with sidecar X.pixels (raw RGBA8 row-major,
// width*height*4 bytes), assert that decode(X).pixels matches
// X.pixels by PSNR ≥ 40 dB.
//
// Single-oracle authoring (matching rasterizer / dctdecode / jpx).
// The NumPy reference in
// Feedstock/PDF/Fixtures/glyph_rasterizer/generate_fixtures.py
// implements the EXACT algorithm specified in the spec
// (adaptive De Casteljau cubic flattening with FLATNESS=0.25
// and MAX_DEPTH=12, then 4×4 centred super-sampling AA). The
// 40 dB floor absorbs IEEE-754 rounding drift between Python
// and Rust implementations of the same arithmetic; algorithmic
// bugs (cubic-vs-quadratic confusion, ctm transposition,
// curve-after-close cursor reset) produce single or low-double-
// digit PSNR.
//
// Composes the foundation `transform` + `colorspace` + `rasterizer`
// primitives.

#[path = "../../src/internal/transform.rs"]
mod transform;

#[path = "../../src/internal/colorspace.rs"]
mod colorspace;

#[path = "../../src/internal/rasterizer.rs"]
mod rasterizer;

#[path = "../../src/internal/glyph_rasterizer.rs"]
mod glyph_rasterizer;

use std::fs;
use std::path::PathBuf;

const PSNR_MIN: f64 = 40.0;

fn corpus_dir() -> PathBuf {
    if let Ok(env_dir) = std::env::var("SPECDEV_CORPUS_DIR") {
        if !env_dir.is_empty() {
            return PathBuf::from(env_dir);
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("glyph_rasterizer")
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
fn lossy_oracle_agreement_over_corpus() {
    let dir = corpus_dir();
    assert!(dir.is_dir(),
            "corpus dir not found: {} \
             (set SPECDEV_CORPUS_DIR or vendor tests/fixtures/glyph_rasterizer/)",
            dir.display());

    let mut fixtures: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("read corpus dir")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("bin"))
        .collect();
    fixtures.sort();
    assert!(!fixtures.is_empty(),
            "no .bin fixtures under {}", dir.display());

    let mut checked = 0_usize;
    let mut failed = 0_usize;
    for path in &fixtures {
        eprintln!("TRY {}", path.display());
        let pixels_path = path.with_extension("pixels");
        assert!(pixels_path.is_file(),
                "missing sidecar: {}", pixels_path.display());

        let bin_bytes = fs::read(path).expect("read .bin");
        let expected = fs::read(&pixels_path).expect("read .pixels sidecar");

        let decoded = glyph_rasterizer::decode(&bin_bytes)
            .unwrap_or_else(|e| panic!("decode failed for {}: {}",
                                       path.display(), e));
        if decoded.pixels.len() != expected.len() {
            eprintln!(
                "MISMATCH {}: pixel-buffer length differs \
                 (decoded {} vs expected {})",
                path.display(), decoded.pixels.len(), expected.len());
            failed += 1;
            checked += 1;
            continue;
        }
        let p = psnr(&decoded.pixels, &expected);
        if p < PSNR_MIN {
            eprintln!(
                "MISMATCH {}: PSNR={p:.2} below threshold {PSNR_MIN}",
                path.display());
            failed += 1;
        }
        checked += 1;
    }
    eprintln!("glyph_rasterizer lossy_oracle_agreement: \
               checked={} failed={} psnr_min={}",
              checked, failed, PSNR_MIN);
    assert_eq!(failed, 0, "{} fixture mismatches", failed);
}
