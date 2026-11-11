// Public-API smoke driver — explicit-destination subtypes.
//
// Ports the cpp explicit_destination_subtypes_smoke_test (subtype
// parts): the ExplicitDestinationType ordinals, the coordinate
// accessors, the polymorphic page_number() / to_string()
// destination-array rendering, and the suffix-named constructors.
// The GoToAction / LinkAnnotation cases are deferred to the
// actions / annotations clusters.

use aspose_pdf_foss::{
    ExplicitDestination, ExplicitDestinationType, FitBExplicitDestination,
    FitBHExplicitDestination, FitBVExplicitDestination, FitExplicitDestination,
    FitHExplicitDestination, FitRExplicitDestination, FitVExplicitDestination,
    XYZExplicitDestination,
};

#[test]
fn destination_type_ordinals() {
    assert_eq!(ExplicitDestinationType::XYZ as i32, 0);
    assert_eq!(ExplicitDestinationType::Fit as i32, 1);
    assert_eq!(ExplicitDestinationType::FitH as i32, 2);
    assert_eq!(ExplicitDestinationType::FitV as i32, 3);
    assert_eq!(ExplicitDestinationType::FitR as i32, 4);
    assert_eq!(ExplicitDestinationType::FitB as i32, 5);
    assert_eq!(ExplicitDestinationType::FitBH as i32, 6);
    assert_eq!(ExplicitDestinationType::FitBV as i32, 7);
}

#[test]
fn xyz_accessors_and_to_string() {
    let d = XYZExplicitDestination::new(3, 100.0, 700.0, 2.0);
    assert_eq!(d.page_number(), 3);
    assert_eq!(d.left(), 100.0);
    assert_eq!(d.top(), 700.0);
    assert_eq!(d.zoom(), 2.0);
    assert_eq!(d.to_string(), "3 XYZ 100 700 2");
}

#[test]
fn fit_family_to_string() {
    assert_eq!(FitExplicitDestination::new(2).to_string(), "2 Fit");
    assert_eq!(FitHExplicitDestination::new(2, 500.0).to_string(), "2 FitH 500");
    assert_eq!(FitVExplicitDestination::new(2, 50.0).to_string(), "2 FitV 50");
    assert_eq!(
        FitRExplicitDestination::new(2, 10.0, 20.0, 30.0, 40.0).to_string(),
        "2 FitR 10 20 30 40"
    );
    assert_eq!(FitBExplicitDestination::new(2).to_string(), "2 FitB");
    assert_eq!(FitBHExplicitDestination::new(2, 500.0).to_string(), "2 FitBH 500");
    assert_eq!(FitBVExplicitDestination::new(2, 50.0).to_string(), "2 FitBV 50");
}

#[test]
fn fit_subtype_accessors() {
    assert_eq!(FitHExplicitDestination::new(1, 612.0).top(), 612.0);
    assert_eq!(FitVExplicitDestination::new(1, 50.0).left(), 50.0);
    let r = FitRExplicitDestination::new(1, 10.0, 20.0, 30.0, 40.0);
    assert_eq!(r.left(), 10.0);
    assert_eq!(r.bottom(), 20.0);
    assert_eq!(r.right(), 30.0);
    assert_eq!(r.top(), 40.0);
    assert_eq!(FitBHExplicitDestination::new(1, 7.0).top(), 7.0);
    assert_eq!(FitBVExplicitDestination::new(1, 8.0).left(), 8.0);
}

#[test]
fn for_document_uses_page_number() {
    // for_document takes a document for API parity; the explicit
    // page number is authoritative. Build a throwaway document to
    // exercise the ctor.
    let doc = aspose_pdf_foss::Document::new();
    let d = XYZExplicitDestination::for_document(&doc, 5, 1.0, 2.0, 3.0);
    assert_eq!(d.page_number(), 5);
    assert_eq!(d.to_string(), "5 XYZ 1 2 3");
}

#[test]
fn boxed_dyn_is_polymorphic() {
    // A heterogeneous set held behind the ExplicitDestination trait
    // keeps each subtype's own to_string verb.
    let dests: Vec<Box<dyn ExplicitDestination>> = vec![
        Box::new(XYZExplicitDestination::new(1, 100.0, 700.0, 2.0)),
        Box::new(FitExplicitDestination::new(1)),
        Box::new(FitHExplicitDestination::new(1, 0.0)),
        Box::new(FitRExplicitDestination::new(1, 10.0, 20.0, 30.0, 40.0)),
    ];
    let rendered: Vec<String> = dests.iter().map(|d| d.to_string()).collect();
    assert_eq!(rendered[0], "1 XYZ 100 700 2");
    assert_eq!(rendered[1], "1 Fit");
    assert_eq!(rendered[2], "1 FitH 0");
    assert_eq!(rendered[3], "1 FitR 10 20 30 40");
    assert!(dests.iter().all(|d| d.page_number() == 1));
}
