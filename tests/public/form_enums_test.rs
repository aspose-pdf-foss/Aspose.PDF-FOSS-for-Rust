// Public-API smoke driver — forms leaf enums (F-0a).
//
// Pins the canonical Aspose.PDF 26.4.0 ordinals for the forms enum
// family.

use aspose_pdf_foss::*;

#[test]
fn form_enum_ordinals() {
    assert_eq!(BoxStyle::Circle as i32, 0);
    assert_eq!(BoxStyle::Star as i32, 5);

    assert_eq!(DocMDPAccessPermissions::NoChanges as i32, 1); // starts at 1
    assert_eq!(DocMDPAccessPermissions::AnnotationModification as i32, 3);
    assert_eq!(
        DocMDPAccessPermissions::default(),
        DocMDPAccessPermissions::NoChanges
    );

    assert_eq!(FormType::Standard as i32, 0);
    assert_eq!(FormType::Dynamic as i32, 2);

    assert_eq!(IconCaptionPosition::NoIcon as i32, 0);
    assert_eq!(IconCaptionPosition::CaptionOverlaid as i32, 6);

    assert_eq!(ScalingMode::Proportional as i32, 0);
    assert_eq!(ScalingMode::Anamorphic as i32, 1);

    assert_eq!(ScalingReason::Always as i32, 0);
    assert_eq!(ScalingReason::Never as i32, 3);

    assert_eq!(SignDependentElementsRenderingModes::RenderFormAsUnsigned as i32, 0);
    assert_eq!(SignDependentElementsRenderingModes::RenderFormAsSigned as i32, 1);

    assert_eq!(SubjectNameElements::CN as i32, 0);
    assert_eq!(SubjectNameElements::E as i32, 6);

    assert_eq!(Symbology::PDF417 as i32, 0);
    assert_eq!(Symbology::DataMatrix as i32, 2);
}
