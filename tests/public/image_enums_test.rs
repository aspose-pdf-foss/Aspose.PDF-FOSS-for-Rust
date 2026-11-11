// Public-API smoke driver — the image-cluster enums and the members
// they un-block: XImageCollection filtered Add / Delete-with-action
// and PdfExtractor's ExtractImageMode.

use aspose_pdf_foss::{
    BitmapInfo, ExtractImageMode, ImageDeleteAction, ImageFilterType, PdfExtractor, PixelFormat,
    XImageCollection,
};

#[test]
fn enum_ordinals_match_canonical() {
    assert_eq!(ImageFilterType::Jpeg2000 as i32, 0);
    assert_eq!(ImageFilterType::Jpeg as i32, 1);
    assert_eq!(ImageFilterType::Flate as i32, 2);
    assert_eq!(ImageFilterType::CCITTFax as i32, 3);

    assert_eq!(ImageDeleteAction::KeepContents as i32, 0);
    assert_eq!(ImageDeleteAction::None as i32, 1);
    assert_eq!(ImageDeleteAction::ForceDelete as i32, 2);
    assert_eq!(ImageDeleteAction::Check as i32, 3);

    assert_eq!(ExtractImageMode::DefinedInResources as i32, 0);
    assert_eq!(ExtractImageMode::ActuallyUsed as i32, 1);

    assert_eq!(ImageFilterType::default(), ImageFilterType::Jpeg2000);
    assert_eq!(ImageDeleteAction::default(), ImageDeleteAction::KeepContents);
    assert_eq!(ExtractImageMode::default(), ExtractImageMode::DefinedInResources);
}

#[test]
fn ximage_filtered_add_and_delete_with_action() {
    let col = XImageCollection::new();
    let bmp = BitmapInfo::new(vec![0u8; 3 * 2 * 2], 2, 2, PixelFormat::Rgb24);

    let name = col.add_bitmap_filtered(&bmp, ImageFilterType::Flate);
    assert_eq!(col.count(), 1);

    col.delete_by_name_with_action(&name, ImageDeleteAction::ForceDelete);
    assert_eq!(col.count(), 0);

    col.add_bitmap_filtered(&bmp, ImageFilterType::Jpeg);
    assert_eq!(col.count(), 1);
    col.delete_at_with_action(0, ImageDeleteAction::KeepContents);
    assert_eq!(col.count(), 0);
}

#[test]
fn extractor_exposes_image_mode() {
    assert_eq!(
        PdfExtractor::new().extract_image_mode(),
        ExtractImageMode::DefinedInResources
    );
}
