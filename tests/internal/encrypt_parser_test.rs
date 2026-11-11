// Hand-authored gate driver — field_match_agreement gate.
// Primitive: encrypt_parser
// Freeze gate: for each `.case` text fixture under
// `Feedstock/PDF/Fixtures/encrypt_parser/cases/`, parse the
// declared inputs (EncryptDict + password + file_id) and
// expected outputs (authenticated, used_owner_path, file_key),
// run authenticate(), and compare each declared output field
// against the expected value. Pikepdf is the authoring oracle
// — its expected values are baked into the .case sidecars.

#[path = "../../src/internal/aes_cbc.rs"]
mod aes_cbc;

#[path = "../../src/internal/rc4.rs"]
mod rc4;

#[path = "../../src/internal/digest.rs"]
mod digest;

#[path = "../../src/internal/encrypt_parser.rs"]
mod encrypt_parser;

use std::fs;
use std::path::PathBuf;

use encrypt_parser::{authenticate, EncryptDict};

fn corpus_dir() -> PathBuf {
    if let Ok(env_dir) = std::env::var("SPECDEV_CORPUS_DIR") {
        if !env_dir.is_empty() {
            return PathBuf::from(env_dir);
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("encrypt_parser")
        .join("cases")
}

fn unhex(s: &str) -> Vec<u8> {
    let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    let mut out = Vec::with_capacity(s.len() / 2);
    let bytes = s.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        let h = match bytes[i] {
            b'0'..=b'9' => bytes[i] - b'0',
            b'a'..=b'f' => bytes[i] - b'a' + 10,
            b'A'..=b'F' => bytes[i] - b'A' + 10,
            _ => panic!("bad hex byte"),
        };
        let l = match bytes[i + 1] {
            b'0'..=b'9' => bytes[i + 1] - b'0',
            b'a'..=b'f' => bytes[i + 1] - b'a' + 10,
            b'A'..=b'F' => bytes[i + 1] - b'A' + 10,
            _ => panic!("bad hex byte"),
        };
        out.push((h << 4) | l);
        i += 2;
    }
    out
}

fn hex_to_string(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

#[derive(Default)]
struct Case {
    v: u32,
    r: u32,
    length_bits: u32,
    p: i32,
    o: Vec<u8>,
    u: Vec<u8>,
    oe: Vec<u8>,
    ue: Vec<u8>,
    perms: Vec<u8>,
    encrypt_metadata: bool,
    file_id: Vec<u8>,
    password: Vec<u8>,
    expected_authenticated: bool,
    expected_used_owner_path: bool,
    expected_file_key: Vec<u8>,
}

fn parse_case(text: &str) -> Case {
    let mut c = Case {
        encrypt_metadata: true, // default per spec
        ..Default::default()
    };
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once(':') else { continue };
        let k = k.trim();
        let v = v.trim();
        match k {
            "v" => c.v = v.parse().unwrap(),
            "r" => c.r = v.parse().unwrap(),
            "length_bits" => c.length_bits = v.parse().unwrap(),
            "p" => c.p = v.parse().unwrap(),
            "o" => c.o = unhex(v),
            "u" => c.u = unhex(v),
            "oe" => c.oe = unhex(v),
            "ue" => c.ue = unhex(v),
            "perms" => c.perms = unhex(v),
            "encrypt_metadata" => c.encrypt_metadata = v == "true",
            "file_id" => c.file_id = unhex(v),
            "password_hex" => c.password = unhex(v),
            "expected_authenticated" => c.expected_authenticated = v == "true",
            "expected_used_owner_path" => c.expected_used_owner_path = v == "true",
            "expected_file_key" => c.expected_file_key = unhex(v),
            _ => {} // ignore unknown / case_kind
        }
    }
    c
}

#[test]
fn field_match_agreement_over_corpus() {
    let dir = corpus_dir();
    assert!(dir.is_dir(),
            "corpus dir not found: {} \
             (set SPECDEV_CORPUS_DIR or vendor tests/fixtures/encrypt_parser/cases/)",
            dir.display());

    let mut fixtures: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("read corpus dir")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("case"))
        .collect();
    fixtures.sort();
    assert!(!fixtures.is_empty(),
            "no .case fixtures under {}", dir.display());

    let mut checked = 0usize;
    let mut failures: Vec<String> = Vec::new();

    for path in &fixtures {
        eprintln!("TRY {}", path.display());
        let text = fs::read_to_string(path).expect("read case");
        let c = parse_case(&text);
        let enc = EncryptDict {
            v: c.v,
            r: c.r,
            length_bits: c.length_bits,
            p: c.p,
            o: c.o,
            u: c.u,
            oe: c.oe,
            ue: c.ue,
            perms: c.perms,
            encrypt_metadata: c.encrypt_metadata,
        };
        let res = authenticate(&enc, &c.password, &c.file_id)
            .unwrap_or_else(|e| panic!("authenticate failed for {}: {}",
                                       path.display(), e));
        let mut local: Vec<String> = Vec::new();
        if res.authenticated != c.expected_authenticated {
            local.push(format!(
                "authenticated: expected={} got={}",
                c.expected_authenticated, res.authenticated));
        }
        if res.used_owner_path != c.expected_used_owner_path {
            local.push(format!(
                "used_owner_path: expected={} got={}",
                c.expected_used_owner_path, res.used_owner_path));
        }
        if res.file_key != c.expected_file_key {
            local.push(format!(
                "file_key: expected={} got={}",
                hex_to_string(&c.expected_file_key),
                hex_to_string(&res.file_key)));
        }
        if !local.is_empty() {
            failures.push(format!("{}: {}", path.display(), local.join("; ")));
        }
        checked += 1;
    }
    if !failures.is_empty() {
        panic!("encrypt_parser field_match_agreement: {} failures of {} cases\n  {}",
               failures.len(), checked, failures.join("\n  "));
    }
    eprintln!("encrypt_parser field_match_agreement: checked={} failed=0", checked);
}
