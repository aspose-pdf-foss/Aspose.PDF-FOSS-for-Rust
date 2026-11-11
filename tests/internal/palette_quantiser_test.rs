// Hand-authored gate driver — contract_validation gate.
// Primitive: palette_quantiser
// Freeze gate: per-fixture invariants (no oracle, no
// byte-equality across implementations — the BCL contract pins
// SHAPE, not algorithm).
//
// Wire layout per fixture (.bin), little-endian:
//   +0  uint32 width
//   +4  uint32 height
//   +8  uint32 max_colors
//  +12  uint8[w*h*4] rgba
//
// Six invariants from the spec yaml:
//   1. palette_size_bound
//   2. indices_length_matches
//   3. indices_in_range
//   4. determinism
//   5. exact_match_self_identity
//   6. palette_covers_unique_colors_when_feasible (relaxable —
//      only when distinct(rgb) ≤ max_colors)
//
// First contract_validation gate on the rust track. Pattern:
//
//   * For each fixture, parse wire format → (w, h, max_colors,
//     rgba).
//   * Call quantise.
//   * Run every applicable invariant; collect failures.
//   * Print per-fixture pass/fail summary, fail the test with
//     a per-invariant message on any violation.

#[path = "../../src/internal/palette_quantiser.rs"]
mod palette_quantiser;

use palette_quantiser::{quantise, IndexedImage};

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

const FIXTURE_EXT: &str = "bin";

fn corpus_dir() -> PathBuf {
    if let Ok(env_dir) = std::env::var("SPECDEV_CORPUS_DIR") {
        if !env_dir.is_empty() {
            return PathBuf::from(env_dir);
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("palette_quantiser")
}

struct Fixture {
    width: usize,
    height: usize,
    max_colors: usize,
    rgba: Vec<u8>,
}

fn parse_fixture(path: &std::path::Path) -> Fixture {
    let bytes = fs::read(path).expect("read fixture");
    assert!(bytes.len() >= 12,
            "fixture {} too short for 12-byte header", path.display());
    let width = u32::from_le_bytes(bytes[0..4].try_into().unwrap()) as usize;
    let height = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
    let max_colors = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    let expected = 12 + width * height * 4;
    assert_eq!(bytes.len(), expected,
               "fixture {} declares {}x{} but pixel buffer length is {}",
               path.display(), width, height, bytes.len() - 12);
    let rgba = bytes[12..].to_vec();
    Fixture { width, height, max_colors, rgba }
}

fn distinct_rgb(rgba: &[u8], pixel_count: usize) -> BTreeSet<[u8; 3]> {
    (0..pixel_count)
        .map(|i| [rgba[4 * i], rgba[4 * i + 1], rgba[4 * i + 2]])
        .collect()
}

fn squared_dist(a: [u8; 3], b: [u8; 3]) -> i32 {
    let dr = a[0] as i32 - b[0] as i32;
    let dg = a[1] as i32 - b[1] as i32;
    let db = a[2] as i32 - b[2] as i32;
    dr * dr + dg * dg + db * db
}

fn validate(fx: &Fixture, img: &IndexedImage, name: &str) -> Vec<String> {
    let mut failures = Vec::new();

    // 1. palette_size_bound.
    if img.palette.is_empty() || img.palette.len() > fx.max_colors {
        failures.push(format!(
            "{name}: palette_size_bound — got {}, expected 1..={}",
            img.palette.len(), fx.max_colors));
    }
    // 2. indices_length_matches.
    let pixel_count = fx.width * fx.height;
    if img.indices.len() != pixel_count {
        failures.push(format!(
            "{name}: indices_length_matches — got {}, expected {}",
            img.indices.len(), pixel_count));
    }
    // 3. indices_in_range.
    let palette_len = img.palette.len();
    for (px, &i) in img.indices.iter().enumerate() {
        if (i as usize) >= palette_len {
            failures.push(format!(
                "{name}: indices_in_range — index {} at pixel {} >= palette.len {}",
                i, px, palette_len));
            break;
        }
    }
    // 5. exact_match_self_identity. For each input pixel whose
    // RGB appears verbatim in the palette, the index must point
    // at that entry. (Equivalent: the palette entry chosen has
    // zero RGB distance to the input pixel.)
    let palette_set: BTreeSet<[u8; 3]> = img.palette.iter().copied().collect();
    for px in 0..pixel_count {
        let p = [fx.rgba[4 * px], fx.rgba[4 * px + 1], fx.rgba[4 * px + 2]];
        if palette_set.contains(&p) {
            let idx = img.indices[px] as usize;
            if idx >= palette_len || img.palette[idx] != p {
                failures.push(format!(
                    "{name}: exact_match_self_identity — pixel {} \
                     RGB {:?} appears in palette but maps to index {} = {:?}",
                    px, p, idx,
                    if idx < palette_len { Some(img.palette[idx]) } else { None }));
                break;
            }
        }
    }
    // 5b. (Stronger property the spec implies but doesn't pin:
    // every assigned index minimises squared distance over the
    // palette. Catches a remap pass that goes off the rails.)
    for px in 0..pixel_count {
        let p = [fx.rgba[4 * px], fx.rgba[4 * px + 1], fx.rgba[4 * px + 2]];
        let assigned = img.indices[px] as usize;
        if assigned >= palette_len { continue; }     // covered above
        let assigned_d = squared_dist(p, img.palette[assigned]);
        for q in &img.palette {
            if squared_dist(p, *q) < assigned_d {
                failures.push(format!(
                    "{name}: pixel {} RGB {:?} assigned palette[{}]={:?} \
                     (d²={}) but palette entry {:?} is closer (d²={})",
                    px, p, assigned, img.palette[assigned], assigned_d,
                    q, squared_dist(p, *q)));
                break;
            }
        }
    }
    // 6. palette_covers_unique_colors_when_feasible (relaxable).
    let distinct = distinct_rgb(&fx.rgba, pixel_count);
    if distinct.len() <= fx.max_colors {
        if img.palette.len() > distinct.len() {
            failures.push(format!(
                "{name}: palette_covers_unique_colors_when_feasible — \
                 palette.len {} > distinct input count {} (max_colors={})",
                img.palette.len(), distinct.len(), fx.max_colors));
        }
        for c in &distinct {
            if !palette_set.contains(c) {
                failures.push(format!(
                    "{name}: palette_covers_unique_colors_when_feasible — \
                     distinct input colour {:?} missing from palette",
                    c));
                break;
            }
        }
    }
    failures
}

#[test]
fn contract_validation_over_corpus() {
    let dir = corpus_dir();
    assert!(dir.is_dir(),
            "corpus dir not found: {} \
             (set SPECDEV_CORPUS_DIR or vendor tests/fixtures/palette_quantiser/)",
            dir.display());

    let mut fixtures: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("read corpus dir")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .filter(|p| p.extension()
                     .and_then(|s| s.to_str()) == Some(FIXTURE_EXT))
        .collect();
    fixtures.sort();

    assert!(!fixtures.is_empty(),
            "no .{} fixtures under {}", FIXTURE_EXT, dir.display());

    let mut all_failures = Vec::new();
    let mut checked = 0_usize;
    for path in &fixtures {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let fx = parse_fixture(path);
        let img = quantise(&fx.rgba, fx.width, fx.height, fx.max_colors)
            .expect("quantise on valid fixture");
        // 4. determinism — run twice, compare bit-for-bit.
        let img2 = quantise(&fx.rgba, fx.width, fx.height, fx.max_colors)
            .expect("quantise second call");
        if img != img2 {
            all_failures.push(format!(
                "{name}: determinism — second call produced different output"));
        }
        all_failures.extend(validate(&fx, &img, &name));
        checked += 1;
    }
    eprintln!("palette_quantiser contract_validation: checked={} failures={}",
              checked, all_failures.len());
    assert!(all_failures.is_empty(),
            "contract violations:\n  {}", all_failures.join("\n  "));
}
