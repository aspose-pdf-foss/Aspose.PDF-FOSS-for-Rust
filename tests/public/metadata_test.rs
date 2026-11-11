// Public-API smoke driver — Metadata accessor over a Document's
// XMP /Metadata packet.
//
// Exercises:
//   * Document::metadata() lazy-equivalent eager parse from
//     with_xmp.pdf — keys + values match the fixture.
//   * Empty Metadata for a doc without /Metadata in catalog.
//   * add / insert / remove / contains_key / get / len / iter
//     semantics on a fresh Metadata.
//   * register_namespace_uri round-trip + empty-string fallback.
//   * Duplicate-add error path.
//   * metadata_mut() mutations do NOT mark the document dirty.

use std::path::PathBuf;

use aspose_pdf_foss::{Document, Error, Metadata, XmpValue};

fn fixture(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(rel)
}

#[test]
fn reads_xmp_from_fixture() {
    let doc = Document::open(fixture("metadata/with_xmp.pdf")).unwrap();
    let m = doc.metadata();
    assert!(!m.is_empty(), "expected at least one XMP property");
    assert!(m.contains_key("dc:title"));
    assert_eq!(
        m.get("dc:title").unwrap().to_string_value(),
        "LibForge Test Document"
    );
    // pdf:Producer is an element-form simple-text property.
    assert_eq!(
        m.get("pdf:Producer").unwrap().to_string_value(),
        "LibForge Fixtures"
    );
}

#[test]
fn empty_metadata_for_fixture_without_xmp() {
    let doc = Document::open(fixture("metadata/without_xmp.pdf")).unwrap();
    assert_eq!(doc.metadata().len(), 0);
    assert!(doc.metadata().is_empty());
}

#[test]
fn add_indexer_and_count_track() {
    let mut doc = Document::open(fixture("metadata/with_xmp.pdf")).unwrap();
    let fresh = doc.metadata_mut();
    let initial = fresh.len();
    fresh.add("dc:format", XmpValue::from_string("application/pdf"))
        .unwrap();
    assert!(fresh.contains_key("dc:format"));
    assert_eq!(fresh.len(), initial + 1);
    assert_eq!(
        fresh.get("dc:format").unwrap().to_string_value(),
        "application/pdf"
    );
}

#[test]
fn duplicate_add_errors() {
    let mut m = Metadata::new();
    m.add("dc:format", XmpValue::from_string("application/pdf"))
        .unwrap();
    match m.add("dc:format", XmpValue::from_string("text/plain")) {
        Err(Error::DuplicateMetadataKey(k)) => assert_eq!(k, "dc:format"),
        other => panic!("expected DuplicateMetadataKey, got {other:?}"),
    }
}

#[test]
fn insert_replaces_silently() {
    let mut m = Metadata::new();
    let prev = m.insert("dc:format", XmpValue::from_string("v1"));
    assert!(prev.is_none());
    let prev = m.insert("dc:format", XmpValue::from_string("v2"));
    assert_eq!(prev.unwrap().to_string_value(), "v1");
    assert_eq!(m.get("dc:format").unwrap().to_string_value(), "v2");
}

#[test]
fn remove_drops_key_and_is_idempotent() {
    let mut m = Metadata::new();
    m.add("dc:format", XmpValue::from_string("application/pdf"))
        .unwrap();
    assert!(m.remove("dc:format"));
    assert!(!m.contains_key("dc:format"));
    assert!(!m.remove("dc:format"));
}

#[test]
fn namespace_prefix_uri_round_trip() {
    let mut m = Metadata::new();
    m.register_namespace_uri("dc", "http://purl.org/dc/elements/1.1/");
    assert_eq!(
        m.namespace_uri_by_prefix("dc"),
        "http://purl.org/dc/elements/1.1/"
    );
    assert_eq!(
        m.prefix_by_namespace_uri("http://purl.org/dc/elements/1.1/"),
        "dc"
    );
    assert_eq!(m.namespace_uri_by_prefix("missing"), "");
    assert_eq!(m.prefix_by_namespace_uri("missing"), "");
}

#[test]
fn keys_values_iter_and_shape() {
    let mut m = Metadata::new();
    m.add("xmp:CreateDate", XmpValue::from_string("2026-05-04"))
        .unwrap();
    m.add("pdf:Producer", XmpValue::from_string("FOSS")).unwrap();
    assert_eq!(m.keys().count(), 2);
    assert_eq!(m.values().count(), 2);
    let collected: Vec<_> = m.iter().map(|(k, _)| k).collect();
    // BTreeMap iteration is sorted — pdf:* < xmp:*.
    assert_eq!(collected, vec!["pdf:Producer", "xmp:CreateDate"]);
    assert!(!m.is_fixed_size());
    assert!(!m.is_read_only());
}

#[test]
fn clear_removes_entries_and_bindings() {
    let mut m = Metadata::new();
    m.add("dc:title", XmpValue::from_string("T")).unwrap();
    m.register_namespace_uri("dc", "http://purl.org/dc/elements/1.1/");
    m.clear();
    assert_eq!(m.len(), 0);
    assert_eq!(m.namespace_uri_by_prefix("dc"), "");
}

// v1.1: metadata_mut() mutations DO mark Metadata dirty;
// save emits a fresh /Metadata stream object via
// pdf_writer_incremental.

#[test]
fn metadata_mut_save_emits_metadata_stream() {
    let mut doc = Document::open(fixture("metadata/with_xmp.pdf")).unwrap();
    doc.metadata_mut()
        .insert("dc:format", XmpValue::from_string("application/pdf"));

    let mut buf = Vec::new();
    doc.save_to_writer(&mut buf).unwrap();
    assert!(find_bytes(&buf, b"<dc:format>application/pdf</dc:format>"),
        "save output should carry the new XMP element");
    assert!(find_bytes(&buf, b"xmlns:dc=\"http://purl.org/dc/elements/1.1/\""),
        "save output should declare the dc xmlns");
}

#[test]
fn save_then_reload_round_trips_metadata() {
    let mut doc = Document::open(fixture("metadata/with_xmp.pdf")).unwrap();
    doc.metadata_mut()
        .insert("dc:title", XmpValue::from_string("Persisted Title"));
    doc.metadata_mut()
        .insert("pdf:Producer", XmpValue::from_string("Aspose.PDF FOSS"));

    let tmp = common::unique_temp(format!("aspose_meta_rt_{}.pdf",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()));
    doc.save_to(&tmp).unwrap();

    let reloaded = Document::open(&tmp).unwrap();
    assert_eq!(
        reloaded.metadata().get("dc:title").unwrap().to_string_value(),
        "Persisted Title");
    assert_eq!(
        reloaded.metadata().get("pdf:Producer").unwrap().to_string_value(),
        "Aspose.PDF FOSS");
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn save_clean_when_nothing_dirty() {
    let doc = Document::open(fixture("metadata/with_xmp.pdf")).unwrap();
    let original = std::fs::read(fixture("metadata/with_xmp.pdf")).unwrap();
    let mut buf = Vec::new();
    doc.save_to_writer(&mut buf).unwrap();
    assert_eq!(buf, original,
        "no edits → save must round-trip source bytes verbatim");
}

#[test]
fn custom_namespace_uri_round_trips_through_save_and_reload() {
    // v1.1: source-population captures `xmlns:prefix → uri`
    // declarations into the Metadata registry. A custom
    // prefix saved + reloaded preserves its URI binding
    // (closes the xmp::parse URI capture parity gap).
    let mut doc = Document::open(fixture("metadata/with_xmp.pdf")).unwrap();
    doc.metadata_mut()
        .register_namespace_uri("custom", "http://example.com/v1/");
    doc.metadata_mut()
        .insert("custom:tag", XmpValue::from_string("value"));

    let tmp = common::unique_temp(format!("aspose_meta_ns_{}.pdf",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()));
    doc.save_to(&tmp).unwrap();

    let reloaded = Document::open(&tmp).unwrap();
    assert_eq!(
        reloaded.metadata().namespace_uri_by_prefix("custom"),
        "http://example.com/v1/");
    assert_eq!(
        reloaded.metadata().get("custom:tag").unwrap().to_string_value(),
        "value");
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn xml_special_chars_escape_in_packet() {
    let mut doc = Document::open(fixture("metadata/with_xmp.pdf")).unwrap();
    doc.metadata_mut()
        .insert("dc:title", XmpValue::from_string("A & B <test>"));
    let mut buf = Vec::new();
    doc.save_to_writer(&mut buf).unwrap();
    assert!(find_bytes(&buf,
        b"<dc:title>A &amp; B &lt;test&gt;</dc:title>"));
}

fn find_bytes(hay: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() { return true; }
    if hay.len() < needle.len() { return false; }
    for i in 0..=(hay.len() - needle.len()) {
        if &hay[i..i + needle.len()] == needle { return true; }
    }
    false
}

#[path = "../common/mod.rs"]
mod common;
