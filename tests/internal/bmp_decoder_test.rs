// Hand-authored gate driver — lossy_oracle_agreement gate.
// Primitive: bmp_decoder
// Freeze gate: per-fixture PSNR ≥ 50 dB against a `.pixels`
// sidecar authored from the source RGBA buffer the fixture
// writer encoded into the BMP (source-as-oracle, since BMP is
// lossless on every in-scope variant — for a correct decoder
// real-world PSNR is infinity / byte-identical).
//
// Each fixture pair is `<name>.bmp` + `<name>.pixels`; the
// sidecar is raw RGBA8 (`width*height*4` bytes), row-major
// top-to-bottom, R before G before B before A. Width and height
// are recovered from the BMP itself (BITMAPINFOHEADER.biWidth /
// biHeight) so the driver does not need a separate metadata
// file.
//
// The 50 dB floor is a safety net: bottom-up row inversion bugs,
// 24-bit row-padding miscalculations, and BGR/RGB channel-order
// swaps all push PSNR well below 50 dB. A correct decoder
// produces byte-identical output (PSNR == infinity), which we
// also assert directly as the canonical pass criterion.

#[path = "../../src/internal/bmp_decoder.rs"]
mod bmp_decoder;

use bmp_decoder::decode;

use std::fs;
use std::path::PathBuf;

const PSNR_FLOOR_DB: f64 = 50.0;

fn corpus_dir() -> PathBuf {
    if let Ok(env_dir) = std::env::var("SPECDEV_CORPUS_DIR") {
        if !env_dir.is_empty() {
            return PathBuf::from(env_dir);
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("bmp_decoder")
}

/// Peak signal-to-noise ratio in decibels between two equal-
/// length byte buffers interpreted as 8-bit samples (MAX = 255).
/// Returns f64::INFINITY when the buffers are byte-identical.
fn psnr_db(a: &[u8], b: &[u8]) -> f64 {
    assert_eq!(a.len(), b.len(), "psnr inputs differ in length");
    let mut sum_sq: u64 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        let d = *x as i32 - *y as i32;
        sum_sq += (d * d) as u64;
    }
    if sum_sq == 0 {
        return f64::INFINITY;
    }
    let mse = sum_sq as f64 / a.len() as f64;
    10.0 * (255.0 * 255.0 / mse).log10()
}

#[test]
fn lossy_oracle_agreement_over_corpus() {
    let dir = corpus_dir();
    assert!(dir.is_dir(),
            "corpus dir not found: {} \
             (set SPECDEV_CORPUS_DIR or vendor tests/fixtures/bmp_decoder/)",
            dir.display());

    let mut bmps: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("read corpus dir")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("bmp"))
        .collect();
    bmps.sort();

    assert!(!bmps.is_empty(),
            "no .bmp fixtures under {}", dir.display());

    let mut failures: Vec<String> = Vec::new();
    let mut checked = 0_usize;
    for bmp_path in &bmps {
        let name = bmp_path.file_name().unwrap().to_string_lossy().into_owned();
        let sidecar_path = bmp_path.with_extension("pixels");
        let bmp_bytes = fs::read(bmp_path).expect("read bmp");
        let oracle = fs::read(&sidecar_path).expect("read sidecar");

        let decoded = match decode(&bmp_bytes) {
            Ok(img) => img,
            Err(e) => {
                failures.push(format!("{name}: decode failed: {e}"));
                continue;
            }
        };

        let expected_bytes = decoded.width * decoded.height * 4;
        if decoded.pixels.len() != expected_bytes {
            failures.push(format!(
                "{name}: pixels.len {} != width*height*4 = {}",
                decoded.pixels.len(), expected_bytes));
            continue;
        }
        if decoded.components != 4 {
            failures.push(format!(
                "{name}: components {} != 4",
                decoded.components));
        }
        if oracle.len() != decoded.pixels.len() {
            failures.push(format!(
                "{name}: sidecar length {} does not match decoded {} \
                 (decoder reports {}x{})",
                oracle.len(), decoded.pixels.len(),
                decoded.width, decoded.height));
            continue;
        }

        // Canonical pass criterion — BMP is lossless on every
        // in-scope variant, so a correct decoder produces output
        // byte-identical to the source-authored sidecar.
        if decoded.pixels != oracle {
            failures.push(format!(
                "{name}: decoded pixels diverge from source-authored sidecar"));
        }

        // Safety-net floor — PSNR ≥ 50 dB. A byte-identical
        // decode is PSNR == infinity which trivially clears.
        let psnr = psnr_db(&decoded.pixels, &oracle);
        if psnr < PSNR_FLOOR_DB {
            failures.push(format!(
                "{name}: PSNR {psnr:.2} dB < floor {PSNR_FLOOR_DB} dB"));
        }
        checked += 1;
    }

    eprintln!("bmp_decoder lossy_oracle_agreement: checked={} failures={}",
              checked, failures.len());
    assert!(failures.is_empty(),
            "fixture violations:\n  {}", failures.join("\n  "));
}
