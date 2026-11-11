// Hand-authored gate driver — numeric_oracle_agreement gate.
// Primitive: transform
// Freeze gate: numeric_oracle_agreement over
//   `Feedstock/PDF/Fixtures/transform/*.case`. Each .case file
//   describes one operation (op + named operands + expected_*),
//   the driver dispatches to the right entry point and compares
//   each output component against the expected with eps_abs.
//
// Tolerance: 1e-12 per component — comfortably above ulp drift
// for compose / apply / inverse, tight enough to catch a sign
// error or transposed off-diagonal.

#[path = "../../src/internal/transform.rs"]
mod transform;

use std::fs;
use std::path::{Path, PathBuf};

use transform::{Matrix, Point, Rect};

const EPS_ABS: f64 = 1e-12;

fn corpus_dir() -> PathBuf {
    if let Ok(env_dir) = std::env::var("SPECDEV_CORPUS_DIR") {
        if !env_dir.is_empty() {
            return PathBuf::from(env_dir);
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("transform")
}

#[derive(Debug)]
enum Expected {
    Matrix(Matrix),
    Point(Point),
    Rect(Rect),
    Singular,
    Throws,
}

#[derive(Debug, Default)]
struct Case {
    op: Option<String>,
    fields: Vec<(String, Vec<f64>)>,
    expected: Option<Expected>,
}

fn parse_case(path: &Path) -> Case {
    let text = fs::read_to_string(path).expect("read case");
    let mut case = Case::default();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let mut parts = trimmed.split_whitespace();
        let key = parts.next().expect("case key");
        let rest: Vec<&str> = parts.collect();
        match key {
            "op" => {
                case.op = Some(rest[0].to_string());
            }
            "expected_matrix" => {
                let v = parse_floats(&rest);
                assert_eq!(v.len(), 6, "expected_matrix needs 6 floats");
                case.expected = Some(Expected::Matrix(Matrix::new(
                    v[0], v[1], v[2], v[3], v[4], v[5])));
            }
            "expected_point" => {
                let v = parse_floats(&rest);
                assert_eq!(v.len(), 2, "expected_point needs 2 floats");
                case.expected = Some(Expected::Point(
                    Point { x: v[0], y: v[1] }));
            }
            "expected_rect" => {
                let v = parse_floats(&rest);
                assert_eq!(v.len(), 4, "expected_rect needs 4 floats");
                case.expected = Some(Expected::Rect(Rect {
                    x_min: v[0], y_min: v[1],
                    x_max: v[2], y_max: v[3],
                }));
            }
            "expected_singular" => {
                case.expected = Some(Expected::Singular);
            }
            "expected_throws" => {
                // The token after `expected_throws` is the cpp
                // canonical type name (`std::invalid_argument`);
                // we accept any value here since the rust error
                // type is local to this crate.
                case.expected = Some(Expected::Throws);
            }
            _ => {
                case.fields.push((key.to_string(), parse_floats(&rest)));
            }
        }
    }
    case
}

fn parse_floats(tokens: &[&str]) -> Vec<f64> {
    tokens.iter()
        .map(|t| t.parse::<f64>()
                  .unwrap_or_else(|e| panic!("bad float {t}: {e}")))
        .collect()
}

fn field<'a>(case: &'a Case, name: &str) -> &'a [f64] {
    &case.fields.iter()
        .find(|(k, _)| k == name)
        .unwrap_or_else(|| panic!("case missing field {name}"))
        .1
}

fn approx_close(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() < EPS_ABS
}

fn matrix_eq(a: Matrix, b: Matrix) -> bool {
    approx_close(a.a, b.a) && approx_close(a.b, b.b)
        && approx_close(a.c, b.c) && approx_close(a.d, b.d)
        && approx_close(a.e, b.e) && approx_close(a.f, b.f)
}

fn point_eq(a: Point, b: Point) -> bool {
    approx_close(a.x, b.x) && approx_close(a.y, b.y)
}

fn rect_eq(a: Rect, b: Rect) -> bool {
    approx_close(a.x_min, b.x_min) && approx_close(a.y_min, b.y_min)
        && approx_close(a.x_max, b.x_max)
        && approx_close(a.y_max, b.y_max)
}

fn matrix_from(slice: &[f64]) -> Matrix {
    assert_eq!(slice.len(), 6);
    Matrix::new(slice[0], slice[1], slice[2],
                slice[3], slice[4], slice[5])
}

fn run_case(case: &Case, path: &Path) {
    let op = case.op.as_deref().expect("missing op");
    let expected = case.expected.as_ref().expect("missing expected");
    match op {
        "identity" => {
            let m = transform::identity();
            check_matrix(m, expected, path);
        }
        "translate" => {
            let tx = field(case, "tx")[0];
            let ty = field(case, "ty")[0];
            check_matrix(transform::translate(tx, ty), expected, path);
        }
        "scale" => {
            let sx = field(case, "sx")[0];
            let sy = field(case, "sy")[0];
            check_matrix(transform::scale(sx, sy), expected, path);
        }
        "rotate" => {
            let rad = field(case, "radians")[0];
            check_matrix(transform::rotate(rad), expected, path);
        }
        "skew" => {
            let alpha = field(case, "alpha")[0];
            let beta = field(case, "beta")[0];
            check_matrix(transform::skew(alpha, beta), expected, path);
        }
        "compose" => {
            let lhs = matrix_from(field(case, "lhs"));
            let rhs = matrix_from(field(case, "rhs"));
            check_matrix(transform::compose(lhs, rhs), expected, path);
        }
        "inverse" => {
            let m = matrix_from(field(case, "m"));
            let result = transform::inverse(m);
            match (expected, result) {
                (Expected::Singular, None) => {}
                (Expected::Singular, Some(_)) => {
                    panic!("expected singular for {} but got matrix",
                           path.display());
                }
                (Expected::Matrix(want), Some(got)) => {
                    if !matrix_eq(got, *want) {
                        panic!("inverse mismatch for {}: got {:?} want {:?}",
                               path.display(), got, want);
                    }
                }
                _ => panic!("inverse expectation mismatch for {}",
                            path.display()),
            }
        }
        "apply_point" => {
            let m = matrix_from(field(case, "m"));
            let p_raw = field(case, "p");
            let p = Point { x: p_raw[0], y: p_raw[1] };
            let got = transform::apply(m, p);
            match expected {
                Expected::Point(want) => {
                    if !point_eq(got, *want) {
                        panic!("apply_point mismatch for {}: got {:?} want {:?}",
                               path.display(), got, want);
                    }
                }
                _ => panic!("apply_point needs expected_point in {}",
                            path.display()),
            }
        }
        "apply_rect" => {
            let m = matrix_from(field(case, "m"));
            let r_raw = field(case, "r");
            let r = Rect {
                x_min: r_raw[0], y_min: r_raw[1],
                x_max: r_raw[2], y_max: r_raw[3],
            };
            let got = transform::apply_to_rect(m, r);
            match expected {
                Expected::Rect(want) => {
                    if !rect_eq(got, *want) {
                        panic!("apply_rect mismatch for {}: got {:?} want {:?}",
                               path.display(), got, want);
                    }
                }
                _ => panic!("apply_rect needs expected_rect in {}",
                            path.display()),
            }
        }
        "viewport" => {
            let mb_raw = field(case, "media_box");
            let mb = Rect {
                x_min: mb_raw[0], y_min: mb_raw[1],
                x_max: mb_raw[2], y_max: mb_raw[3],
            };
            let rotate = field(case, "rotate")[0] as i32;
            let pw = field(case, "pixel_width")[0] as i64;
            let ph = field(case, "pixel_height")[0] as i64;
            // Negative pixel dims fall through the u32 cast as
            // huge values; the case files only use 0 and positive
            // ints so a saturating cast is fine, but reject
            // negatives explicitly to keep the gate honest.
            let pw_u = if pw < 0 { 0 } else { pw as u32 };
            let ph_u = if ph < 0 { 0 } else { ph as u32 };
            let result = transform::viewport(mb, rotate, pw_u, ph_u);
            match (expected, result) {
                (Expected::Throws, Err(_)) => {}
                (Expected::Throws, Ok(_)) => {
                    panic!("expected throws for {} but got matrix",
                           path.display());
                }
                (Expected::Matrix(want), Ok(got)) => {
                    if !matrix_eq(got, *want) {
                        panic!("viewport mismatch for {}: got {:?} want {:?}",
                               path.display(), got, want);
                    }
                }
                _ => panic!("viewport expectation mismatch for {}",
                            path.display()),
            }
        }
        other => panic!("unknown op {other} in {}", path.display()),
    }
}

fn check_matrix(got: Matrix, expected: &Expected, path: &Path) {
    match expected {
        Expected::Matrix(want) => {
            if !matrix_eq(got, *want) {
                panic!("matrix mismatch for {}: got {:?} want {:?}",
                       path.display(), got, want);
            }
        }
        _ => panic!("expected matrix for {}", path.display()),
    }
}

#[test]
fn numeric_oracle_agreement_over_corpus() {
    let dir = corpus_dir();
    assert!(dir.is_dir(),
            "corpus dir not found: {} \
             (set SPECDEV_CORPUS_DIR or vendor tests/fixtures/transform/)",
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
    for path in &cases {
        let case = parse_case(path);
        run_case(&case, path);
        checked += 1;
    }
    eprintln!("transform numeric_oracle_agreement: \
               checked={} failed=0 eps_abs={EPS_ABS}",
              checked);
}
