// Public-API smoke driver — outlines (bookmark) tree.
//
// Ports the cpp outlines_smoke_test BuildTraverseAndNavigate: build
// a two-level tree through Document.outlines(), then exercise the
// 1-based indexer, sibling / parent navigation, level, the open
// flag's effect on visible_count, and a destination round-trip.
// Identity assertions use ptr_eq (the Rust analogue of the cpp
// pointer-identity checks), plus a save / reopen round-trip exercising
// the catalog /Outlines persistence (titles, style flags, the open-flag
// /Count sign, nesting, and destinations).

use aspose_pdf_foss::{
    Document, NamedAction, Outlines, OutlineItemCollection, PredefinedAction,
    XYZExplicitDestination,
};
use std::path::{Path, PathBuf};

fn two_pages() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/text_extractor/two_pages.pdf")
}

#[test]
fn build_traverse_and_navigate() {
    let doc = Document::new();
    let root = doc.outlines();
    assert_eq!(root.count(), 0);

    let a = OutlineItemCollection::new(root);
    a.set_title("Chapter 1".into());
    a.set_bold(true);
    root.add(&a);

    let b = OutlineItemCollection::new(root);
    b.set_title("Chapter 2".into());
    b.set_italic(true);
    root.add(&b);

    assert_eq!(root.count(), 2);
    assert_eq!(root.get(1).title(), "Chapter 1");
    assert!(root.get(1).bold());
    assert!(!root.get(1).italic());
    assert!(root.get(2).italic());

    assert_eq!(root.first().unwrap().title(), "Chapter 1");
    assert_eq!(root.last().unwrap().title(), "Chapter 2");
    assert_eq!(root.get(1).next().unwrap().title(), "Chapter 2");
    assert_eq!(root.get(2).prev().unwrap().title(), "Chapter 1");
    assert!(root.get(1).has_next());
    assert!(!root.get(2).has_next());
    assert_eq!(root.get(1).level(), 1);

    // root[1].Parent() is the root collection.
    match root.get(1).parent() {
        Some(Outlines::Collection(p)) => assert!(p.ptr_eq(root)),
        _ => panic!("expected the root collection as parent"),
    }

    // Nest a section under chapter 1.
    let sec = OutlineItemCollection::new(root);
    sec.set_title("Section 1.1".into());
    root.get(1).add(&sec);
    assert_eq!(root.get(1).count(), 1);

    let child = root.get(1).first().unwrap();
    assert_eq!(child.title(), "Section 1.1");
    assert_eq!(child.level(), 2);
    match child.parent() {
        Some(Outlines::Item(p)) => assert!(p.ptr_eq(&root.get(1))),
        _ => panic!("expected chapter 1 as parent"),
    }

    // Visible count honours the open flag.
    assert_eq!(root.visible_count(), 3);
    root.get(1).set_open(false);
    assert_eq!(root.visible_count(), 2);

    // Destination round-trip.
    let jump = OutlineItemCollection::new(root);
    jump.set_title("Jump".into());
    jump.set_destination(Box::new(XYZExplicitDestination::new(1, 0.0, 700.0, 1.0)));
    root.add(&jump);
    assert_eq!(root.get(3).destination().unwrap().to_string(), "1 XYZ 0 700 1");
}

#[test]
fn action_round_trip() {
    let doc = Document::new();
    let root = doc.outlines();
    let item = OutlineItemCollection::new(root);
    item.set_action(Box::new(NamedAction::from_predefined(PredefinedAction::LastPage)));
    root.add(&item);
    let act = root.get(1).action().unwrap();
    assert_eq!(act.to_string(), "LastPage");
    assert_eq!(act.get_ecma_script_string(), "");
}

#[test]
fn save_reopen_round_trip() {
    let out = common::unique_temp("aspose_rust_outlines.pdf");
    {
        let doc = Document::open(two_pages()).unwrap();
        // A real PDF without bookmarks loads an empty outline tree.
        assert_eq!(doc.outlines().count(), 0);
        let root = doc.outlines();

        let ch1 = OutlineItemCollection::new(root);
        ch1.set_title("Chapter 1".into());
        ch1.set_bold(true);
        ch1.set_destination(Box::new(XYZExplicitDestination::new(1, 0.0, 700.0, 1.0)));
        root.add(&ch1);

        let sec = OutlineItemCollection::new(root);
        sec.set_title("Section 1.1".into());
        sec.set_destination(Box::new(XYZExplicitDestination::new(2, 0.0, 500.0, 1.0)));
        ch1.add(&sec);
        // Collapse chapter 1 so the /Count sign round-trips.
        ch1.set_open(false);

        let ch2 = OutlineItemCollection::new(root);
        ch2.set_title("Chapter 2".into());
        ch2.set_italic(true);
        root.add(&ch2);

        doc.save_to(&out).unwrap();
    }

    let doc = Document::open(&out).unwrap();
    let root = doc.outlines();
    assert_eq!(root.count(), 2);

    let ch1 = root.get(1);
    assert_eq!(ch1.title(), "Chapter 1");
    assert!(ch1.bold());
    assert!(!ch1.italic());
    assert!(!ch1.open());
    assert_eq!(ch1.destination().unwrap().to_string(), "1 XYZ 0 700 1");

    assert_eq!(ch1.count(), 1);
    let sec = ch1.first().unwrap();
    assert_eq!(sec.title(), "Section 1.1");
    assert_eq!(sec.destination().unwrap().to_string(), "2 XYZ 0 500 1");
    assert_eq!(sec.level(), 2);

    let ch2 = root.get(2);
    assert_eq!(ch2.title(), "Chapter 2");
    assert!(ch2.italic());
    assert!(ch2.open());
    assert!(ch2.destination().is_none());

    let _ = std::fs::remove_file(&out);
}

#[test]
fn verbatim_resave_keeps_no_outlines() {
    // Opening then saving without touching the tree must not write an
    // /Outlines (the tree is not dirty).
    let out = common::unique_temp("aspose_rust_outlines_verbatim.pdf");
    let original = std::fs::read(two_pages()).unwrap();
    {
        let doc = Document::open(two_pages()).unwrap();
        let _ = doc.outlines().count();
        doc.save_to(&out).unwrap();
    }
    let resaved = std::fs::read(&out).unwrap();
    assert_eq!(resaved, original, "untouched outline tree must resave verbatim");
    let _ = std::fs::remove_file(&out);
}

#[test]
fn delete_contains_and_remove() {
    let doc = Document::new();
    let root = doc.outlines();
    let a = OutlineItemCollection::new(root);
    a.set_title("A".into());
    let b = OutlineItemCollection::new(root);
    b.set_title("B".into());
    root.add(&a);
    root.add(&b);
    assert_eq!(root.count(), 2);

    assert!(root.contains(&a));
    assert!(root.remove(&a));
    assert!(!root.contains(&a));
    assert_eq!(root.count(), 1);

    root.delete_by_name("B");
    assert_eq!(root.count(), 0);

    // Child self-delete via parent back-link.
    let p = OutlineItemCollection::new(root);
    let c = OutlineItemCollection::new(root);
    root.add(&p);
    p.add(&c);
    assert_eq!(p.count(), 1);
    c.delete();
    assert_eq!(p.count(), 0);
}

#[path = "../common/mod.rs"]
mod common;

/// A bookmark given an action writes `/A` and leaves `/Dest` off:
/// §12.3.3 says the action wins when both are present, so writing both
/// would say two things at once.
#[test]
fn a_bookmark_carries_its_action() {
    use aspose_pdf_foss::GoToURIAction;
    let doc = Document::new();
    {
        let root = doc.outlines();
        let item = OutlineItemCollection::new(root);
        item.set_title("Example".into());
        item.set_action(Box::new(GoToURIAction::new(
            "https://example.invalid/b".to_string(),
        )));
        root.add(&item);
    }
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();
    let text = String::from_utf8_lossy(&out);
    assert!(text.contains("/S /URI"), "the action subtype");
    assert!(text.contains("https://example.invalid/b"), "the target");
}

/// The whole `/Outlines` tree is rewritten whenever any bookmark
/// changes, and a loaded item knew only its title, style and
/// destination — so adding one bookmark stripped every other
/// bookmark's action. A reader's `/A` is now carried verbatim unless it
/// is a `GoTo`, which is read as a destination instead.
#[test]
fn adding_a_bookmark_does_not_strip_the_actions_already_there() {
    use aspose_pdf_foss::GoToURIAction;

    let first = common::unique_temp("aspose_rust_outline_carry_1.pdf");
    {
        let doc = Document::open(two_pages()).unwrap();
        {
            let root = doc.outlines();
            let item = OutlineItemCollection::new(root);
            item.set_title("Site".into());
            item.set_action(Box::new(GoToURIAction::new(
                "https://example.invalid/keep".to_string(),
            )));
            root.add(&item);
        }
        doc.save_to(&first).unwrap();
    }
    assert!(
        String::from_utf8_lossy(&std::fs::read(&first).unwrap()).contains("/S /URI"),
        "the first bookmark acts"
    );

    let second = common::unique_temp("aspose_rust_outline_carry_2.pdf");
    {
        let doc = Document::open(&first).unwrap();
        {
            let root = doc.outlines();
            assert_eq!(root.count(), 1, "the bookmark is read back");
            let added = OutlineItemCollection::new(root);
            added.set_title("Plain".into());
            root.add(&added);
        }
        doc.save_to(&second).unwrap();
    }
    let text = String::from_utf8_lossy(&std::fs::read(&second).unwrap()).into_owned();

    // The rewritten tree carries both titles, and the first one still
    // runs its URI (once per revision, the original being untouched).
    assert!(text.contains("(Plain)"), "the new bookmark");
    assert_eq!(
        text.matches("https://example.invalid/keep").count(),
        2,
        "the action survived the rewrite"
    );
}

/// `CreateBookmarks()` is the overload that reads the document: it
/// fills the editor's own list from the bound document's outline, with
/// the title, the style, the nesting level and the page a destination
/// names.
#[test]
fn the_bookmark_editor_reads_the_documents_outline() {
    use aspose_pdf_foss::PdfBookmarkEditor;

    let path = common::unique_temp("aspose_rust_outline_editor.pdf");
    {
        let doc = Document::open(two_pages()).unwrap();
        {
            let root = doc.outlines();
            let parent = OutlineItemCollection::new(root);
            parent.set_title("Chapter".into());
            parent.set_bold(true);
            parent.set_destination(Box::new(XYZExplicitDestination::new(2, 0.0, 792.0, 0.0)));
            let child = OutlineItemCollection::new(root);
            child.set_title("Section".into());
            parent.add(&child);
            root.add(&parent);
        }
        doc.save_to(&path).unwrap();
    }

    let mut editor = PdfBookmarkEditor::with_document(Document::open(&path).unwrap());
    assert_eq!(editor.extract_bookmarks().count(), 0, "the list starts empty");
    editor.create_bookmarks();

    let bookmarks = editor.extract_bookmarks();
    assert_eq!(bookmarks.count(), 1, "one top-level bookmark");
    let chapter = bookmarks.get(0).expect("the chapter");
    assert_eq!(chapter.title(), "Chapter");
    assert!(chapter.bold_flag(), "the style came across");
    assert_eq!(chapter.level(), 1, "levels are 1-based");
    assert_eq!(chapter.page_number(), 2, "the page its destination names");
    assert_eq!(chapter.child_items().count(), 1, "and its child");
    assert_eq!(chapter.child_items().get(0).expect("the section").level(), 2);

    // An unbound editor reads nothing rather than failing.
    let mut bare = PdfBookmarkEditor::new();
    bare.create_bookmarks();
    assert_eq!(bare.extract_bookmarks().count(), 0);

    let _ = std::fs::remove_file(&path);
}
