// Hand-authored gate driver — field_match_agreement gate.
// Primitive: encrypt_writer
// Freeze gate: for each `.case` text fixture under
// `tests/fixtures/encrypt_writer/cases/`, parse the declared
// inputs (passwords + p + v + r + length_bits + file_id +
// encrypt_metadata + salt) and expected outputs (/O, /U, /OE,
// /UE, /Perms, file_key), run build_encrypt_dict(), and compare
// each declared output field against the expected value. Pikepdf
// is the authoring oracle — its expected values are baked into
// the .case sidecars.

#[path = "../../src/internal/aes_cbc.rs"]
mod aes_cbc;

#[path = "../../src/internal/rc4.rs"]
mod rc4;

#[path = "../../src/internal/digest.rs"]
mod digest;

#[path = "../../src/internal/encrypt_parser.rs"]
mod encrypt_parser;

#[path = "../../src/internal/encrypt_writer.rs"]
mod encrypt_writer;

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
        .join("encrypt_writer")
        .join("cases")
}

fn unhex(s: &str) -> Vec<u8> {
    let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(s.len() / 2);
    let mut i = 0;
    while i + 1 < bytes.len() {
        let d = |b: u8| match b {
            b'0'..=b'9' => b - b'0',
            b'a'..=b'f' => b - b'a' + 10,
            b'A'..=b'F' => b - b'A' + 10,
            _ => panic!("bad hex byte"),
        };
        out.push((d(bytes[i]) << 4) | d(bytes[i + 1]));
        i += 2;
    }
    out
}

#[derive(Default)]
struct Case {
    user_password: Vec<u8>,
    owner_password: Vec<u8>,
    v: u32,
    r: u32,
    length_bits: u32,
    p: i32,
    file_id: Vec<u8>,
    encrypt_metadata: bool,
    salt: Vec<u8>,
    expected_o: Vec<u8>,
    expected_u: Vec<u8>,
    expected_oe: Vec<u8>,
    expected_ue: Vec<u8>,
    expected_perms: Vec<u8>,
    expected_file_key: Vec<u8>,
}

fn parse_case(text: &str) -> Case {
    let mut c = Case { encrypt_metadata: true, ..Case::default() };
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once(':') else { continue };
        let (k, v) = (k.trim(), v.trim());
        match k {
            "user_password_hex" => c.user_password = unhex(v),
            "owner_password_hex" => c.owner_password = unhex(v),
            "v" => c.v = v.parse().unwrap(),
            "r" => c.r = v.parse().unwrap(),
            "length_bits" => c.length_bits = v.parse().unwrap(),
            "p" => c.p = v.parse().unwrap(),
            "file_id" => c.file_id = unhex(v),
            "encrypt_metadata" => c.encrypt_metadata = v == "true",
            "salt" => c.salt = unhex(v),
            "expected_o" => c.expected_o = unhex(v),
            "expected_u" => c.expected_u = unhex(v),
            "expected_oe" => c.expected_oe = unhex(v),
            "expected_ue" => c.expected_ue = unhex(v),
            "expected_perms" => c.expected_perms = unhex(v),
            "expected_file_key" => c.expected_file_key = unhex(v),
            _ => {}
        }
    }
    c
}

#[test]
fn encrypt_writer_field_match_agreement() {
    let dir = corpus_dir();
    let mut cases: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|_| panic!("no corpus dir at {}", dir.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|e| e == "case").unwrap_or(false))
        .collect();
    cases.sort();
    assert!(!cases.is_empty(), "no .case fixtures under {}", dir.display());

    for path in &cases {
        let name = path.file_name().unwrap().to_string_lossy();
        let c = parse_case(&fs::read_to_string(path).unwrap());
        let out = encrypt_writer::build_encrypt_dict(
            &c.user_password,
            &c.owner_password,
            c.p,
            c.v,
            c.r,
            c.length_bits,
            &c.file_id,
            c.encrypt_metadata,
            &c.salt,
        )
        .unwrap_or_else(|e| panic!("{name}: build_encrypt_dict failed: {e}"));

        let d = &out.encrypt_dict;
        assert_eq!(d.o, c.expected_o, "{name}: /O mismatch");
        assert_eq!(d.u, c.expected_u, "{name}: /U mismatch");
        assert_eq!(d.oe, c.expected_oe, "{name}: /OE mismatch");
        assert_eq!(d.ue, c.expected_ue, "{name}: /UE mismatch");
        assert_eq!(d.perms, c.expected_perms, "{name}: /Perms mismatch");
        assert_eq!(out.file_key, c.expected_file_key, "{name}: file_key mismatch");
    }
}
