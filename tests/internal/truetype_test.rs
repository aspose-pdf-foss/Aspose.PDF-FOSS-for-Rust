// Hand-authored gate driver — two_oracle_agreement gate.
// Primitive: truetype
// Freeze gate: each `<name>.ttf` fixture has a paired
// `<name>.canonical` sidecar containing the text manifest
// produced by fontTools + freetype-py (the two oracles, agreed
// byte-for-byte at fixture-authoring time). The gate parses
// each fixture, formats via `to_canonical`, and compares to the
// sidecar exactly.

#[path = "../../src/internal/truetype.rs"]
mod truetype;

use std::fs;
use std::path::PathBuf;

const FIXTURE_EXT: &str = "ttf";

fn corpus_dir() -> PathBuf {
    if let Ok(env_dir) = std::env::var("SPECDEV_CORPUS_DIR") {
        if !env_dir.is_empty() {
            return PathBuf::from(env_dir);
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("truetype")
}

#[test]
fn two_oracle_agreement_over_corpus() {
    let dir = corpus_dir();
    assert!(dir.is_dir(),
            "corpus dir not found: {} \
             (set SPECDEV_CORPUS_DIR or vendor tests/fixtures/truetype/)",
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

    let mut checked = 0_usize;
    for path in &fixtures {
        let canon_path = path.with_extension("canonical");
        assert!(canon_path.is_file(),
                "missing canonical sidecar for {}", path.display());
        let input = fs::read(path).expect("read fixture");
        let want = fs::read_to_string(&canon_path)
            .expect("read canonical sidecar");
        let parsed = truetype::parse(&input)
            .unwrap_or_else(|e| panic!(
                "parse failed for {}: {}", path.display(), e));
        let got = truetype::to_canonical(&parsed);
        if got != want {
            // Render a small diff context so a regression is
            // immediately legible.
            let want_lines: Vec<&str> = want.split('\n').collect();
            let got_lines: Vec<&str> = got.split('\n').collect();
            let max = want_lines.len().max(got_lines.len());
            let mut first_div = None;
            for i in 0..max {
                let w = want_lines.get(i).copied().unwrap_or("<EOF>");
                let g = got_lines.get(i).copied().unwrap_or("<EOF>");
                if w != g {
                    first_div = Some((i, w.to_string(), g.to_string()));
                    break;
                }
            }
            panic!(
                "canonical mismatch for {}\n  first diverging line: {:?}\n  expected len={} got len={}",
                path.display(), first_div, want.len(), got.len());
        }
        checked += 1;
    }
    eprintln!("truetype two_oracle_agreement: checked={} failed=0",
              checked);
}
