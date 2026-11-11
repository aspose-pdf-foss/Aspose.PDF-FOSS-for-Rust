// Hand-authored gate driver — componentwise_agreement gate.
// Primitive: colorspace
// Freeze gate: componentwise_agreement over
//   `Feedstock/PDF/Fixtures/colorspace/*.case`. Each .case file
//   describes one operation (input_kind + input doubles +
//   expected doubles or expected_throws), the driver dispatches
//   to the right entry point and compares each output component
//   against the expected with eps_abs.
//
// Tolerance: 1e-15 per component. Justification matches the
// anchor — every conversion is a fixed sum-of-products in
// IEEE-754 double over operands in [0, 1]; ulp drift is
// ~2.22e-16. 1e-15 is one order above ulp, comfortable for any
// legal rounding variant. A bug (sign flip, missing K factor,
// channel swap) produces drift of order 1, fifteen orders above
// eps_abs.
//
// Two-oracle agreement is established at fixture-AUTHORING time
// by `generate_fixtures.py` (NumPy + PDF 32000 §8.6.4 closed
// form); the driver only enforces componentwise equality of the
// implementation's output against the committed expected values.

#[path = "../../src/internal/colorspace.rs"]
mod colorspace;

use std::fs;
use std::path::{Path, PathBuf};

use colorspace::{quantize, to_srgb, ColorRgb, Family};

const EPS_ABS: f64 = 1e-15;

fn corpus_dir() -> PathBuf {
    if let Ok(env_dir) = std::env::var("SPECDEV_CORPUS_DIR") {
        if !env_dir.is_empty() {
            return PathBuf::from(env_dir);
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("colorspace")
}

#[derive(Debug, Default)]
struct Case {
    input_kind: Option<String>,
    input: Vec<f64>,
    expected: Option<Vec<f64>>,
    expected_throws: Option<String>,
}

fn parse_case(path: &Path) -> Case {
    let text = fs::read_to_string(path).expect("read case");
    let mut case = Case::default();
    for (lineno, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let mut tokens = trimmed.split_whitespace();
        let key = tokens.next().expect("case key");
        let rest: Vec<&str> = tokens.collect();
        match key {
            "input_kind" => {
                assert_eq!(rest.len(), 1,
                    "{}:{}: input_kind takes exactly one value",
                    path.display(), lineno + 1);
                case.input_kind = Some(rest[0].to_string());
            }
            "input" => {
                case.input = parse_doubles(&rest);
            }
            "expected" => {
                case.expected = Some(parse_doubles(&rest));
            }
            "expected_throws" => {
                assert_eq!(rest.len(), 1,
                    "{}:{}: expected_throws takes exactly one value",
                    path.display(), lineno + 1);
                case.expected_throws = Some(rest[0].to_string());
            }
            other => panic!("{}:{}: unknown directive '{}'",
                            path.display(), lineno + 1, other),
        }
    }
    assert!(case.input_kind.is_some(),
        "{}: missing required input_kind", path.display());
    assert!(case.expected.is_some() || case.expected_throws.is_some(),
        "{}: missing expected or expected_throws", path.display());
    case
}

fn parse_doubles(tokens: &[&str]) -> Vec<f64> {
    tokens.iter()
        .map(|t| {
            let lower = t.to_ascii_lowercase();
            match lower.as_str() {
                "nan" => f64::NAN,
                "inf" | "+inf" | "infinity" | "+infinity"
                    => f64::INFINITY,
                "-inf" | "-infinity" => f64::NEG_INFINITY,
                _ => t.parse::<f64>()
                      .unwrap_or_else(|e| panic!("bad double {t}: {e}")),
            }
        })
        .collect()
}

/// Per-kind dispatch — sibling of the TS / C++ / C# *_dispatch
/// helpers. Returns the primitive's output as a flat `Vec<f64>`
/// so the comparison is uniform across kinds.
fn convert(input_kind: &str, input: &[f64])
    -> Result<Vec<f64>, colorspace::Error>
{
    match input_kind {
        "DeviceGray" => {
            let ColorRgb { r, g, b } =
                to_srgb(Family::DeviceGray, input)?;
            Ok(vec![r, g, b])
        }
        "DeviceRGB" => {
            let ColorRgb { r, g, b } =
                to_srgb(Family::DeviceRGB, input)?;
            Ok(vec![r, g, b])
        }
        "DeviceCMYK" => {
            let ColorRgb { r, g, b } =
                to_srgb(Family::DeviceCMYK, input)?;
            Ok(vec![r, g, b])
        }
        "Quantize" => {
            assert_eq!(input.len(), 1,
                "Quantize: expected exactly 1 input double");
            Ok(vec![quantize(input[0]) as f64])
        }
        other => panic!("unknown input_kind '{other}'"),
    }
}

/// Cross-target alias: the .case fixtures use the C++ canonical
/// exception name. Any wrong-component-count error from the Rust
/// primitive matches the canonical token.
fn exception_matches(_err: &colorspace::Error, expected: &str) -> bool {
    matches!(expected,
        "std::invalid_argument"
        | "ArgumentException"
        | "RangeError"
        | "WrongComponentCount")
}

fn compare_components(path: &Path,
                      actual: &[f64],
                      expected: &[f64]) -> bool {
    if actual.len() != expected.len() {
        eprintln!(
            "MISMATCH {}: output size differs (actual={} expected={})",
            path.display(), actual.len(), expected.len());
        return false;
    }
    for (i, (a, e)) in actual.iter().zip(expected.iter()).enumerate() {
        let diff = (a - e).abs();
        if diff.is_nan() || diff > EPS_ABS {
            eprintln!(
                "MISMATCH {}: component[{}] actual={} expected={} \
                 diff={} eps_abs={}",
                path.display(), i, a, e, diff, EPS_ABS);
            return false;
        }
    }
    true
}

fn run_case(case: &Case, path: &Path) -> bool {
    let kind = case.input_kind.as_deref().unwrap();

    if let Some(expected_throws) = &case.expected_throws {
        match convert(kind, &case.input) {
            Ok(_) => {
                eprintln!(
                    "MISMATCH {}: expected_throws {} — actual: \
                     (returned without erroring)",
                    path.display(), expected_throws);
                return false;
            }
            Err(e) => {
                if exception_matches(&e, expected_throws) {
                    return true;
                }
                eprintln!(
                    "MISMATCH {}: expected_throws {} — actual: {:?}",
                    path.display(), expected_throws, e);
                return false;
            }
        }
    }

    let expected = case.expected.as_ref().unwrap();
    match convert(kind, &case.input) {
        Ok(actual) => compare_components(path, &actual, expected),
        Err(e) => {
            eprintln!("MISMATCH {}: dispatch errored: {:?}",
                      path.display(), e);
            false
        }
    }
}

#[test]
fn componentwise_agreement_over_corpus() {
    let dir = corpus_dir();
    assert!(dir.is_dir(),
            "corpus dir not found: {} \
             (set SPECDEV_CORPUS_DIR or vendor tests/fixtures/colorspace/)",
            dir.display());

    let mut cases: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("read corpus dir")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .filter(|p| p.extension()
                     .and_then(|s| s.to_str()) == Some("case"))
        .collect();
    cases.sort();

    assert!(!cases.is_empty(),
            "no .case fixtures under {}", dir.display());

    let mut checked = 0_usize;
    let mut failed = 0_usize;
    for path in &cases {
        eprintln!("TRY {}", path.display());
        let case = parse_case(path);
        if !run_case(&case, path) {
            failed += 1;
        }
        checked += 1;
    }
    eprintln!("colorspace componentwise_agreement: \
               checked={} failed={} eps_abs={}",
              checked, failed, EPS_ABS);
    assert_eq!(failed, 0, "{} fixture mismatches", failed);
}
