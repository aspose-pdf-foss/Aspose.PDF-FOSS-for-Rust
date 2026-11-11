// Public-API smoke driver — Forms extras. Ports three cpp tests:
//   * forms_field_smoke_test            — Field / TextBoxField surface
//   * forms_load_on_open_smoke_test     — /AcroForm add→save→reopen
//   * forms_acroform_save_through_smoke — /AcroForm byte-level emit
//
// The Rust form load-on-open surfaces reopened fields through the
// `Field` trait (`&dyn Field`); there is no concrete downcast, so
// type-specific read-backs (checkbox `checked()`, combo `options()`)
// after reopen are asserted at byte level on the saved PDF instead of
// through a downcast.

use aspose_pdf_foss::{
    Annotation, AnnotationType, CheckboxField, ChoiceField, ComboBoxField, Document, Field,
    ListBoxField,
    PasswordBoxField, Point, RadioButtonField, Rectangle, SignatureField, TextBoxField,
    VerticalAlignment,
};
use std::path::{Path, PathBuf};

fn hello_world() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/text_extractor/hello_world.pdf")
}

fn rect(a: f64, b: f64, c: f64, d: f64) -> Rectangle {
    Rectangle::new(a, b, c, d, false)
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

// ---------------------------------------------------------------------------
// forms_field_smoke_test
// ---------------------------------------------------------------------------

#[test]
fn text_box_construct_defaults() {
    let doc = Document::new();
    let f = TextBoxField::new(&doc);
    assert_eq!(f.annotation_type(), AnnotationType::Widget);
    assert!(f.partial_name().is_empty());
    assert!(f.value().is_empty());
    // Rust `count()` is the number of widget kids (0 for a fresh
    // field); a single-widget field is not a group.
    assert_eq!(f.count(), 0);
    assert!(!f.is_group());
}

#[test]
fn text_box_accessors_roundtrip() {
    let doc = Document::new();
    let mut f = TextBoxField::new(&doc);

    f.set_partial_name("Last_Name".into());
    f.set_alternate_name("Surname".into());
    f.set_mapping_name("dc:contributor".into());
    f.set_value("Smith".into());
    f.set_annotation_index(2);
    f.set_is_shared_field(true);
    f.set_fit_into_rectangle(true);
    f.set_max_font_size(18.0);
    f.set_min_font_size(8.0);
    f.set_tab_order(5);

    assert_eq!(f.partial_name(), "Last_Name");
    assert_eq!(f.alternate_name(), "Surname");
    assert_eq!(f.mapping_name(), "dc:contributor");
    assert_eq!(f.value(), "Smith");
    assert_eq!(f.annotation_index(), 2);
    assert!(f.is_shared_field());
    assert!(f.fit_into_rectangle());
    assert_eq!(f.max_font_size(), 18.0);
    assert_eq!(f.min_font_size(), 8.0);
    assert_eq!(f.tab_order(), 5);

    f.set_multiline(true);
    f.set_scrollable(false);
    f.set_force_combs(true);
    f.set_max_len(120);
    f.set_spell_check(false);
    f.set_text_vertical_alignment(VerticalAlignment::Center);
    assert!(f.multiline());
    assert!(!f.scrollable());
    assert!(f.force_combs());
    assert_eq!(f.max_len(), 120);
    assert!(!f.spell_check());
    assert_eq!(f.text_vertical_alignment(), VerticalAlignment::Center);
}

#[test]
fn text_box_rect_ctor_and_set_position() {
    let doc = Document::open(hello_world()).unwrap();
    let page = doc.pages().get(1).unwrap();
    let mut f = TextBoxField::on_page(&page, rect(100.0, 200.0, 250.0, 240.0));
    assert_eq!(f.rect().llx(), 100.0);
    assert_eq!(f.rect().ury(), 240.0);
    assert_eq!(f.rect().width(), 150.0);
    assert_eq!(f.rect().height(), 40.0);

    // SetPosition moves the Rect; width / height preserved.
    f.set_position(Point::new(50.0, 60.0));
    assert_eq!(f.rect().llx(), 50.0);
    assert_eq!(f.rect().lly(), 60.0);
    assert_eq!(f.rect().width(), 150.0);
    assert_eq!(f.rect().height(), 40.0);
}

#[test]
fn text_box_recalculate() {
    let doc = Document::new();
    let mut f = TextBoxField::new(&doc);
    assert!(f.recalculate());
}

#[test]
#[should_panic(expected = "barcode_rendering")]
fn text_box_barcode_is_pending() {
    let doc = Document::new();
    let mut f = TextBoxField::new(&doc);
    f.add_barcode("978-0-13-235088-4".into());
}

// ---------------------------------------------------------------------------
// forms_load_on_open_smoke_test
// ---------------------------------------------------------------------------

#[test]
fn text_field_round_trip() {
    let out = common::unique_temp("asp_rust_form_loo_text.pdf");
    {
        let mut doc = Document::open(hello_world()).unwrap();
        let mut tf = {
            let page = doc.pages().get(1).unwrap();
            TextBoxField::on_page(&page, rect(100.0, 600.0, 300.0, 620.0))
        };
        tf.set_value("Jane Doe".into());
        doc.form_mut().add_named(Box::new(tf), "fullname".into(), 1);
        doc.save_to(&out).unwrap();
    }

    let doc = Document::open(&out).unwrap();
    assert_eq!(doc.form().count(), 1);
    assert!(doc.form().has_field_by_name("fullname"));
    let f = doc.form().get_by_name("fullname").unwrap();
    assert_eq!(f.value(), "Jane Doe");
    let _ = std::fs::remove_file(&out);
}

#[test]
fn multiple_fields_round_trip() {
    let out = common::unique_temp("asp_rust_form_loo_multi.pdf");
    {
        let mut doc = Document::open(hello_world()).unwrap();
        let (mut a, mut b, c) = {
            let page = doc.pages().get(1).unwrap();
            (
                TextBoxField::on_page(&page, rect(100.0, 600.0, 300.0, 620.0)),
                TextBoxField::on_page(&page, rect(100.0, 560.0, 300.0, 580.0)),
                TextBoxField::on_page(&page, rect(100.0, 520.0, 300.0, 540.0)),
            )
        };
        a.set_value("first".into());
        b.set_value("second".into());
        // c left empty
        doc.form_mut().add_named(Box::new(a), "f1".into(), 1);
        doc.form_mut().add_named(Box::new(b), "f2".into(), 1);
        doc.form_mut().add_named(Box::new(c), "f3".into(), 1);
        doc.save_to(&out).unwrap();
    }

    let doc = Document::open(&out).unwrap();
    assert_eq!(doc.form().count(), 3);
    assert_eq!(doc.form().get_by_name("f1").unwrap().value(), "first");
    assert_eq!(doc.form().get_by_name("f2").unwrap().value(), "second");
    assert_eq!(doc.form().get_by_name("f3").unwrap().value(), "");
    let _ = std::fs::remove_file(&out);
}

#[test]
fn remove_after_reopen_drops_field() {
    let out = common::unique_temp("asp_rust_form_loo_rm1.pdf");
    let out2 = common::unique_temp("asp_rust_form_loo_rm2.pdf");
    {
        let mut doc = Document::open(hello_world()).unwrap();
        let (a, b) = {
            let page = doc.pages().get(1).unwrap();
            (
                TextBoxField::on_page(&page, rect(100.0, 600.0, 300.0, 620.0)),
                TextBoxField::on_page(&page, rect(100.0, 560.0, 300.0, 580.0)),
            )
        };
        doc.form_mut().add_named(Box::new(a), "keep".into(), 1);
        doc.form_mut().add_named(Box::new(b), "drop".into(), 1);
        doc.save_to(&out).unwrap();
    }
    {
        let mut doc = Document::open(&out).unwrap();
        assert_eq!(doc.form().count(), 2);
        doc.form_mut().delete_by_name("drop");
        assert_eq!(doc.form().count(), 1);
        doc.save_to(&out2).unwrap();
    }
    let doc = Document::open(&out2).unwrap();
    assert_eq!(doc.form().count(), 1);
    assert!(doc.form().has_field_by_name("keep"));
    assert!(!doc.form().has_field_by_name("drop"));
    let _ = std::fs::remove_file(&out);
    let _ = std::fs::remove_file(&out2);
}

#[test]
fn read_only_resave_is_lossless() {
    let out = common::unique_temp("asp_rust_form_loo_loss1.pdf");
    let out2 = common::unique_temp("asp_rust_form_loo_loss2.pdf");
    {
        let mut doc = Document::open(hello_world()).unwrap();
        let mut a = {
            let page = doc.pages().get(1).unwrap();
            TextBoxField::on_page(&page, rect(100.0, 600.0, 300.0, 620.0))
        };
        a.set_value("payload".into());
        doc.form_mut().add_named(Box::new(a), "only".into(), 1);
        doc.save_to(&out).unwrap();
    }
    {
        let doc = Document::open(&out).unwrap();
        assert_eq!(doc.form().count(), 1);
        doc.save_to(&out2).unwrap();
    }
    let doc = Document::open(&out2).unwrap();
    assert_eq!(doc.form().count(), 1);
    assert_eq!(doc.form().get_by_name("only").unwrap().value(), "payload");
    let _ = std::fs::remove_file(&out);
    let _ = std::fs::remove_file(&out2);
}

// ---------------------------------------------------------------------------
// forms_acroform_save_through_smoke_test
// ---------------------------------------------------------------------------

#[test]
fn no_form_bytes_untouched() {
    let src = std::fs::read(hello_world()).unwrap();
    let out = common::unique_temp("asp_rust_acroform_passthrough.pdf");
    let doc = Document::open(hello_world()).unwrap();
    let _ = doc.form().count();
    doc.save_to(&out).unwrap();
    assert_eq!(std::fs::read(&out).unwrap(), src);
    let _ = std::fs::remove_file(&out);
}

#[test]
fn text_box_field_emits_acroform_dict() {
    let out = common::unique_temp("asp_rust_acroform_textbox.pdf");
    {
        let mut doc = Document::open(hello_world()).unwrap();
        let mut name = {
            let page = doc.pages().get(1).unwrap();
            TextBoxField::on_page(&page, rect(100.0, 600.0, 300.0, 620.0))
        };
        name.set_partial_name("FullName".into());
        name.set_value("Jane Doe".into());
        doc.form_mut().add(Box::new(name));
        doc.save_to(&out).unwrap();
    }
    let saved = std::fs::read(&out).unwrap();
    for marker in [
        &b"/AcroForm"[..],
        b"/Widget",
        b"/FT",
        b"/Tx",
        b"FullName",
        b"Jane Doe",
    ] {
        assert!(
            contains(&saved, marker),
            "expected {:?} in the saved PDF",
            std::str::from_utf8(marker).unwrap()
        );
    }

    // And it round-trips back through the form.
    let doc = Document::open(&out).unwrap();
    assert_eq!(doc.form().count(), 1);
    assert_eq!(doc.form().get_by_name("FullName").unwrap().value(), "Jane Doe");
    let _ = std::fs::remove_file(&out);
}

#[test]
fn field_subtype_dispatched_ft() {
    let out = common::unique_temp("asp_rust_acroform_subtypes.pdf");
    {
        let mut doc = Document::open(hello_world()).unwrap();
        let r = rect(0.0, 0.0, 10.0, 10.0);
        let page_no = 1;

        let mut pwd = PasswordBoxField::new(&doc);
        pwd.set_partial_name("pwd".into());
        let mut cb = CheckboxField::new(&doc);
        cb.set_partial_name("subscribe".into());
        let mut rb = RadioButtonField::new(&doc);
        rb.set_partial_name("color".into());
        let mut combo = ComboBoxField::new(&doc);
        combo.set_partial_name("country".into());
        let mut lb = ListBoxField::for_document(&doc, r);
        lb.set_partial_name("languages".into());
        let mut sf = {
            let page = doc.pages().get(1).unwrap();
            SignatureField::on_page(&page, r)
        };
        sf.set_partial_name("sig".into());

        doc.form_mut().add_at(Box::new(pwd), page_no);
        doc.form_mut().add_at(Box::new(cb), page_no);
        doc.form_mut().add_at(Box::new(rb), page_no);
        doc.form_mut().add_at(Box::new(combo), page_no);
        doc.form_mut().add_at(Box::new(lb), page_no);
        doc.form_mut().add_at(Box::new(sf), page_no);
        doc.save_to(&out).unwrap();
    }
    let saved = std::fs::read(&out).unwrap();
    // Each field type writes its discriminating /FT into the PDF.
    for marker in [
        &b"/Tx"[..], // pwd
        b"/Btn",     // subscribe / color
        b"/Ch",      // country / languages
        b"/Sig",     // sig
    ] {
        assert!(
            contains(&saved, marker),
            "expected /FT {:?} in the saved PDF",
            std::str::from_utf8(marker).unwrap()
        );
    }
    // All six round-trip back into the form.
    let doc = Document::open(&out).unwrap();
    assert_eq!(doc.form().count(), 6);
    for name in ["pwd", "subscribe", "color", "country", "languages", "sig"] {
        assert!(doc.form().has_field_by_name(name), "missing field {name}");
    }
    let _ = std::fs::remove_file(&out);
}

/// A choice field's options are its content: without /Opt a drop-down
/// or list box reaches the file with nothing to choose from. The
/// options, the selected indices and the list box's first visible row
/// were all stored and never written.
#[test]
fn a_choice_field_carries_its_options() {
    use aspose_pdf_foss::ChoiceField;

    let out = common::unique_temp("aspose_rust_choice_opt.pdf");
    {
        let mut doc = Document::open(hello_world()).unwrap();
        let mut combo = ComboBoxField::new(&doc);
        combo.set_partial_name("country".into());
        combo.add_option("Ukraine".into());
        // An option whose shown name differs from its export value.
        combo.add_option_named("pl".into(), "Poland".into());

        let mut lb = ListBoxField::for_document(&doc, rect(0.0, 0.0, 80.0, 40.0));
        lb.set_partial_name("languages".into());
        lb.add_option("rust".into());
        lb.add_option("c".into());
        lb.add_option("go".into());
        lb.set_selected_items(vec![2, 0]);
        lb.set_top_index(1);

        doc.form_mut().add_at(Box::new(combo), 1);
        doc.form_mut().add_at(Box::new(lb), 1);
        doc.save_to(&out).unwrap();
    }
    let saved = std::fs::read(&out).unwrap();
    assert!(contains(&saved, b"/Opt"), "the option list");
    assert!(contains(&saved, b"(Ukraine)"), "a plain option is one string");
    assert!(
        contains(&saved, b"[ (pl) (Poland) ]"),
        "a named option is the export value and the shown name"
    );
    // §12.7.4.4: the selected indices are written ascending.
    assert!(contains(&saved, b"/I [ 0 2 ]"), "the selected indices, sorted");
    assert!(contains(&saved, b"/TI 1"), "the first visible option");
    let _ = std::fs::remove_file(&out);
}

/// A radio group whose selection cannot be cleared says so with
/// NoToggleToOff (§12.7.4 Table 229 bit 15); the flag was stored and
/// dropped, so the group stayed clearable.
#[test]
fn a_radio_group_carries_no_toggle_to_off() {
    let out = common::unique_temp("aspose_rust_radio_ntto.pdf");
    {
        let mut doc = Document::open(hello_world()).unwrap();
        let mut rb = RadioButtonField::new(&doc);
        rb.set_partial_name("color".into());
        rb.set_no_toggle_to_off(true);
        doc.form_mut().add_at(Box::new(rb), 1);
        doc.save_to(&out).unwrap();
    }
    let saved = std::fs::read(&out).unwrap();
    // Radio (1 << 15) | NoToggleToOff (1 << 14) = 49152.
    assert!(contains(&saved, b"/Ff 49152"), "both button bits");
    let _ = std::fs::remove_file(&out);
}

/// /TU is the name a viewer shows the user and /TM the name the field
/// exports under (§12.7.4.1 Table 220). Both were accepted, stored and
/// dropped, and a field loaded from a document lost them — along with
/// every /Ff bit this crate's model does not name.
#[test]
fn a_field_carries_and_keeps_its_names_and_flags() {
    let out = common::unique_temp("aspose_rust_field_names.pdf");
    {
        let mut doc = Document::open(hello_world()).unwrap();
        let mut tf = TextBoxField::new(&doc);
        tf.set_partial_name("surname".into());
        tf.set_alternate_name("Family name".into());
        tf.set_mapping_name("last_name".into());
        tf.set_multiline(true);
        doc.form_mut().add_at(Box::new(tf), 1);
        doc.save_to(&out).unwrap();
    }
    let saved = std::fs::read(&out).unwrap();
    assert!(contains(&saved, b"/TU (Family name)"), "the user-facing name");
    assert!(contains(&saved, b"/TM (last_name)"), "the export name");
    assert!(contains(&saved, b"/Ff 4096"), "Multiline is bit 13");

    // Reopen and write the form again: the names and the multiline bit
    // survive the round trip rather than being cleared by the rewrite.
    let again = common::unique_temp("aspose_rust_field_names_2.pdf");
    {
        let mut doc = Document::open(&out).unwrap();
        let f = doc.form().get_by_name("surname").unwrap();
        assert_eq!(f.alternate_name(), "Family name");
        assert_eq!(f.mapping_name(), "last_name");
        let mut extra = TextBoxField::new(&doc);
        extra.set_partial_name("extra".into());
        doc.form_mut().add_at(Box::new(extra), 1);
        doc.save_to(&again).unwrap();
    }
    let resaved = std::fs::read(&again).unwrap();
    assert!(contains(&resaved, b"/TU (Family name)"), "kept on rewrite");
    assert!(contains(&resaved, b"/TM (last_name)"), "kept on rewrite");
    assert!(contains(&resaved, b"/Ff 4096"), "Multiline kept on rewrite");

    let _ = std::fs::remove_file(&out);
    let _ = std::fs::remove_file(&again);
}

/// A field IS a widget (§12.7.3.1 merges the two objects), so the
/// widget's own entries belong on it: the action it runs when
/// activated, the highlighting a viewer uses, and the /DA its text is
/// set with. None reached the file, so a push button that runs
/// JavaScript — the whole point of one — did nothing. NoExport was
/// dropped the same way.
#[test]
fn a_field_carries_its_widget_entries() {
    use aspose_pdf_foss::{ButtonField, DefaultAppearance, HighlightingMode, JavascriptAction};

    let out = common::unique_temp("aspose_rust_widget_entries.pdf");
    {
        let mut doc = Document::open(hello_world()).unwrap();
        let mut b = ButtonField::for_document(&doc, rect(0.0, 0.0, 60.0, 20.0));
        b.set_partial_name("run".into());
        b.set_on_activated(Box::new(JavascriptAction::new("app.alert('hi')".into())));
        b.set_highlighting(HighlightingMode::Push);
        let mut da = DefaultAppearance::new();
        da.set_font_name("Helv".into());
        da.set_font_size(9.0);
        b.set_default_appearance(da);
        b.set_exportable(false);
        doc.form_mut().add_at(Box::new(b), 1);
        doc.save_to(&out).unwrap();
    }
    let saved = std::fs::read(&out).unwrap();
    assert!(contains(&saved, b"/S /JavaScript"), "the action it runs");
    assert!(contains(&saved, b"app.alert"), "with its script");
    assert!(contains(&saved, b"/H /P"), "the highlighting mode");
    assert!(contains(&saved, b"/DA (/Helv 9 Tf 0 g)"), "the default appearance");
    // Pushbutton (1 << 16) | NoExport (1 << 2) = 65540.
    assert!(contains(&saved, b"/Ff 65540"), "NoExport joins the type's bits");

    // NoExport survives a reopen and a rewrite rather than being lost.
    let doc = Document::open(&out).unwrap();
    assert!(!doc.form().get_by_name("run").unwrap().exportable());
    let _ = std::fs::remove_file(&out);
}

/// An exportable field with no action writes neither /A nor /Ff. /H is
/// the exception: this carrier defaults to "does not highlight" where
/// the format defaults to invert, so /N is written rather than letting
/// the file disagree with the getter.
#[test]
fn a_plain_field_writes_no_widget_entries() {
    let out = common::unique_temp("aspose_rust_widget_plain.pdf");
    {
        let mut doc = Document::open(hello_world()).unwrap();
        let mut tf = TextBoxField::new(&doc);
        tf.set_partial_name("plain".into());
        doc.form_mut().add_at(Box::new(tf), 1);
        doc.save_to(&out).unwrap();
    }
    let saved = std::fs::read(&out).unwrap();
    assert!(contains(&saved, b"/Tx"), "the field is written");
    assert!(contains(&saved, b"/H /N"), "the carrier's own default, stated");
    assert!(!contains(&saved, b"/S /"), "no action");
    assert!(!contains(&saved, b"/Ff"), "and no flags at all");
    let _ = std::fs::remove_file(&out);
}

#[path = "../common/mod.rs"]
mod common;

// ---- inherited TextBoxField surface --------------------------------
//
// Canonically BarcodeField, NumberField, DateField, FileSelectBoxField,
// PasswordBoxField and RichTextBoxField all derive from TextBoxField,
// and every peer track ships that base surface on them through
// inheritance. Rust has none, and the nine inherited members had simply
// vanished from all six — `number.set_max_len(..)` did not exist.
//
// They now embed a TextBoxField and Deref to it. The base must be the
// same object the field's own Field / Annotation surface writes to, or
// the two halves would drift apart.

#[test]
fn text_box_derived_fields_carry_the_inherited_surface() {
    let doc = Document::new();
    let rect = Rectangle::new(0.0, 0.0, 120.0, 20.0, false);

    macro_rules! check {
        ($field:expr) => {{
            let mut f = $field;
            f.set_max_len(24);
            f.set_multiline(true);
            f.set_scrollable(false);
            f.set_spell_check(true);
            f.set_force_combs(true);
            f.set_text_vertical_alignment(VerticalAlignment::Center);
            assert_eq!(f.max_len(), 24);
            assert!(f.multiline());
            assert!(!f.scrollable());
            assert!(f.spell_check());
            assert!(f.force_combs());
            assert_eq!(f.text_vertical_alignment(), VerticalAlignment::Center);

            // The inherited base is the same object the field's own
            // Field / Annotation surface writes to, not a second copy.
            f.set_partial_name("name".into());
            assert_eq!(f.partial_name(), "name");
            assert_eq!(f.annotation_type(), AnnotationType::Widget);
        }};
    }

    check!(aspose_pdf_foss::BarcodeField::for_document(&doc, rect));
    check!(aspose_pdf_foss::NumberField::for_document(&doc, rect));
    check!(aspose_pdf_foss::DateField::new(&doc));
    check!(aspose_pdf_foss::FileSelectBoxField::new(&doc));
    check!(aspose_pdf_foss::PasswordBoxField::new(&doc));
    check!(aspose_pdf_foss::RichTextBoxField::on_page(
        &doc.pages().add(),
        rect
    ));
}

// Value is a TextBoxField member, and it reaches the derived fields that
// do not redefine it. DateField is the exception canonically — its own
// Value is a DateTime — so it is checked on its own terms below.
#[test]
fn a_derived_field_inherits_the_value_member() {
    let doc = Document::new();
    let rect = Rectangle::new(0.0, 0.0, 120.0, 20.0, false);

    let mut number = aspose_pdf_foss::NumberField::for_document(&doc, rect);
    number.set_value("42".into());
    assert_eq!(number.value(), "42");

    let mut password = aspose_pdf_foss::PasswordBoxField::new(&doc);
    password.set_value("secret".into());
    assert_eq!(password.value(), "secret");
}

// The derived field's own surface still works, and is not shadowed by
// the base it now embeds.
#[test]
fn a_derived_field_keeps_its_own_surface() {
    let doc = Document::new();
    let barcode = aspose_pdf_foss::BarcodeField::for_document(
        &doc,
        Rectangle::new(0.0, 0.0, 120.0, 20.0, false),
    );
    assert_eq!(barcode.symbology(), aspose_pdf_foss::Symbology::PDF417);
    assert_eq!(barcode.ecc(), 0);

    let mut date = aspose_pdf_foss::DateField::new(&doc);
    date.set_date_format("yyyy-MM-dd".into());
    assert_eq!(date.date_format(), "yyyy-MM-dd");
}

/// A checked box has to reach the file. `CheckboxField::checked` used
/// to set a flag nothing read, so every checkbox was written unchecked
/// however the caller left it.
///
/// PDF names a button field's state rather than spelling it as a
/// string (§12.7.4.2.3), so the assertion looks for `/V /Yes` — a
/// `(Yes)` string there reads as unchecked in a viewer.
#[test]
fn a_checked_box_is_written_as_its_on_state() {
    fn saved(check: bool, export: Option<&str>) -> String {
        let mut doc = Document::open(hello_world()).unwrap();
        let mut cb = CheckboxField::new(&doc);
        cb.set_partial_name("agree".into());
        if let Some(e) = export {
            cb.set_export_value(e.to_string());
        }
        cb.set_checked(check);
        doc.form_mut().add_at(Box::new(cb), 1);
        let mut out = Vec::new();
        doc.save_to_writer(&mut out).unwrap();
        String::from_utf8_lossy(&out).into_owned()
    }
    let on = saved(true, None);
    assert!(on.contains("/V /Yes"), "a checked box should carry /V /Yes");
    assert!(!on.contains("/V (Yes)"), "the state is a name, not a string");

    let off = saved(false, None);
    assert!(off.contains("/V /Off"), "an unchecked box should carry /V /Off");

    // An export value names the on state in its place.
    let custom = saved(true, Some("Accepted"));
    assert!(custom.contains("/V /Accepted"), "the export value names the on state");
}

/// The flag and the field value are one thing, not two: setting either
/// is visible through the other.
#[test]
fn checked_and_the_field_value_agree() {
    let doc = Document::open(hello_world()).unwrap();
    let mut cb = CheckboxField::new(&doc);
    assert!(!cb.checked(), "a fresh box is unchecked");
    cb.set_checked(true);
    assert_eq!(cb.value(), "Yes", "checking it names the on state");
    cb.set_value("Off".into());
    assert!(!cb.checked(), "setting the value off unchecks it");
    cb.set_value("Yes".into());
    assert!(cb.checked(), "and setting an on state checks it");
}

/// `/Ff` carried only ReadOnly and Required, so every behavioural flag
/// a field type defines was lost: a multiline box was written
/// single-line, an editable combo non-editable, and a radio group as a
/// checkbox — which makes its buttons stop being exclusive.
///
/// Bit numbering in §12.7.4 is 1-based, so bit 13 is `1 << 12`. The
/// assertions name the resulting integer, because that is what a
/// viewer reads.
#[test]
fn field_flags_reach_the_file() {
    fn saved(build: impl FnOnce(&mut Document)) -> String {
        let mut doc = Document::open(hello_world()).unwrap();
        build(&mut doc);
        let mut out = Vec::new();
        doc.save_to_writer(&mut out).unwrap();
        String::from_utf8_lossy(&out).into_owned()
    }

    // Multiline is bit 13 → 4096.
    let multiline = saved(|doc| {
        let mut tb = TextBoxField::new(doc);
        tb.set_partial_name("notes".into());
        tb.set_multiline(true);
        doc.form_mut().add_at(Box::new(tb), 1);
    });
    assert!(multiline.contains("/Ff 4096"), "multiline, got: {}",
            flag_of(&multiline));

    // A radio group is bit 16 → 32768.
    let radio = saved(|doc| {
        let mut rb = RadioButtonField::new(doc);
        rb.set_partial_name("colour".into());
        doc.form_mut().add_at(Box::new(rb), 1);
    });
    assert!(radio.contains("/Ff 32768"), "radio, got: {}", flag_of(&radio));

    // A password box is bit 14 → 8192.
    let password = saved(|doc| {
        let mut pwd = PasswordBoxField::new(doc);
        pwd.set_partial_name("pin".into());
        doc.form_mut().add_at(Box::new(pwd), 1);
    });
    assert!(password.contains("/Ff 8192"), "password, got: {}",
            flag_of(&password));

    // An editable combo is Combo 18 | Edit 19 → 131072 + 262144.
    let combo = saved(|doc| {
        let mut cb = ComboBoxField::new(doc);
        cb.set_partial_name("country".into());
        cb.set_editable(true);
        doc.form_mut().add_at(Box::new(cb), 1);
    });
    assert!(combo.contains("/Ff 393216"), "editable combo, got: {}",
            flag_of(&combo));
}

/// Whatever `/Ff` the saved document carries, for a readable failure.
fn flag_of(pdf: &str) -> String {
    pdf.find("/Ff ")
        .map(|i| pdf[i..].chars().take(12).collect())
        .unwrap_or_else(|| "no /Ff at all".to_string())
}

/// A text field's length limit is an instruction to the viewer, and a
/// comb field is meaningless without it.
#[test]
fn a_length_limit_reaches_the_file() {
    let doc = Document::open(hello_world()).unwrap();
    let mut tb = TextBoxField::new(&doc);
    tb.set_partial_name("zip".into());
    tb.set_max_len(5);
    let mut d = doc;
    d.form_mut().add_at(Box::new(tb), 1);
    let mut out = Vec::new();
    d.save_to_writer(&mut out).unwrap();
    assert!(String::from_utf8_lossy(&out).contains("/MaxLen 5"));
}

/// A flattened form stops being a form: §12.7.2 makes the catalog's
/// /AcroForm the document's form, so a structural flatten removes that
/// entry rather than writing a form with no fields in it. The values
/// are not burned into the page content — that needs an appearance
/// generator this crate does not have, which Form::flatten says.
#[test]
fn a_flattened_form_leaves_no_acroform_behind() {
    let out = common::unique_temp("aspose_rust_flatten_form.pdf");
    {
        let mut doc = Document::open(hello_world()).unwrap();
        let mut tf = TextBoxField::new(&doc);
        tf.set_partial_name("name".into());
        tf.set_value("Ada".into());
        doc.form_mut().add_at(Box::new(tf), 1);
        doc.save_to(&out).unwrap();
    }
    // With the field, there is a form.
    let with_form = String::from_utf8_lossy(&std::fs::read(&out).unwrap()).into_owned();
    assert!(with_form.contains("/AcroForm"), "the form is written");
    assert!(with_form.contains("(name)"), "with its field");

    let flattened = common::unique_temp("aspose_rust_flatten_form_done.pdf");
    {
        let mut doc = Document::open(&out).unwrap();
        assert_eq!(doc.form().count(), 1, "the field is read back");
        doc.form_mut().flatten();
        assert_eq!(doc.form().count(), 0, "and dropped by the flatten");
        doc.save_to(&flattened).unwrap();
    }
    let after = Document::open(&flattened).unwrap();
    assert_eq!(after.form().count(), 0, "the reopened document has no fields");
    let bytes = std::fs::read(&flattened).unwrap();
    // The catalog's last word on /AcroForm is its removal: the
    // incremental update rewrites the catalog without the key.
    let text = String::from_utf8_lossy(&bytes);
    let last_catalog = text.rfind("/Type /Catalog").expect("a catalog");
    assert!(
        !text[last_catalog..].contains("/AcroForm"),
        "the newest catalog carries no /AcroForm"
    );

    let _ = std::fs::remove_file(&out);
    let _ = std::fs::remove_file(&flattened);
}

/// §12.7.4.2 Table 228 bit 23 and Table 230 bit 23 are the same flag,
/// DoNotSpellCheck, and both families stored `SpellCheck` without ever
/// writing it. The default had drifted the other way too: a fresh field
/// read as `spell_check() == false` while the format's own default — no
/// such bit in `/Ff` — means the field IS checked, which is how
/// `scrollable` right beside it already read its default.
#[test]
fn a_field_that_declines_spell_checking_says_so_in_ff() {
    let doc = Document::open(hello_world()).unwrap();
    assert!(
        TextBoxField::new(&doc).spell_check(),
        "the format's default: no DoNotSpellCheck bit means checked"
    );
    assert!(ComboBoxField::new(&doc).spell_check(), "and the same for a combo box");

    let mut doc = doc;
    let mut tf = TextBoxField::new(&doc);
    tf.set_partial_name("notes".into());
    tf.set_spell_check(false);
    doc.form_mut().add_at(Box::new(tf), 1);
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();
    let text = String::from_utf8_lossy(&out);
    // DoNotSpellCheck alone: 1 << 22.
    assert!(text.contains("/Ff 4194304"), "the text box declines spell-checking");
}

/// Table 230: the flag "shall not be used unless the Combo and Edit
/// flags are both set". Combo always is here, so an uneditable combo
/// box must write nothing — there is no typed text to check.
#[test]
fn an_uneditable_combo_box_writes_no_spell_check_flag() {
    let doc = Document::open(hello_world()).unwrap();
    let mut doc = doc;
    let mut editable = ComboBoxField::new(&doc);
    editable.set_partial_name("city".into());
    editable.set_editable(true);
    editable.set_spell_check(false);
    let mut fixed = ComboBoxField::new(&doc);
    fixed.set_partial_name("country".into());
    fixed.set_spell_check(false);
    doc.form_mut().add_at(Box::new(editable), 1);
    doc.form_mut().add_at(Box::new(fixed), 1);
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();
    let text = String::from_utf8_lossy(&out);
    // Combo (1 << 17) | Edit (1 << 18) | DoNotSpellCheck (1 << 22).
    assert!(text.contains("/Ff 4587520"), "the editable one carries the flag");
    // Combo alone.
    assert!(text.contains("/Ff 131072"), "the fixed one carries only Combo");
}

/// `Selected` was stored and dropped while `SelectedItems` wrote `/I`,
/// so selecting one option recorded the choice nowhere. §12.7.4.4 wants
/// both halves: `/V` is what a viewer displays and `/I` the index it
/// was chosen by, which is why selecting also sets the field value.
#[test]
fn selecting_one_option_reaches_both_entries() {
    let doc = Document::open(hello_world()).unwrap();
    let mut doc = doc;
    let mut combo = ComboBoxField::new(&doc);
    combo.set_partial_name("country".into());
    combo.add_option("Ukraine".into());
    combo.add_option_named("pl".into(), "Poland".into());
    combo.set_selected(1);
    assert_eq!(combo.selected(), 1);
    assert_eq!(combo.value(), "pl", "the export value, not the shown name");

    doc.form_mut().add_at(Box::new(combo), 1);
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();
    let text = String::from_utf8_lossy(&out);
    assert!(text.contains("/V (pl)"), "the value a viewer displays");
    assert!(text.contains("/I [ 1 ]"), "and the index it was chosen by");
}

/// An index outside the option list names no option, so it sets no
/// value and writes no `/I` — the alternative is a form that claims a
/// selection a viewer cannot resolve.
#[test]
fn an_out_of_range_selection_writes_nothing() {
    let doc = Document::open(hello_world()).unwrap();
    let mut doc = doc;
    let mut combo = ComboBoxField::new(&doc);
    combo.set_partial_name("country".into());
    combo.add_option("Ukraine".into());
    combo.set_selected(7);
    assert_eq!(combo.selected(), 7, "the property still reads back");
    assert_eq!(combo.value(), "", "but no option was named");

    doc.form_mut().add_at(Box::new(combo), 1);
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();
    let text = String::from_utf8_lossy(&out);
    assert!(text.contains("(Ukraine)"), "the option list is written");
    assert!(!text.contains("/I ["), "and no index array");
}
