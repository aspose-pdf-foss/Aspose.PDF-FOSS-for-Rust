// Hand-authored gate driver — contract_validation gate.
// Primitive: encoding
// Freeze gate: for each fixture under
// `Feedstock/PDF/Fixtures/encoding/*.txt` (raw UTF-8 bytes ARE
// the test input string), evaluate the BCL contract invariants
// against each of the five v1 charsets:
//   * web_name_canonical / code_page_canonical / preamble_canonical
//   * factory_singleton (get_encoding(canonical_name) == factory)
//   * byte_count_matches (get_byte_count == get_bytes().len())
//   * char_count_matches (get_char_count == get_string().chars().count())
//   * roundtrip_lossless (when input is fully representable)
//   * single_byte_per_codepoint (latin-1 / win-1252 invariant)

#[path = "../../src/internal/encoding.rs"]
mod encoding;

use std::fs;
use std::path::PathBuf;

use encoding::Encoding;

fn corpus_dir() -> PathBuf {
    if let Ok(env_dir) = std::env::var("SPECDEV_CORPUS_DIR") {
        if !env_dir.is_empty() {
            return PathBuf::from(env_dir);
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("encoding")
}

struct Charset {
    tag: &'static str,
    expected_web_name: &'static str,
    expected_code_page: u32,
    expected_preamble: &'static [u8],
    canonical_alias: &'static str,
    enc: Encoding,
}

const CHARSETS: &[Charset] = &[
    Charset {
        tag: "utf-8",
        expected_web_name: "utf-8",
        expected_code_page: 65001,
        expected_preamble: &[0xEF, 0xBB, 0xBF],
        canonical_alias: "utf-8",
        enc: Encoding::Utf8,
    },
    Charset {
        tag: "utf-16-le",
        expected_web_name: "utf-16",
        expected_code_page: 1200,
        expected_preamble: &[0xFF, 0xFE],
        canonical_alias: "unicode",
        enc: Encoding::Utf16Le,
    },
    Charset {
        tag: "utf-16-be",
        expected_web_name: "utf-16BE",
        expected_code_page: 1201,
        expected_preamble: &[0xFE, 0xFF],
        canonical_alias: "utf-16BE",
        enc: Encoding::Utf16Be,
    },
    Charset {
        tag: "latin-1",
        expected_web_name: "iso-8859-1",
        expected_code_page: 28591,
        expected_preamble: &[],
        canonical_alias: "iso-8859-1",
        enc: Encoding::Latin1,
    },
    Charset {
        tag: "windows-1252",
        expected_web_name: "windows-1252",
        expected_code_page: 1252,
        expected_preamble: &[],
        canonical_alias: "windows-1252",
        enc: Encoding::Windows1252,
    },
];

fn all_representable(text: &str, tag: &str) -> bool {
    match tag {
        "utf-8" | "utf-16-le" | "utf-16-be" => true,
        "latin-1" => text.chars().all(|c| (c as u32) <= 0xFF),
        "windows-1252" => {
            let enc = Encoding::Windows1252;
            for c in text.chars() {
                let cp = c as u32;
                if cp == 0x3F {
                    continue; // literal '?' is representable
                }
                if cp < 0x80 || (0xA0..=0xFF).contains(&cp) {
                    continue;
                }
                let probe: String = std::iter::once(c).collect();
                let bytes = enc.get_bytes(&probe);
                if bytes.len() == 1 && bytes[0] == 0x3F {
                    return false;
                }
            }
            true
        }
        _ => false,
    }
}

#[test]
fn contract_validation_over_corpus() {
    let dir = corpus_dir();
    assert!(dir.is_dir(),
            "corpus dir not found: {} \
             (set SPECDEV_CORPUS_DIR or vendor tests/fixtures/encoding/)",
            dir.display());

    let mut fixtures: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("read corpus dir")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("txt"))
        .collect();
    fixtures.sort();

    assert!(!fixtures.is_empty(),
            "no .txt fixtures under {}", dir.display());

    let mut total_invariants = 0usize;
    let mut total_failed = 0usize;

    for path in &fixtures {
        eprintln!("TRY {}", path.display());
        let bytes = fs::read(path).expect("read fixture");
        // Decode the fixture via our own UTF-8 codec to recover
        // the test string — bytes ARE the input, interpreted as
        // UTF-8.
        let text = Encoding::Utf8.get_string(&bytes);
        let mut local_failed: Vec<String> = Vec::new();

        for cs in CHARSETS {
            let tag = cs.tag;
            // (1) WebName
            total_invariants += 1;
            if cs.enc.web_name() != cs.expected_web_name {
                total_failed += 1;
                local_failed.push(format!("{tag}/web_name_canonical"));
            }
            // (2) CodePage
            total_invariants += 1;
            if cs.enc.code_page() != cs.expected_code_page {
                total_failed += 1;
                local_failed.push(format!("{tag}/code_page_canonical"));
            }
            // (3) Preamble
            total_invariants += 1;
            if cs.enc.preamble() != cs.expected_preamble {
                total_failed += 1;
                local_failed.push(format!("{tag}/preamble_canonical"));
            }
            // (4) GetEncoding factory returns same value
            total_invariants += 1;
            match Encoding::get_encoding(cs.canonical_alias) {
                Ok(found) if found == cs.enc => {}
                _ => {
                    total_failed += 1;
                    local_failed.push(format!("{tag}/factory_singleton"));
                }
            }
            // (5) byte-count matches GetBytes length
            total_invariants += 1;
            let bytes_out = cs.enc.get_bytes(&text);
            if cs.enc.get_byte_count(&text) != bytes_out.len() {
                total_failed += 1;
                local_failed.push(format!("{tag}/byte_count_matches"));
            }
            // (6) char-count matches GetString length (chars)
            total_invariants += 1;
            let decoded = cs.enc.get_string(&bytes_out);
            if cs.enc.get_char_count(&bytes_out) != decoded.chars().count() {
                total_failed += 1;
                local_failed.push(format!("{tag}/char_count_matches"));
            }
            // (7) Roundtrip when fully representable, OR
            //     single-byte-per-codepoint for latin-1 / win-1252.
            if all_representable(&text, tag) {
                total_invariants += 1;
                if decoded != text {
                    total_failed += 1;
                    local_failed.push(format!("{tag}/roundtrip_lossless"));
                }
            } else if tag == "latin-1" || tag == "windows-1252" {
                total_invariants += 1;
                let cp_count = text.chars().count();
                if bytes_out.len() != cp_count {
                    total_failed += 1;
                    local_failed.push(format!("{tag}/single_byte_per_codepoint"));
                }
            }
        }

        if !local_failed.is_empty() {
            panic!(
                "fixture {} failed invariants: {}",
                path.display(),
                local_failed.join(", ")
            );
        }
    }
    eprintln!(
        "encoding contract_validation: invariants={} failed={}",
        total_invariants, total_failed
    );
    assert_eq!(total_failed, 0);
}
