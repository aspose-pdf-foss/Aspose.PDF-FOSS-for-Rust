// Hand-authored gate driver — parse_serialize_roundtrip gate.
// Primitive: pdf_writer_incremental
//
// For each fixture under tests/fixtures/pdf_writer_incremental/
// (or Feedstock/PDF/Fixtures/pdf_writer_incremental/ via
// SPECDEV_CORPUS_DIR), build a fixture-specific dirty list,
// run `append_incremental`, parse the result by walking xref
// + objects::parse_object, and assert: (a) the post-mutation
// Value of each dirty id matches the expected new Value, and
// (b) every other indirect object's Value is byte-equal to its
// original parsed Value.
//
// Composes lexer + xref + objects + pdf_writer_incremental.
// flate is mounted because xref's #[path] tree includes it
// transitively.

#[path = "../../src/internal/lexer.rs"]
mod lexer;

#[path = "../../src/internal/flate.rs"]
mod flate;

#[path = "../../src/internal/xref.rs"]
mod xref;

#[path = "../../src/internal/objects.rs"]
mod objects;

#[path = "../../src/internal/pdf_writer_incremental.rs"]
mod pdf_writer_incremental;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use objects::{IndirectObject, Value};
use pdf_writer_incremental::{append_incremental, DirtyObject};
use xref::Entry;

fn corpus_dir() -> PathBuf {
    if let Ok(env_dir) = std::env::var("SPECDEV_CORPUS_DIR") {
        if !env_dir.is_empty() {
            return PathBuf::from(env_dir);
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("pdf_writer_incremental")
}

fn build_dump(input: &[u8]) -> BTreeMap<(u32, u16), IndirectObject> {
    let table = xref::parse(input).expect("xref parse");
    let mut dump = BTreeMap::new();
    for entry in &table.entries {
        if let Entry::InUse { id, gen, offset } = *entry {
            let obj = objects::parse_object(input, offset as usize)
                .unwrap_or_else(|e| panic!("parse_object {id}: {e}"));
            dump.insert((id, gen), obj);
        }
    }
    dump
}

fn lit_string(s: &str) -> Value {
    Value::String(s.as_bytes().to_vec())
}

fn dict_lookup<'a>(d: &'a [(Vec<u8>, Value)], key: &[u8]) -> Option<&'a Value> {
    d.iter().find(|(k, _)| k.as_slice() == key).map(|(_, v)| v)
}

// Run append_incremental, re-build the dump from the result,
// and verify (a) dirty values match expectation, (b) other
// objects are unchanged from their pre-mutation Value.
fn check_roundtrip(
    pdf_path: &Path,
    dirty: &[DirtyObject],
    expected_after: &BTreeMap<u32, Value>,
) {
    let original = fs::read(pdf_path).expect("read fixture");
    let pre_dump = build_dump(&original);

    let out = append_incremental(&original, dirty)
        .unwrap_or_else(|e| panic!("append failed for {}: {}", pdf_path.display(), e));
    assert!(out.len() > original.len(),
            "{}: output is not longer than input", pdf_path.display());

    let post_dump = build_dump(&out);

    // (a) dirty values match expectation.
    for d in dirty {
        let post = post_dump.get(&(d.id, d.generation))
            .unwrap_or_else(|| panic!(
                "{}: dirty id {} not in post-dump",
                pdf_path.display(), d.id));
        let expected = expected_after.get(&d.id)
            .unwrap_or_else(|| panic!(
                "{}: no expectation for dirty id {}",
                pdf_path.display(), d.id));
        assert_eq!(&post.value, expected,
                   "{}: dirty id {} post-mutation value mismatch",
                   pdf_path.display(), d.id);
    }

    // (b) non-dirty objects are unchanged.
    let dirty_ids: Vec<u32> = dirty.iter().map(|d| d.id).collect();
    for ((id, gen), pre_obj) in &pre_dump {
        if dirty_ids.contains(id) { continue; }
        let post_obj = post_dump.get(&(*id, *gen))
            .unwrap_or_else(|| panic!(
                "{}: pre-existing id {id} {gen} missing from post-dump",
                pdf_path.display()));
        assert_eq!(post_obj.value, pre_obj.value,
                   "{}: non-dirty id {id} {gen} value changed",
                   pdf_path.display());
    }
}

#[test]
fn info_update_round_trip() {
    let path = corpus_dir().join("info_update.pdf");
    assert!(path.is_file(), "missing fixture: {}", path.display());

    // Pre-condition: object 4 has /Title (Original) /Author (Tester).
    let pre = fs::read(&path).expect("read");
    let pre_dump = build_dump(&pre);
    let info = pre_dump.get(&(4, 0)).expect("info obj 4");
    if let Value::Dict(entries) = &info.value {
        match dict_lookup(entries, b"Title") {
            Some(Value::String(b)) => assert_eq!(b, b"Original"),
            _ => panic!("info /Title not a string"),
        }
    } else {
        panic!("info obj is not a dict");
    }

    // Replace obj 4 with /Title (Updated) /Author (Tester) /Subject (Test).
    let new_info = Value::Dict(vec![
        (b"Title".to_vec(), lit_string("Updated")),
        (b"Author".to_vec(), lit_string("Tester")),
        (b"Subject".to_vec(), lit_string("Test")),
    ]);

    let dirty = vec![DirtyObject {
        id: 4,
        generation: 0,
        value: new_info.clone(),
    }];

    let mut expected = BTreeMap::new();
    expected.insert(4u32, new_info);
    check_roundtrip(&path, &dirty, &expected);
}

#[test]
fn add_object_round_trip() {
    let path = corpus_dir().join("add_object.pdf");
    assert!(path.is_file(), "missing fixture: {}", path.display());

    // Add a brand-new id 4 holding /Answer 42 — exercises the
    // /Size bump path (orig /Size = 4 → new max id+1 = 5).
    let new_obj = Value::Dict(vec![
        (b"Answer".to_vec(), Value::Integer(42)),
    ]);

    let dirty = vec![DirtyObject {
        id: 4,
        generation: 0,
        value: new_obj.clone(),
    }];

    let mut expected = BTreeMap::new();
    expected.insert(4u32, new_obj);
    check_roundtrip(&path, &dirty, &expected);
}

#[test]
fn gap_dirty_round_trip() {
    let path = corpus_dir().join("gap_dirty.pdf");
    assert!(path.is_file(), "missing fixture: {}", path.display());

    // Mutate obj 2 and obj 5 — ids 3 and 4 are NOT dirty,
    // forcing the writer to emit two xref subsections.
    let new_pages = Value::Dict(vec![
        (b"Type".to_vec(), Value::Name(b"Pages".to_vec())),
        (b"Kids".to_vec(), Value::Array(vec![
            Value::Ref { id: 3, gen: 0 },
        ])),
        (b"Count".to_vec(), Value::Integer(1)),
        (b"Comment".to_vec(), lit_string("annotated")),
    ]);
    let new_int = Value::Integer(99);

    let dirty = vec![
        DirtyObject { id: 2, generation: 0, value: new_pages.clone() },
        DirtyObject { id: 5, generation: 0, value: new_int.clone() },
    ];

    let mut expected = BTreeMap::new();
    expected.insert(2u32, new_pages);
    expected.insert(5u32, new_int);
    check_roundtrip(&path, &dirty, &expected);
}
