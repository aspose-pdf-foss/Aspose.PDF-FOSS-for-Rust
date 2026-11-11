// Hand-authored gate driver — byte_roundtrip gate.
// Primitive: lexer
// Freeze gate: for every fixture in the corpus,
//   `emit(lex(bytes)) == bytes`
// byte-for-byte. No external oracle — the gate is self-
// certifying because the lexer's trivia-preservation invariant
// makes the emitter a pure inverse: every byte of input ends
// up inside exactly one token, and the emitter just
// concatenates each token's byte view.
//
// Stronger checks bundled with the byte-equality assertion:
//   * Token offsets are monotonically non-decreasing and equal
//     the cumulative byte view length up to that token (i.e.
//     no gaps and no overlaps).
//   * Total bytes across all tokens equals input.len().
//   * Every Token::bytes is non-empty (zero-length tokens
//     would be ambiguous and could mask lexer bugs).
//
// Per the spec, this is the one foundation primitive where
// byte-identity is both achievable and meaningful — higher
// primitives use two_oracle_agreement / lossy_oracle_agreement.

#[path = "../../src/internal/lexer.rs"]
mod lexer;

use lexer::{emit, lex};

use std::fs;
use std::path::PathBuf;

fn corpus_dir() -> PathBuf {
    if let Ok(env_dir) = std::env::var("SPECDEV_CORPUS_DIR") {
        if !env_dir.is_empty() {
            return PathBuf::from(env_dir);
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("lexer")
}

#[test]
fn byte_roundtrip_over_corpus() {
    let dir = corpus_dir();
    assert!(dir.is_dir(),
            "corpus dir not found: {} \
             (set SPECDEV_CORPUS_DIR or vendor tests/fixtures/lexer/)",
            dir.display());

    let mut pdfs: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("read corpus dir")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("pdf"))
        .collect();
    pdfs.sort();

    assert!(!pdfs.is_empty(),
            "no .pdf fixtures under {}", dir.display());

    let mut failures: Vec<String> = Vec::new();
    let mut checked = 0_usize;
    for pdf_path in &pdfs {
        let name = pdf_path.file_name().unwrap().to_string_lossy().into_owned();
        let bytes = fs::read(pdf_path).expect("read pdf");
        let tokens = lex(&bytes);

        // Every token must hold a non-empty byte view.
        if let Some(empty_idx) = tokens.iter().position(|t| t.bytes.is_empty()) {
            failures.push(format!(
                "{name}: token #{empty_idx} ({:?}) has empty byte view",
                tokens[empty_idx].kind));
            continue;
        }

        // Offsets must equal cumulative position (no gaps / no
        // overlaps).
        let mut cursor = 0_usize;
        let mut offset_ok = true;
        for (i, t) in tokens.iter().enumerate() {
            if t.offset != cursor {
                failures.push(format!(
                    "{name}: token #{i} ({:?}) offset {} != cumulative {}",
                    t.kind, t.offset, cursor));
                offset_ok = false;
                break;
            }
            cursor += t.bytes.len();
        }
        if !offset_ok {
            continue;
        }
        if cursor != bytes.len() {
            failures.push(format!(
                "{name}: total token bytes {} != input bytes {}",
                cursor, bytes.len()));
            continue;
        }

        // Canonical byte_roundtrip property.
        let reemitted = emit(&tokens);
        if reemitted != bytes {
            // Surface first diverging offset for diagnosis.
            let first_diff = bytes.iter().zip(reemitted.iter())
                .position(|(a, b)| a != b)
                .unwrap_or(bytes.len().min(reemitted.len()));
            failures.push(format!(
                "{name}: byte_roundtrip violated; first divergence at offset {} \
                 (expected {} bytes, got {} bytes)",
                first_diff, bytes.len(), reemitted.len()));
            continue;
        }
        checked += 1;
    }

    eprintln!("lexer byte_roundtrip: checked={} failures={}",
              checked, failures.len());
    assert!(failures.is_empty(),
            "fixture violations:\n  {}", failures.join("\n  "));
}
