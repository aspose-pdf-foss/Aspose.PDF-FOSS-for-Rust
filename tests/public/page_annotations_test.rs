//! Public-API smoke driver — `Page.Annotations`.
//!
//! Ports the cpp page_annotations + annotation_save_through tests: the
//! live per-page collection (empty default, stable identity, add /
//! contains / remove / accept) and the `/Annots` save-through round-trip
//! (add → save → reopen → read back count + types).

use aspose_pdf_foss::{
    Annotation, AnnotationSelector, AnnotationType, CircleAnnotation, Document, Rectangle,
    SquareAnnotation, TextAnnotation,
};
use std::path::{Path, PathBuf};

fn hello_world() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/text_extractor/hello_world.pdf")
}

/// Build a PDF carrying three annotations (Text "note one", Square
/// "box two", Circle) at `out`, via the lib's own save-through.
fn make_three_annot_pdf(out: &Path) {
    let doc = Document::open(hello_world()).unwrap();
    let __pg0 = doc.pages().get(1).unwrap();
    let annots = __pg0.annotations();
    let mut t = TextAnnotation::new(&doc);
    t.set_rect(Rectangle::new(10.0, 710.0, 30.0, 730.0, false));
    t.set_contents("note one".into());
    let mut s = SquareAnnotation::new(&doc);
    s.set_rect(Rectangle::new(40.0, 710.0, 80.0, 740.0, false));
    s.set_contents("box two".into());
    let c = CircleAnnotation::new(&doc);
    annots.add(Box::new(t));
    annots.add(Box::new(s));
    annots.add(Box::new(c));
    doc.save_to(out).unwrap();
}

#[test]
fn empty_by_default() {
    let doc = Document::open(hello_world()).unwrap();
    assert_eq!(doc.pages().get(1).unwrap().annotations().count(), 0);
}

#[test]
fn stable_identity_across_calls() {
    // `doc.pages().get(n)` returns an owned `Page` handle that shares its
    // sub-collections (annotations / resources / …) by `Rc`, so two
    // independent `get(1)` handles observe the SAME annotation storage —
    // a mutation through one is visible through the other.
    let doc = Document::open(hello_world()).unwrap();
    let pg1 = doc.pages().get(1).unwrap();
    let pg2 = doc.pages().get(1).unwrap();
    assert_eq!(pg2.annotations().count(), 0);
    pg1.annotations().add(Box::new(TextAnnotation::new(&doc)));
    assert_eq!(pg2.annotations().count(), 1);
}

#[test]
fn add_roundtrips_through_collection() {
    let doc = Document::open(hello_world()).unwrap();
    doc.pages().get(1).unwrap().annotations().add(Box::new(TextAnnotation::new(&doc)));
    doc.pages().get(1).unwrap().annotations().add(Box::new(CircleAnnotation::new(&doc)));

    let __pg3 = doc.pages().get(1).unwrap();
    let annots = __pg3.annotations();
    assert_eq!(annots.count(), 2);
    assert_eq!(annots.get(0).unwrap().annotation_type(), AnnotationType::Text);
    assert_eq!(annots.get(1).unwrap().annotation_type(), AnnotationType::Circle);

    let mut sel = AnnotationSelector::new();
    annots.accept(&mut sel);
    assert_eq!(sel.selected(), &[AnnotationType::Text, AnnotationType::Circle]);
}

#[test]
fn remove_and_contains() {
    let doc = Document::open(hello_world()).unwrap();
    let mut a = TextAnnotation::new(&doc);
    {
        use aspose_pdf_foss::Annotation;
        a.set_name("first".into());
    }
    let mut b = TextAnnotation::new(&doc);
    {
        use aspose_pdf_foss::Annotation;
        b.set_name("second".into());
    }
    let __pg4 = doc.pages().get(1).unwrap();
    let annots = __pg4.annotations();
    annots.add(Box::new(a));
    annots.add(Box::new(b));
    assert_eq!(annots.count(), 2);

    let probe = {
        let mut p = TextAnnotation::new(&doc);
        use aspose_pdf_foss::Annotation;
        p.set_name("first".into());
        p
    };
    assert!(annots.contains(&probe));
    assert!(annots.remove(&probe));
    assert!(!annots.contains(&probe));
    assert_eq!(annots.count(), 1);
}

#[test]
fn save_through_round_trip() {
    let out = common::unique_temp("aspose_rust_page_annots.pdf");
    {
        let doc = Document::open(hello_world()).unwrap();
        let __pg5 = doc.pages().get(1).unwrap();
        let annots = __pg5.annotations();
        annots.add(Box::new(TextAnnotation::new(&doc)));
        annots.add(Box::new(CircleAnnotation::new(&doc)));
        annots.add(Box::new(TextAnnotation::new(&doc)));
        doc.save_to(&out).unwrap();
    }
    let doc = Document::open(&out).unwrap();
    let __pg6 = doc.pages().get(1).unwrap();
    let annots = __pg6.annotations();
    assert_eq!(annots.count(), 3);
    assert_eq!(annots.get(0).unwrap().annotation_type(), AnnotationType::Text);
    assert_eq!(annots.get(1).unwrap().annotation_type(), AnnotationType::Circle);
    assert_eq!(annots.get(2).unwrap().annotation_type(), AnnotationType::Text);
    let _ = std::fs::remove_file(&out);
}

// ---- ported from C++ annotation_save_through + annotation_load_on_open ----

#[test]
fn text_annotation_persists_with_rect_and_contents() {
    let out = common::unique_temp("aspose_rust_annot_st.pdf");
    {
        let doc = Document::open(hello_world()).unwrap();
        let mut a = TextAnnotation::new(&doc);
        a.set_rect(Rectangle::new(100.0, 700.0, 200.0, 720.0, false));
        a.set_contents("Cluster save-through proof".into());
        doc.pages().get(1).unwrap().annotations().add(Box::new(a));
        doc.save_to(&out).unwrap();
    }
    let doc = Document::open(&out).unwrap();
    let __pg7 = doc.pages().get(1).unwrap();
    let annots = __pg7.annotations();
    assert_eq!(annots.count(), 1);
    let a = annots.get(0).unwrap();
    assert_eq!(a.annotation_type(), AnnotationType::Text);
    assert_eq!(a.contents(), "Cluster save-through proof");
    let r = a.rect();
    assert_eq!([r.llx(), r.lly(), r.urx(), r.ury()], [100.0, 700.0, 200.0, 720.0]);
    let _ = std::fs::remove_file(&out);
}

#[test]
fn no_annotations_leaves_bytes_untouched() {
    let src = std::fs::read(hello_world()).unwrap();
    let out = common::unique_temp("aspose_rust_annot_noop.pdf");
    let doc = Document::open(hello_world()).unwrap();
    // No annotation changes.
    doc.save_to(&out).unwrap();
    assert_eq!(std::fs::read(&out).unwrap(), src);
    let _ = std::fs::remove_file(&out);
}

#[test]
fn multiple_annotations_all_persist() {
    let out = common::unique_temp("aspose_rust_annot_multi.pdf");
    {
        let doc = Document::open(hello_world()).unwrap();
        let __pg8 = doc.pages().get(1).unwrap();
        let annots = __pg8.annotations();
        let mut a = TextAnnotation::new(&doc);
        a.set_rect(Rectangle::new(10.0, 10.0, 50.0, 30.0, false));
        a.set_contents("first".into());
        let mut b = TextAnnotation::new(&doc);
        b.set_rect(Rectangle::new(60.0, 60.0, 90.0, 80.0, false));
        b.set_contents("second".into());
        annots.add(Box::new(a));
        annots.add(Box::new(b));
        doc.save_to(&out).unwrap();
    }
    let doc = Document::open(&out).unwrap();
    let __pg9 = doc.pages().get(1).unwrap();
    let annots = __pg9.annotations();
    assert_eq!(annots.count(), 2);
    assert_eq!(annots.get(0).unwrap().contents(), "first");
    assert_eq!(annots.get(1).unwrap().contents(), "second");
    let _ = std::fs::remove_file(&out);
}

#[test]
fn round_trip_count_types_contents() {
    let out = common::unique_temp("aspose_rust_annot_rt.pdf");
    make_three_annot_pdf(&out);
    let doc = Document::open(&out).unwrap();
    let __pg10 = doc.pages().get(1).unwrap();
    let annots = __pg10.annotations();
    assert_eq!(annots.count(), 3);
    assert_eq!(annots.get(0).unwrap().annotation_type(), AnnotationType::Text);
    assert_eq!(annots.get(0).unwrap().contents(), "note one");
    assert_eq!(annots.get(1).unwrap().annotation_type(), AnnotationType::Square);
    assert_eq!(annots.get(1).unwrap().contents(), "box two");
    assert_eq!(annots.get(2).unwrap().annotation_type(), AnnotationType::Circle);
    let _ = std::fs::remove_file(&out);
}

#[test]
fn delete_on_disk_reduces_count() {
    let out = common::unique_temp("aspose_rust_annot_del1.pdf");
    let out2 = common::unique_temp("aspose_rust_annot_del2.pdf");
    make_three_annot_pdf(&out);
    {
        let doc = Document::open(&out).unwrap();
        let __pg11 = doc.pages().get(1).unwrap();
        let annots = __pg11.annotations();
        assert_eq!(annots.count(), 3);
        annots.delete_at(1).unwrap(); // drop the Square
        assert_eq!(annots.count(), 2);
        doc.save_to(&out2).unwrap();
    }
    let doc = Document::open(&out2).unwrap();
    let __pg12 = doc.pages().get(1).unwrap();
    let annots = __pg12.annotations();
    assert_eq!(annots.count(), 2);
    assert_eq!(annots.get(0).unwrap().annotation_type(), AnnotationType::Text);
    assert_eq!(annots.get(1).unwrap().annotation_type(), AnnotationType::Circle);
    let _ = std::fs::remove_file(&out);
    let _ = std::fs::remove_file(&out2);
}

#[test]
fn delete_all_on_disk_clears() {
    let out = common::unique_temp("aspose_rust_annot_clr1.pdf");
    let out2 = common::unique_temp("aspose_rust_annot_clr2.pdf");
    make_three_annot_pdf(&out);
    {
        let doc = Document::open(&out).unwrap();
        let __pg13 = doc.pages().get(1).unwrap();
        let annots = __pg13.annotations();
        assert_eq!(annots.count(), 3);
        annots.delete_all();
        assert_eq!(annots.count(), 0);
        doc.save_to(&out2).unwrap();
    }
    let doc = Document::open(&out2).unwrap();
    assert_eq!(doc.pages().get(1).unwrap().annotations().count(), 0);
    let _ = std::fs::remove_file(&out);
    let _ = std::fs::remove_file(&out2);
}

#[test]
fn read_only_resave_is_lossless() {
    let out = common::unique_temp("aspose_rust_annot_loss1.pdf");
    let out2 = common::unique_temp("aspose_rust_annot_loss2.pdf");
    make_three_annot_pdf(&out);
    {
        let doc = Document::open(&out).unwrap();
        assert_eq!(doc.pages().get(1).unwrap().annotations().count(), 3);
        doc.save_to(&out2).unwrap(); // no changes
    }
    let doc = Document::open(&out2).unwrap();
    let __pg14 = doc.pages().get(1).unwrap();
    let annots = __pg14.annotations();
    assert_eq!(annots.count(), 3);
    assert_eq!(annots.get(0).unwrap().contents(), "note one");
    let _ = std::fs::remove_file(&out);
    let _ = std::fs::remove_file(&out2);
}

#[path = "../common/mod.rs"]
mod common;
