// Public-API smoke driver — TextEditOptions, its four nested enums,
// and the TextFragmentAbsorber editing constructors.

use aspose_pdf_foss::{
    ClippingPathsProcessingMode, Document, FontReplace, LanguageTransformation, NoCharacterAction,
    TextEditOptions, TextFragmentAbsorber, TextSearchOptions,
};
use std::path::PathBuf;

fn hello_world_pdf() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/text_extractor/hello_world.pdf")
}

#[test]
fn nested_enum_ordinals_match_canonical() {
    assert_eq!(NoCharacterAction::ThrowException as i32, 0);
    assert_eq!(NoCharacterAction::UseStandardFont as i32, 1);
    assert_eq!(NoCharacterAction::ReplaceAnyway as i32, 2);
    assert_eq!(NoCharacterAction::ReplaceFonts as i32, 3);
    assert_eq!(NoCharacterAction::UseCustomReplacementFont as i32, 4);

    assert_eq!(FontReplace::Default as i32, 0);
    assert_eq!(FontReplace::RemoveUnusedFonts as i32, 1);

    assert_eq!(LanguageTransformation::Default as i32, 0);
    assert_eq!(LanguageTransformation::ExactlyAsISee as i32, 1);
    assert_eq!(LanguageTransformation::None as i32, 2);

    assert_eq!(ClippingPathsProcessingMode::KeepIntact as i32, 0);
    assert_eq!(ClippingPathsProcessingMode::Expand as i32, 1);
    assert_eq!(ClippingPathsProcessingMode::Remove as i32, 2);

    // The zero variant is the default.
    assert_eq!(NoCharacterAction::default(), NoCharacterAction::ThrowException);
    assert_eq!(ClippingPathsProcessingMode::default(), ClippingPathsProcessingMode::KeepIntact);
}

#[test]
fn options_constructors_and_round_trip() {
    let a = TextEditOptions::with_no_character_behavior(NoCharacterAction::ReplaceFonts);
    assert_eq!(a.no_character_behavior(), NoCharacterAction::ReplaceFonts);

    let b = TextEditOptions::with_font_replace_behavior(FontReplace::RemoveUnusedFonts);
    assert_eq!(b.font_replace_behavior(), FontReplace::RemoveUnusedFonts);

    let c = TextEditOptions::with_allow_language_transformation(true);
    assert!(c.allow_language_transformation());

    let d = TextEditOptions::with_language_transformation_behavior(LanguageTransformation::None);
    assert_eq!(d.language_transformation_behavior(), LanguageTransformation::None);

    let mut o = TextEditOptions::default();
    assert_eq!(o.no_character_behavior(), NoCharacterAction::ThrowException);
    assert!(o.replacement_font().is_none());
    o.set_clipping_paths_processing(ClippingPathsProcessingMode::Remove);
    o.set_to_attempt_get_underline_from_source(true);
    assert_eq!(o.clipping_paths_processing(), ClippingPathsProcessingMode::Remove);
    assert!(o.to_attempt_get_underline_from_source());
}

#[test]
fn absorber_editing_constructors_still_search() {
    let doc = Document::open(hello_world_pdf()).unwrap();
    let opts = TextEditOptions::with_no_character_behavior(NoCharacterAction::ReplaceAnyway);

    // Match-all edit mode collects fragments.
    let mut all = TextFragmentAbsorber::with_edit_options(opts.clone());
    all.visit_document(&doc).unwrap();
    assert!(all.text_fragments().count() >= 1);

    // Phrase + edit options.
    let mut phrase = TextFragmentAbsorber::with_phrase_and_edit_options("World", opts.clone());
    phrase.visit_document(&doc).unwrap();
    assert_eq!(phrase.text_fragments().count(), 1);

    // Phrase + search options + edit options.
    let search = TextSearchOptions::new(false);
    let mut both =
        TextFragmentAbsorber::with_phrase_search_and_edit_options("World", search, opts);
    both.visit_document(&doc).unwrap();
    assert_eq!(both.text_fragments().count(), 1);
}
