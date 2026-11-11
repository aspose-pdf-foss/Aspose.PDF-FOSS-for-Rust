// Public-API smoke driver — named destinations.
//
// Ports the cpp named_destinations_smoke_test (collection mechanics) +
// named_destinations_persistence_smoke_test (Document.named_destinations()
// save-through round-trip).

use aspose_pdf_foss::{
    Document, FitExplicitDestination, FitHExplicitDestination, NamedDestination,
    NamedDestinationCollection, XYZExplicitDestination,
};
use std::path::{Path, PathBuf};

fn two_pages() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/text_extractor/two_pages.pdf")
}

#[test]
fn document_save_reopen_round_trip() {
    let out = common::unique_temp("aspose_rust_named_dests.pdf");
    {
        let mut doc = Document::open(two_pages()).unwrap();
        assert_eq!(doc.pages().count(), 2);
        // Empty on a doc with no /Names.
        assert_eq!(doc.named_destinations().count(), 0);
        let dests = doc.named_destinations_mut();
        dests.add("Intro".into(), Box::new(XYZExplicitDestination::new(1, 100.0, 700.0, 2.0)));
        dests.add("Middle".into(), Box::new(FitHExplicitDestination::new(1, 540.0)));
        dests.add("Outro".into(), Box::new(FitExplicitDestination::new(2)));
        assert_eq!(dests.count(), 3);
        doc.save_to(&out).unwrap();
    }
    let doc = Document::open(&out).unwrap();
    let dests = doc.named_destinations();
    assert_eq!(dests.count(), 3);
    assert_eq!(dests.names(), vec!["Intro".to_string(), "Middle".into(), "Outro".into()]);
    assert_eq!(dests.get("Intro").unwrap().to_string(), "1 XYZ 100 700 2");
    assert_eq!(dests.get("Middle").unwrap().to_string(), "1 FitH 540");
    assert_eq!(dests.get("Outro").unwrap().to_string(), "2 Fit");
    assert!(dests.get("absent").is_none());
    let _ = std::fs::remove_file(&out);
}

#[test]
fn document_destinations_reads_explicit_destinations() {
    let out = common::unique_temp("aspose_rust_dest_collection.pdf");
    {
        let mut doc = Document::open(two_pages()).unwrap();
        // Empty on a doc with no /Names /Dests.
        assert_eq!(doc.destinations().count(), 0);
        let dests = doc.named_destinations_mut();
        dests.add("Intro".into(), Box::new(XYZExplicitDestination::new(1, 100.0, 700.0, 2.0)));
        dests.add("Outro".into(), Box::new(FitExplicitDestination::new(2)));
        doc.save_to(&out).unwrap();
    }
    let doc = Document::open(&out).unwrap();
    let d = doc.destinations();
    assert_eq!(d.count(), 2);
    assert_eq!(d.get_page_number("Intro", false), 1);
    assert_eq!(d.get_page_number("Outro", false), 2);
    assert_eq!(d.get_page_number("absent", false), 0);
    assert_eq!(
        d.get_explicit_destination("Intro", false).unwrap().to_string(),
        "1 XYZ 100 700 2",
    );
    assert!(d.get_explicit_destination("absent", false).is_none());
    let _ = std::fs::remove_file(&out);
}

#[test]
fn named_destination_name_and_to_string() {
    let doc = Document::new();
    let nd = NamedDestination::from_document(&doc, "Chapter1".into());
    assert_eq!(nd.name(), "Chapter1");
    assert_eq!(nd.to_string(), "Chapter1");
}

#[test]
fn add_query_names_count() {
    let mut dests = NamedDestinationCollection::new();
    assert_eq!(dests.count(), 0);
    assert!(dests.get("missing").is_none());

    dests.add(
        "Intro".into(),
        Box::new(XYZExplicitDestination::new(1, 0.0, 800.0, 1.0)),
    );
    dests.add("Body".into(), Box::new(FitExplicitDestination::new(2)));
    assert_eq!(dests.count(), 2);

    let names = dests.names();
    assert_eq!(names, vec!["Intro".to_string(), "Body".to_string()]);

    assert_eq!(dests.get("Intro").unwrap().to_string(), "1 XYZ 0 800 1");
    assert_eq!(dests.get("Body").unwrap().to_string(), "2 Fit");
}

#[test]
fn replace_same_name_and_remove() {
    let mut dests = NamedDestinationCollection::new();
    dests.add(
        "Spot".into(),
        Box::new(XYZExplicitDestination::new(1, 0.0, 0.0, 1.0)),
    );
    dests.add(
        "Spot".into(),
        Box::new(XYZExplicitDestination::new(3, 10.0, 20.0, 2.0)),
    ); // replace
    assert_eq!(dests.count(), 1);
    assert_eq!(dests.get("Spot").unwrap().to_string(), "3 XYZ 10 20 2");

    dests.remove("Spot");
    assert_eq!(dests.count(), 0);
    assert!(dests.get("Spot").is_none());
    dests.remove("Spot"); // no-op on absent name
}

#[test]
fn stores_named_destination_appointment() {
    let doc = Document::new();
    let mut dests = NamedDestinationCollection::new();
    dests.add(
        "ByName".into(),
        Box::new(NamedDestination::from_document(&doc, "Chapter1".into())),
    );
    assert_eq!(dests.get("ByName").unwrap().to_string(), "Chapter1");
}

#[path = "../common/mod.rs"]
mod common;
