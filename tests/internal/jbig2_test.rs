// Hand-authored gate driver — lossy_oracle_agreement gate.
// Primitive: jbig2
// Freeze gate: for each `tests/fixtures/jbig2/*.bin` fixture X
// with sidecar X.pixels (packed MSB-first bilevel scanlines,
// ((width + 7) / 8) * height bytes; JBIG2 polarity 1 = black,
// 0 = white), assert that decode(X).pixels matches X.pixels by
// PSNR >= 50 dB.
//
// PSNR floor 50 dB on bilevel data is effectively bit-equality
// (RMSE ~0.81 on the 0..255 scale). JBIG2 output is bit-
// deterministic by construction (T.88 + T.82 Annex E fully
// specify every decoded bit), so the threshold absorbs no
// real drift — it is the same kind/units used by the cpp / cs
// / typescript siblings to keep the gate template uniform.

#[path = "../../src/internal/jbig2.rs"]
mod jbig2;

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
        .join("jbig2")
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
    assert!(
        dir.is_dir(),
        "corpus dir not found: {} \
         (set SPECDEV_CORPUS_DIR or vendor tests/fixtures/jbig2/)",
        dir.display()
    );

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
    for path in &fixtures {
        eprintln!("TRY {}", path.display());
        let pixels_path = path.with_extension("pixels");
        assert!(pixels_path.is_file(),
                "missing sidecar: {}", pixels_path.display());

        let bin_bytes = fs::read(path).expect("read jbig2 segment stream");
        let expected = fs::read(&pixels_path).expect("read pixels sidecar");

        let decoded = jbig2::decode(&bin_bytes)
            .unwrap_or_else(|e| panic!("decode failed for {}: {}",
                                       path.display(), e));
        assert_eq!(
            decoded.pixels.len(),
            expected.len(),
            "pixel-buffer length mismatch for {} \
             (decoded {} vs expected {}; width={} height={} components={})",
            path.display(),
            decoded.pixels.len(),
            expected.len(),
            decoded.width,
            decoded.height,
            decoded.components,
        );
        let p = psnr(&decoded.pixels, &expected);
        assert!(
            p >= PSNR_MIN,
            "PSNR {p:.2} dB below floor {PSNR_MIN} dB for {}",
            path.display(),
        );
        eprintln!("  OK  {}  PSNR={:.2} dB",
                  path.file_name().unwrap().to_string_lossy(), p);
        checked += 1;
    }
    eprintln!("jbig2 lossy_oracle_agreement: checked={} failed=0", checked);
}
