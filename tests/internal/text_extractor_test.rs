// Hand-authored gate driver — two_oracle_agreement gate.
// Primitive: text_extractor
// Freeze gate: for each `Feedstock/PDF/Fixtures/text_extractor/*.pdf`
// fixture X with sidecar X.canonical, asserts that
// to_canonical(parse(X)) == X.canonical byte-for-byte. The
// .canonical sidecar is the agreed output of pdftotext + pdfminer.six
// (computed at fixture-authoring time).
//
// text_extractor composes the entire foundation read-side text
// path; the integration-test crate pulls in lexer + flate + xref
// + objects + content_stream + to_unicode + encoding_tables(+data)
// + agl(+data) + crypt_filter (which itself wants aes_cbc + rc4 +
// digest + ascii85 + asciihex + lzw + runlength + transform — but
// we only need direct deps for THIS body, since the lib build
// already covers transitive linkage and our `#[path]` mounts
// only what text_extractor.rs `use`s).

#![allow(clippy::duplicate_mod)] // internal modules re-included by #[path] for white-box tests
#![allow(dead_code)] // this white-box crate exercises only part of each #[path]-included module

#[path = "../../src/internal/lexer.rs"]
mod lexer;

#[path = "../../src/internal/flate.rs"]
mod flate;

#[path = "../../src/internal/xref.rs"]
mod xref;

#[path = "../../src/internal/objects.rs"]
mod objects;

#[path = "../../src/internal/content_stream.rs"]
mod content_stream;

#[path = "../../src/internal/to_unicode.rs"]
mod to_unicode;

#[path = "../../src/internal/encoding_tables_data.rs"]
mod encoding_tables_data;

#[path = "../../src/internal/encoding_tables.rs"]
mod encoding_tables;

#[path = "../../src/internal/agl_data.rs"]
mod agl_data;

#[path = "../../src/internal/agl.rs"]
mod agl;

#[path = "../../src/internal/aes_cbc.rs"]
mod aes_cbc;

#[path = "../../src/internal/rc4.rs"]
mod rc4;

#[path = "../../src/internal/digest.rs"]
mod digest;

#[path = "../../src/internal/crypt_filter.rs"]
mod crypt_filter;

#[path = "../../src/internal/standard14_metrics.rs"]
mod standard14_metrics;

#[path = "../../src/internal/text_extractor.rs"]
mod text_extractor;

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
        .join("text_extractor")
}

#[test]
fn two_oracle_agreement_over_corpus() {
    let dir = corpus_dir();
    assert!(dir.is_dir(),
            "corpus dir not found: {} \
             (set SPECDEV_CORPUS_DIR or vendor tests/fixtures/text_extractor/)",
            dir.display());

    let mut fixtures: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("read corpus dir")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("pdf"))
        .collect();
    fixtures.sort();

    assert!(!fixtures.is_empty(),
            "no .pdf fixtures under {}", dir.display());

    let mut checked = 0_usize;
    for path in &fixtures {
        eprintln!("TRY {}", path.display());
        let canonical_path = path.with_extension("canonical");
        assert!(canonical_path.is_file(),
                "missing sidecar: {}", canonical_path.display());

        let input = fs::read(path).expect("read fixture");
        let expected = fs::read_to_string(&canonical_path).expect("read sidecar");

        let pages = text_extractor::parse(&input)
            .unwrap_or_else(|e| panic!("parse failed for {}: {}",
                                       path.display(), e));
        let got = text_extractor::to_canonical(&pages);

        if got != expected {
            panic!(
                "canonical mismatch for {}:\n--- expected ({} bytes) ---\n{:?}\n--- got ({} bytes) ---\n{:?}",
                path.display(),
                expected.len(), expected,
                got.len(), got);
        }
        checked += 1;
    }
    eprintln!("text_extractor two_oracle_agreement: checked={} failed=0", checked);
}

// Encrypted-PDF extraction (mirrors the C++ text_extractor_encrypted
// regression). `hello_world_aes128.pdf` is an AES-128 (V=4 R=4) document
// produced from hello_world.pdf by this lib's own encrypt-on-save; the
// file key below is its recovered key. `parse_with_key` decrypts +
// extracts; `parse` (no key) rejects it; an empty key fails.
#[test]
fn parse_with_key_extracts_encrypted_text() {
    let pdf = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/text_extractor_encrypted/hello_world_aes128.pdf"),
    )
    .unwrap();
    let key: Vec<u8> = (0..16)
        .map(|i| u8::from_str_radix(&"8c9f79a6a786e5e5aab1d5a935e3f77b"[i * 2..i * 2 + 2], 16).unwrap())
        .collect();

    let pages = text_extractor::parse_with_key(&pdf, &key).unwrap();
    assert_eq!(pages.len(), 1);
    assert!(pages[0].contains("Hello"), "got: {:?}", pages[0]);

    // Plaintext parse rejects an encrypted document.
    assert!(text_extractor::parse(&pdf).is_err());
    // An empty / wrong-length key fails decryption.
    assert!(text_extractor::parse_with_key(&pdf, &[]).is_err());
}
