// Public-API smoke driver — F-7a form value types.
//
// FlattenSettings + IconFit (+ converters) + XFA. The Form facade
// itself is a separate beat.

use aspose_pdf_foss::{FlattenSettings, IconFit, ScalingMode, ScalingReason, XFA};

#[test]
fn flatten_settings() {
    let mut f = FlattenSettings::new();
    assert!(!f.update_appearances());
    f.set_update_appearances(true);
    f.set_hide_buttons(true);
    assert!(f.update_appearances());
    assert!(f.hide_buttons());
    assert!(!f.call_events());
    assert!(!f.apply_redactions());
}

#[test]
fn icon_fit_props_and_converters() {
    let mut i = IconFit::new();
    assert_eq!(i.scaling_reason(), ScalingReason::Always);
    assert_eq!(i.scaling_mode(), ScalingMode::Proportional);
    i.set_scaling_mode(ScalingMode::Anamorphic);
    i.set_leftover_left(0.5);
    assert_eq!(i.scaling_mode(), ScalingMode::Anamorphic);
    assert_eq!(i.leftover_left(), 0.5);

    // static name converters round-trip
    assert_eq!(IconFit::scaling_reason_to_name(ScalingReason::Never), "N");
    assert_eq!(IconFit::name_to_scaling_reason("B"), ScalingReason::IconIsBigger);
    assert_eq!(IconFit::scaling_mode_to_name(ScalingMode::Anamorphic), "A");
    assert_eq!(IconFit::name_to_scaling_mode("P"), ScalingMode::Proportional);
}

#[test]
fn xfa_field_values() {
    let mut x = XFA::new();
    assert!(x.field_names().is_empty());
    assert_eq!(x.get_value("missing"), "");
    x.set_value("name", "Ada".into());
    x.set_value("city", "London".into());
    assert_eq!(x.get_value("name"), "Ada");
    assert_eq!(x.field_names(), vec!["city".to_string(), "name".to_string()]); // sorted
}
