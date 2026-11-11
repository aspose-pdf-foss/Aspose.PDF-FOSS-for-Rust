// Freeze-gate driver for foundation/binarize_bradley
// (binding_roundtrip). For each fixture under
// tests/fixtures/binarize_bradley/, parse the little-endian
// header (width / height / window_size / threshold_percent),
// run apply, and assert byte-equal to the .expected sidecar.
// Bradley's algorithm is fully integer-deterministic, so every
// target implementing the shared spec yaml produces identical
// bytes for identical inputs.

#[path = "../../src/internal/binarize_bradley.rs"]
mod binarize_bradley;

use std::fs;
use std::path::PathBuf;

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("binarize_bradley")
}

fn read_case(path: &std::path::Path) -> (u32, u32, u32, u32, Vec<u8>) {
    let data = fs::read(path).expect("read case");
    assert!(data.len() >= 16, "header truncated: {:?}", path);
    let width = u32::from_le_bytes(data[0..4].try_into().unwrap());
    let height = u32::from_le_bytes(data[4..8].try_into().unwrap());
    let window = u32::from_le_bytes(data[8..12].try_into().unwrap());
    let threshold = u32::from_le_bytes(data[12..16].try_into().unwrap());
    (width, height, window, threshold, data[16..].to_vec())
}

#[test]
fn binding_roundtrip_all_fixtures() {
    let dir = fixture_dir();
    let mut entries: Vec<_> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read_dir {:?}: {}", dir, e))
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().extension().and_then(|s| s.to_str())
                == Some("case")
        })
        .map(|e| e.path())
        .collect();
    entries.sort();
    assert!(!entries.is_empty(), "no .case fixtures under {:?}", dir);

    for case_path in entries {
        let (width, height, window, threshold, grayscale) =
            read_case(&case_path);
        let expected_path = case_path.with_extension("expected");
        let expected =
            fs::read(&expected_path).expect("read .expected");

        let got = binarize_bradley::apply(
            &grayscale, width, height, window, threshold,
        )
        .unwrap_or_else(|e| {
            panic!("apply failed for {:?}: {}", case_path, e)
        });

        assert_eq!(
            got, expected,
            "byte mismatch for {:?}\n  width={} height={} window={} threshold={}",
            case_path, width, height, window, threshold
        );
    }
}

#[test]
fn rejects_grayscale_length_mismatch() {
    let err = binarize_bradley::apply(&[0u8; 10], 8, 8, 4, 15).unwrap_err();
    assert!(matches!(
        err,
        binarize_bradley::Error::LengthMismatch { .. }
    ));
}

#[test]
fn rejects_zero_width() {
    let err = binarize_bradley::apply(&[], 0, 1, 2, 15).unwrap_err();
    assert!(matches!(err, binarize_bradley::Error::WidthOutOfRange(0)));
}

#[test]
fn rejects_zero_height() {
    let err = binarize_bradley::apply(&[], 1, 0, 2, 15).unwrap_err();
    assert!(matches!(err, binarize_bradley::Error::HeightOutOfRange(0)));
}

#[test]
fn rejects_window_size_below_minimum() {
    let err = binarize_bradley::apply(&[0u8; 64], 8, 8, 1, 15).unwrap_err();
    assert!(matches!(
        err,
        binarize_bradley::Error::WindowSizeOutOfRange(1)
    ));
}

#[test]
fn rejects_window_size_above_max_dimension() {
    let err = binarize_bradley::apply(&[0u8; 64], 8, 8, 9, 15).unwrap_err();
    assert!(matches!(
        err,
        binarize_bradley::Error::WindowSizeOutOfRange(9)
    ));
}

#[test]
fn rejects_threshold_percent_above_100() {
    let err = binarize_bradley::apply(&[0u8; 64], 8, 8, 4, 101).unwrap_err();
    assert!(matches!(
        err,
        binarize_bradley::Error::ThresholdPercentOutOfRange(101)
    ));
}
