// Public-API smoke driver — PageSize.
//
// Coverage:
//   * Ctor stores width / height; setters round-trip.
//   * is_landscape reflects width > height.
//   * set_is_landscape swaps when value differs from current
//     orientation, no-op otherwise.
//   * Twelve named-page-size constants match the canonical
//     Aspose.PDF 26.4.0 dimensions (apidev probe oracle — same
//     numbers as the C# / TypeScript peer tracks).
//   * `const fn` named-size factories — usable in const context.

use aspose_pdf_foss::PageSize;

#[test]
fn ctor_sets_width_and_height() {
    let s = PageSize::new(100.0, 200.0);
    assert_eq!(s.width(), 100.0);
    assert_eq!(s.height(), 200.0);
}

#[test]
fn setters_round_trip() {
    let mut s = PageSize::new(100.0, 200.0);
    s.set_width(150.0);
    s.set_height(250.0);
    assert_eq!(s.width(), 150.0);
    assert_eq!(s.height(), 250.0);
}

#[test]
fn is_landscape_when_width_exceeds_height() {
    assert!(!PageSize::new(100.0, 200.0).is_landscape());
    assert!(PageSize::new(300.0, 200.0).is_landscape());
    // Equal width and height — canonical behavior: NOT landscape
    // (strict `>` per the C# body).
    assert!(!PageSize::new(200.0, 200.0).is_landscape());
}

#[test]
fn set_is_landscape_swaps_when_changing_orientation() {
    let mut p = PageSize::new(100.0, 200.0);
    assert!(!p.is_landscape());
    p.set_is_landscape(true);
    assert_eq!(p.width(), 200.0);
    assert_eq!(p.height(), 100.0);
    assert!(p.is_landscape());
}

#[test]
fn set_is_landscape_noop_when_already_in_requested_orientation() {
    let mut p = PageSize::new(100.0, 200.0);
    p.set_is_landscape(false);
    assert_eq!(p.width(), 100.0);
    assert_eq!(p.height(), 200.0);
}

#[test]
fn a4_factory_yields_portrait_positive_dimensions() {
    let a4 = PageSize::a4();
    assert!(a4.width() > 0.0);
    assert!(a4.height() > 0.0);
    assert!(a4.height() > a4.width());
}

#[test]
fn page_letter_matches_us_dimensions() {
    let letter = PageSize::page_letter();
    assert_eq!(letter.width(), 612.0);
    assert_eq!(letter.height(), 792.0);
}

#[test]
fn page_ledger_is_landscape() {
    assert!(PageSize::page_ledger().is_landscape());
}

// Canonical Aspose.PDF 26.4.0 named-page-size dimensions, oracle-
// frozen via apidev --probe-static-values (LibForge/ApiDev). Any
// drift indicates a value mismatch against the canonical assembly.
#[test]
fn named_size_dimensions_match_canonical_oracle() {
    let oracle: &[(PageSize, f32, f32)] = &[
        (PageSize::a0(),          2384.0, 3370.0),
        (PageSize::a1(),          1684.0, 2384.0),
        (PageSize::a2(),          1190.0, 1684.0),
        (PageSize::a3(),           842.0, 1190.0),
        (PageSize::a4(),           595.0,  842.0),
        (PageSize::a5(),           421.0,  595.0),
        (PageSize::a6(),           297.0,  421.0),
        (PageSize::b5(),           501.0,  709.0),
        (PageSize::page_letter(),  612.0,  792.0),
        (PageSize::page_legal(),   612.0, 1008.0),
        (PageSize::page_ledger(), 1224.0,  792.0),
        (PageSize::p11x17(),       792.0, 1224.0),
    ];
    for (ps, w, h) in oracle {
        assert_eq!(ps.width(), *w, "width mismatch for {ps:?}");
        assert_eq!(ps.height(), *h, "height mismatch for {ps:?}");
    }
}

#[test]
fn named_size_factories_usable_in_const_context() {
    const A4: PageSize = PageSize::a4();
    const LETTER: PageSize = PageSize::page_letter();
    assert_eq!(A4.width(), 595.0);
    assert_eq!(LETTER.width(), 612.0);
}

#[test]
fn copy_semantics_independent_after_mutation() {
    let p1 = PageSize::new(100.0, 200.0);
    let mut p2 = p1;
    p2.set_width(999.0);
    assert_eq!(p1.width(), 100.0);
    assert_eq!(p2.width(), 999.0);
}
