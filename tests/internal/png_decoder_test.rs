// Hand-authored gate driver — lossy_oracle_agreement gate.
// Primitive: png_decoder
// Freeze gate: for each `Feedstock/PDF/Fixtures/png_decoder/*.png`
// fixture X with sidecar X.pixels (raw RGBA8, top-to-bottom,
// width*height*4 bytes), assert that decode(X).pixels matches
// X.pixels by PSNR ≥ 50 dB. PNG is lossless; real-world PSNR
// is infinity (byte-identical) for every in-scope fixture.
// The 50 dB floor is a safety net against subtle filter-inverse
// off-by-one bugs.
//
// Composes the frozen flate + png_decoder.

#[path = "../../src/internal/flate.rs"]
mod flate;

#[path = "../../src/internal/png_decoder.rs"]
mod png_decoder;

use std::fs;
use std::path::PathBuf;

const PSNR_MIN: f64 = 50.0;

fn corpus_dir() -> PathBuf {
    if let Ok(env_dir) = std::env::var("SPECDEV_CORPUS_DIR") {
        if !env_dir.is_empty() {
            return PathBuf::from(env_dir);
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("png_decoder")
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
             (set SPECDEV_CORPUS_DIR or vendor tests/fixtures/png_decoder/)",
            dir.display());

    let mut fixtures: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("read corpus dir")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("png"))
        .collect();
    fixtures.sort();
    assert!(!fixtures.is_empty(),
            "no .png fixtures under {}", dir.display());

    let mut checked = 0_usize;
    for path in &fixtures {
        eprintln!("TRY {}", path.display());
        let pixels_path = path.with_extension("pixels");
        assert!(pixels_path.is_file(),
                "missing sidecar: {}", pixels_path.display());

        let png_bytes = fs::read(path).expect("read png");
        let expected = fs::read(&pixels_path).expect("read pixels sidecar");

        let decoded = png_decoder::decode(&png_bytes)
            .unwrap_or_else(|e| panic!("decode failed for {}: {}",
                                       path.display(), e));
        assert_eq!(
            decoded.pixels.len(),
            expected.len(),
            "pixel-buffer length mismatch for {} (decoded {} vs expected {})",
            path.display(),
            decoded.pixels.len(),
            expected.len(),
        );
        let p = psnr(&decoded.pixels, &expected);
        assert!(
            p >= PSNR_MIN,
            "PSNR {p:.2} dB below floor {PSNR_MIN} dB for {}",
            path.display(),
        );
        checked += 1;
    }
    eprintln!("png_decoder lossy_oracle_agreement: checked={} failed=0", checked);
}
