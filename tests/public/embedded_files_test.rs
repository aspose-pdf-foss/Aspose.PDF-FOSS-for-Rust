// Public-API smoke driver — FileSpecification + EmbeddedFileCollection.
//
// FileSpecification: ctor variants, default include-contents, the
// case-insensitive value store, and the property round-trips.
// EmbeddedFileCollection: Add / AddWithKey keying, Count, Keys,
// FindByName (nullable) vs get / get_by_name (Result), and the
// deletion family. Mirrors the cpp embedded_files_smoke_test
// collection assertions. Plus a Document save / reopen round-trip
// exercising the catalog /Names /EmbeddedFiles persistence (the file
// bytes embed as an /EmbeddedFile stream; the metadata round-trips).

use aspose_pdf_foss::{
    AFRelationship, Document, EmbeddedFileCollection, EncryptedPayload, FileEncoding,
    FileSpecification,
};
use std::path::{Path, PathBuf};

fn two_pages() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/text_extractor/two_pages.pdf")
}

fn encrypted_payload_pdf() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/embedded_files/encrypted_payload.pdf")
}

#[test]
fn file_specification_defaults() {
    let fs = FileSpecification::new();
    assert!(fs.include_contents(), "include_contents defaults true");
    assert_eq!(fs.encoding(), FileEncoding::None);
    assert_eq!(fs.af_relationship(), AFRelationship::Unspecified);
    assert_eq!(fs.description(), "");
    // A bare specification carries no encrypted payload.
    assert!(fs.encrypted_payload().is_none());
}

#[test]
fn encrypted_payload_read_from_document() {
    let doc = Document::open(encrypted_payload_pdf()).unwrap();
    let files = doc.embedded_files();
    let fs = files.get(0).unwrap();
    let ep = fs
        .encrypted_payload()
        .expect("the loaded file spec carries an /EP entry");
    assert_eq!(ep.type_(), "EncryptedPayload");
    assert_eq!(ep.subtype(), "AESV3");
    assert_eq!(ep.version(), "1.0");
    // The constructor mirrors the file spec's payload.
    let via_ctor = EncryptedPayload::new(fs);
    assert_eq!(via_ctor.subtype(), "AESV3");
}

#[test]
fn file_specification_ctors() {
    let a = FileSpecification::with_file("/tmp/y.txt".into());
    assert_eq!(a.description(), "");
    let b = FileSpecification::with_file_and_description("/tmp/x.json".into(), "schema".into());
    assert_eq!(b.description(), "schema");
}

#[test]
fn file_specification_value_store_is_case_insensitive() {
    let mut fs = FileSpecification::new();
    fs.set_value("Author", "Ada".into());
    assert_eq!(fs.get_value("author"), "Ada");
    assert_eq!(fs.get_value("AUTHOR"), "Ada");
    // Overwrite via a differently-cased key.
    fs.set_value("AUTHOR", "Grace".into());
    assert_eq!(fs.get_value("Author"), "Grace");
    // Missing key yields an empty string.
    assert_eq!(fs.get_value("missing"), "");
}

#[test]
fn file_specification_property_round_trips() {
    let mut fs = FileSpecification::new();
    fs.set_encoding(FileEncoding::Zip);
    fs.set_include_contents(false);
    fs.set_description("d".into());
    fs.set_af_relationship(AFRelationship::Data);
    fs.set_mime_type("application/json".into());
    fs.set_name("file.json".into());
    fs.set_unicode_name("fïle.json".into());
    fs.set_file_system("URL".into());
    assert_eq!(fs.encoding(), FileEncoding::Zip);
    assert!(!fs.include_contents());
    assert_eq!(fs.description(), "d");
    assert_eq!(fs.af_relationship(), AFRelationship::Data);
    assert_eq!(fs.mime_type(), "application/json");
    assert_eq!(fs.name(), "file.json");
    assert_eq!(fs.unicode_name(), "fïle.json");
    assert_eq!(fs.file_system(), "URL");
}

#[test]
fn collection_add_count_keys_and_indexing() {
    let mut c = EmbeddedFileCollection::new();
    assert_eq!(c.count(), 0);

    let fs1 = FileSpecification::with_file_and_description("/tmp/x.json".into(), "schema".into());
    let mut fs2 = FileSpecification::with_file("/tmp/y.txt".into());
    fs2.set_name("/tmp/y.txt".into());

    c.add(fs1);
    c.add_with_key("custom_key".into(), fs2);
    assert_eq!(c.count(), 2);

    let keys = c.keys();
    assert_eq!(keys.len(), 2);
    assert_eq!(keys[0], "/tmp/x.json"); // add() keys by source path
    assert_eq!(keys[1], "custom_key");

    // Integer indexer (zero-based).
    assert_eq!(c.get(0).unwrap().description(), "schema");
    assert!(c.get(2).is_err());

    // Name indexer + nullable lookup.
    assert!(c.find_by_name("/tmp/y.txt").is_some());
    assert!(c.find_by_name("missing").is_none());
    assert!(c.get_by_name("missing").is_err());
}

#[test]
fn document_save_reopen_round_trip() {
    let content_path = common::unique_temp("aspose_rust_embed_payload.txt");
    let content = b"hello from an embedded file\n";
    std::fs::write(&content_path, content).unwrap();

    let out = common::unique_temp("aspose_rust_embedded_files.pdf");
    {
        let mut doc = Document::open(two_pages()).unwrap();
        assert_eq!(doc.embedded_files().count(), 0);

        let mut spec = FileSpecification::with_file(
            content_path.to_string_lossy().into_owned(),
        );
        spec.set_name("hello.txt".into());
        spec.set_description("a greeting".into());
        spec.set_mime_type("text/plain".into());
        doc.embedded_files_mut().add_with_key("greeting".into(), spec);

        doc.save_to(&out).unwrap();
    }

    // The embedded stream body is written verbatim (uncompressed).
    let saved = std::fs::read(&out).unwrap();
    assert!(
        saved.windows(content.len()).any(|w| w == content),
        "embedded payload bytes must appear in the saved PDF"
    );

    let doc = Document::open(&out).unwrap();
    let ef = doc.embedded_files();
    assert_eq!(ef.count(), 1);
    assert_eq!(ef.keys(), vec!["greeting".to_string()]);

    let spec = ef.get_by_name("hello.txt").unwrap();
    assert_eq!(spec.name(), "hello.txt");
    assert_eq!(spec.description(), "a greeting");
    assert_eq!(spec.mime_type(), "text/plain");

    // /Params /Size is written as the payload length on save.
    assert_eq!(spec.params().size(), content.len() as i32);

    let _ = std::fs::remove_file(&out);
    let _ = std::fs::remove_file(&content_path);
}

#[test]
fn document_verbatim_resave_keeps_no_embedded_files() {
    let out = common::unique_temp("aspose_rust_embedded_verbatim.pdf");
    let original = std::fs::read(two_pages()).unwrap();
    {
        let doc = Document::open(two_pages()).unwrap();
        let _ = doc.embedded_files().count();
        doc.save_to(&out).unwrap();
    }
    let resaved = std::fs::read(&out).unwrap();
    assert_eq!(resaved, original, "untouched embedded-files set must resave verbatim");
    let _ = std::fs::remove_file(&out);
}

#[test]
fn collection_deletion_family() {
    let mut c = EmbeddedFileCollection::new();
    let mut fs = FileSpecification::with_file("/tmp/y.txt".into());
    fs.set_name("payload".into());
    c.add_with_key("custom_key".into(), fs);
    c.add(FileSpecification::with_file("/tmp/z.txt".into()));
    assert_eq!(c.count(), 2);

    c.delete_by_key("custom_key");
    assert_eq!(c.count(), 1);

    c.delete_all();
    assert_eq!(c.count(), 0);
}

#[path = "../common/mod.rs"]
mod common;

#[test]
fn relationship_and_file_system_reach_the_saved_bytes() {
    let content_path = common::unique_temp("aspose_rust_embed_rel.txt");
    std::fs::write(&content_path, b"payload\n").unwrap();

    let out = common::unique_temp("aspose_rust_embedded_rel.pdf");
    {
        let mut doc = Document::open(two_pages()).unwrap();
        let mut spec =
            FileSpecification::with_file(content_path.to_string_lossy().into_owned());
        spec.set_name("data.csv".into());
        spec.set_af_relationship(AFRelationship::Source);
        spec.set_file_system("URL".into());
        doc.embedded_files_mut().add_with_key("data".into(), spec);
        doc.save_to(&out).unwrap();
    }

    let saved = std::fs::read(&out).unwrap();
    let has = |needle: &[u8]| saved.windows(needle.len()).any(|w| w == needle);
    assert!(has(b"/AFRelationship"), "the relationship entry must be written");
    assert!(has(b"/Source"), "it must be written as the §14.13.5 name");
    assert!(has(b"/FS"), "the file-system entry must be written");

    // Both read back through the public carrier.
    let doc = Document::open(&out).unwrap();
    let spec = doc.embedded_files().get_by_name("data.csv").unwrap();
    assert_eq!(spec.af_relationship(), AFRelationship::Source);
    assert_eq!(spec.file_system(), "URL");

    let _ = std::fs::remove_file(&out);
    let _ = std::fs::remove_file(&content_path);
}

#[test]
fn an_unspecified_relationship_writes_no_entry() {
    let content_path = common::unique_temp("aspose_rust_embed_norel.txt");
    std::fs::write(&content_path, b"payload\n").unwrap();

    let out = common::unique_temp("aspose_rust_embedded_norel.pdf");
    {
        let mut doc = Document::open(two_pages()).unwrap();
        let mut spec =
            FileSpecification::with_file(content_path.to_string_lossy().into_owned());
        spec.set_name("plain.txt".into());
        // The default relationship, and no file system: an absent
        // /AFRelationship already means "unspecified" (§14.13.5), and
        // an absent /FS means the standard PDF file-name grammar.
        doc.embedded_files_mut().add_with_key("plain".into(), spec);
        doc.save_to(&out).unwrap();
    }

    let saved = std::fs::read(&out).unwrap();
    let has = |needle: &[u8]| saved.windows(needle.len()).any(|w| w == needle);
    assert!(has(b"/Filespec"), "the specification itself must be written");
    assert!(!has(b"/AFRelationship"), "the default relationship must stay out");
    assert!(!has(b"/FS"), "an unset file system must stay out");

    let _ = std::fs::remove_file(&out);
    let _ = std::fs::remove_file(&content_path);
}
