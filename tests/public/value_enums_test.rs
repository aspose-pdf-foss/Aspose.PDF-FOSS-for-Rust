// Public-API smoke driver — core value enums.
//
// Covers the leaf enums shipped in the G0 value-types cluster plus
// the layout / stamp / metadata enums that ship as reference surface
// with no in-subset consumer. Each assertion pins the canonical
// Aspose.PDF 26.4.0 ordinal so a future drift surfaces here — for the
// consumer-less enums this file is the only thing holding them.

use aspose_pdf_foss::{
    AutoRotateMode, BlendingColorSpace, CryptoAlgorithm, DefaultMetadataProperties, FileEncoding,
    HorizontalAlignment, PositioningMode, Rotation, StampType, VerticalAlignment, WordWrapMode,
};

#[test]
fn rotation_ordinals() {
    assert_eq!(Rotation::None as i32, 0);
    assert_eq!(Rotation::On90 as i32, 1);
    assert_eq!(Rotation::On180 as i32, 2);
    assert_eq!(Rotation::On270 as i32, 3);
    assert_eq!(Rotation::default(), Rotation::None);
}

#[test]
fn horizontal_alignment_ordinals() {
    assert_eq!(HorizontalAlignment::None as i32, 0);
    assert_eq!(HorizontalAlignment::Left as i32, 1);
    assert_eq!(HorizontalAlignment::Center as i32, 2);
    assert_eq!(HorizontalAlignment::Right as i32, 3);
    assert_eq!(HorizontalAlignment::Justify as i32, 4);
    assert_eq!(HorizontalAlignment::FullJustify as i32, 5);
    assert_eq!(HorizontalAlignment::default(), HorizontalAlignment::None);
}

#[test]
fn vertical_alignment_ordinals() {
    assert_eq!(VerticalAlignment::None as i32, 0);
    assert_eq!(VerticalAlignment::Top as i32, 1);
    assert_eq!(VerticalAlignment::Center as i32, 2);
    assert_eq!(VerticalAlignment::Bottom as i32, 3);
    assert_eq!(VerticalAlignment::default(), VerticalAlignment::None);
}

#[test]
fn crypto_algorithm_ordinals() {
    assert_eq!(CryptoAlgorithm::Rc4x40 as i32, 0);
    assert_eq!(CryptoAlgorithm::Rc4x128 as i32, 1);
    assert_eq!(CryptoAlgorithm::Aesx128 as i32, 2);
    assert_eq!(CryptoAlgorithm::Aesx256 as i32, 3);
    assert_eq!(CryptoAlgorithm::default(), CryptoAlgorithm::Rc4x40);
}

#[test]
fn file_encoding_ordinals() {
    assert_eq!(FileEncoding::None as i32, 0);
    assert_eq!(FileEncoding::Zip as i32, 1);
    assert_eq!(FileEncoding::default(), FileEncoding::None);
}

#[test]
fn auto_rotate_mode_ordinals() {
    assert_eq!(AutoRotateMode::None as i32, 0);
    assert_eq!(AutoRotateMode::ClockWise as i32, 1);
    assert_eq!(AutoRotateMode::AntiClockWise as i32, 2);
    assert_eq!(AutoRotateMode::default(), AutoRotateMode::None);
}

#[test]
fn blending_color_space_ordinals() {
    assert_eq!(BlendingColorSpace::DontChange as i32, 0);
    assert_eq!(BlendingColorSpace::Auto as i32, 1);
    assert_eq!(BlendingColorSpace::DeviceRGB as i32, 2);
    assert_eq!(BlendingColorSpace::DeviceCMYK as i32, 3);
    assert_eq!(BlendingColorSpace::default(), BlendingColorSpace::DontChange);
}

#[test]
fn positioning_mode_ordinals() {
    assert_eq!(PositioningMode::Legacy as i32, 0);
    assert_eq!(PositioningMode::ModernLineSpacing as i32, 1);
    assert_eq!(PositioningMode::Current as i32, 2);
    assert_eq!(PositioningMode::default(), PositioningMode::Legacy);
}

#[test]
fn stamp_type_ordinals() {
    assert_eq!(StampType::Form as i32, 0);
    assert_eq!(StampType::Image as i32, 1);
    assert_eq!(StampType::default(), StampType::Form);
}

#[test]
fn word_wrap_mode_ordinals() {
    assert_eq!(WordWrapMode::Default as i32, 0);
    assert_eq!(WordWrapMode::ByWords as i32, 1);
    assert_eq!(WordWrapMode::default(), WordWrapMode::Default);
}

#[test]
fn default_metadata_properties_ordinals() {
    // Nine canonical members, contiguous from zero.
    assert_eq!(DefaultMetadataProperties::Advisory as i32, 0);
    assert_eq!(DefaultMetadataProperties::BaseURL as i32, 1);
    assert_eq!(DefaultMetadataProperties::CreateDate as i32, 2);
    assert_eq!(DefaultMetadataProperties::CreatorTool as i32, 3);
    assert_eq!(DefaultMetadataProperties::Identifier as i32, 4);
    assert_eq!(DefaultMetadataProperties::MetadataDate as i32, 5);
    assert_eq!(DefaultMetadataProperties::ModifyDate as i32, 6);
    assert_eq!(DefaultMetadataProperties::Nickname as i32, 7);
    assert_eq!(DefaultMetadataProperties::Thumbnails as i32, 8);
}
