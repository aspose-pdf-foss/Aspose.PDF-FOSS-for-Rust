// Public-API smoke driver — border styling value types.
//
// Ports border_info_smoke_test.cpp: the styling value types
// GraphInfo, BorderInfo and the BorderSide [Flags] enum.
//
// Coverage:
//   * GraphInfo defaults (line width, scaling, doubled, dash array).
//   * BorderSide flag combination (All / Box == 15, set membership).
//   * BorderInfo default has zero-width edges + zero corner radius.
//   * BorderInfo constructors configure only the named edges, with
//     explicit width / colour / GraphInfo, plus the corner radius.

use aspose_pdf_foss::{BorderInfo, BorderSide, Color, GraphInfo};

// ============================================================
// GraphInfo
// ============================================================

#[test]
fn graph_info_properties() {
    let g = GraphInfo::new();
    assert_eq!(g.line_width(), 1.0);
    assert_eq!(g.scaling_rate_x(), 1.0);
    assert!(!g.is_doubled());
    assert!(g.dash_array().is_empty());
}

// ============================================================
// BorderSide
// ============================================================

#[test]
fn border_side_flags_combine() {
    assert_eq!(BorderSide::ALL.bits(), 15);
    assert_eq!(BorderSide::BOX.bits(), 15);
    let lr = BorderSide::LEFT | BorderSide::RIGHT;
    assert!(lr.contains(BorderSide::LEFT));
    assert!(lr.contains(BorderSide::RIGHT));
    assert!(!lr.contains(BorderSide::TOP));
}

// ============================================================
// BorderInfo
// ============================================================

#[test]
fn border_info_default_has_no_edges() {
    let b = BorderInfo::new();
    assert_eq!(b.left().line_width(), 0.0);
    assert_eq!(b.top().line_width(), 0.0);
    assert_eq!(b.rounded_border_radius(), 0.0);
}

#[test]
fn border_info_constructors_configure_named_edges() {
    // All edges, explicit width + colour.
    let all = BorderInfo::with_side_width_color(
        BorderSide::ALL,
        2.0,
        Color::from_rgb(0.0, 0.0, 1.0),
    );
    assert_eq!(all.left().line_width(), 2.0);
    assert_eq!(all.bottom().line_width(), 2.0);
    assert_eq!(all.right().color().a(), 1.0);

    // Only the named sides are configured; the rest stay at width 0.
    let lr = BorderInfo::with_side_width(BorderSide::LEFT | BorderSide::RIGHT, 3.0);
    assert_eq!(lr.left().line_width(), 3.0);
    assert_eq!(lr.right().line_width(), 3.0);
    assert_eq!(lr.top().line_width(), 0.0);
    assert_eq!(lr.bottom().line_width(), 0.0);

    // GraphInfo-valued constructor.
    let mut g = GraphInfo::new();
    g.set_line_width(5.0);
    let mut top = BorderInfo::with_side_graph_info(BorderSide::TOP, g);
    assert_eq!(top.top().line_width(), 5.0);
    assert_eq!(top.left().line_width(), 0.0);

    top.set_rounded_border_radius(4.0);
    assert_eq!(top.rounded_border_radius(), 4.0);
}
