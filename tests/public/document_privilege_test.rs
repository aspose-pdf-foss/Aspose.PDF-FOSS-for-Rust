// Public-API smoke driver — DocumentPrivilege + AFRelationship.
//
// Ports the cpp document_privilege_smoke_test: default == ForbidAll,
// AllowAll sets every flag + level, the per-operation presets are
// isolated, properties round-trip, and compare_to orders by how
// much each set grants. Also pins the AFRelationship ordinals.

use aspose_pdf_foss::{AFRelationship, DocumentPrivilege};

#[test]
fn af_relationship_ordinals() {
    assert_eq!(AFRelationship::Source as i32, 0);
    assert_eq!(AFRelationship::Data as i32, 1);
    assert_eq!(AFRelationship::Alternative as i32, 2);
    assert_eq!(AFRelationship::Supplement as i32, 3);
    assert_eq!(AFRelationship::Unspecified as i32, 4);
    assert_eq!(AFRelationship::EncryptedPayload as i32, 5);
    assert_eq!(AFRelationship::None as i32, 6);
    assert_eq!(AFRelationship::default(), AFRelationship::Unspecified);
}

#[test]
fn default_constructed_equals_forbid_all() {
    let d = DocumentPrivilege::new();
    assert!(!d.allow_print());
    assert!(!d.allow_degraded_printing());
    assert!(!d.allow_modify_contents());
    assert!(!d.allow_copy());
    assert!(!d.allow_modify_annotations());
    assert!(!d.allow_fill_in());
    assert!(!d.allow_screen_readers());
    assert!(!d.allow_assembly());
    assert_eq!(d.print_allow_level(), 0);
    assert_eq!(d.change_allow_level(), 0);
    assert_eq!(d.copy_allow_level(), 0);
    assert_eq!(d.compare_to(&DocumentPrivilege::forbid_all()), 0);
}

#[test]
fn allow_all_sets_every_flag() {
    let a = DocumentPrivilege::allow_all();
    assert!(a.allow_print());
    assert!(a.allow_degraded_printing());
    assert!(a.allow_modify_contents());
    assert!(a.allow_copy());
    assert!(a.allow_modify_annotations());
    assert!(a.allow_fill_in());
    assert!(a.allow_screen_readers());
    assert!(a.allow_assembly());
    assert_eq!(a.print_allow_level(), 2);
    assert_eq!(a.change_allow_level(), 2);
    assert_eq!(a.copy_allow_level(), 2);
}

#[test]
fn preset_isolation() {
    let p = DocumentPrivilege::print();
    assert!(p.allow_print());
    assert_eq!(p.print_allow_level(), 2);
    assert!(!p.allow_degraded_printing());
    assert!(!p.allow_modify_contents());
    assert!(!p.allow_copy());

    let dp = DocumentPrivilege::degraded_printing();
    assert!(dp.allow_degraded_printing());
    assert_eq!(dp.print_allow_level(), 1);
    assert!(!dp.allow_print());

    let c = DocumentPrivilege::copy();
    assert!(c.allow_copy());
    assert_eq!(c.copy_allow_level(), 2);
    assert!(!c.allow_print());

    let mc = DocumentPrivilege::modify_contents();
    assert!(mc.allow_modify_contents());
    assert_eq!(mc.change_allow_level(), 2);

    let ma = DocumentPrivilege::modify_annotations();
    assert!(ma.allow_modify_annotations());
    assert_eq!(ma.change_allow_level(), 0); // no associated level

    assert!(DocumentPrivilege::fill_in().allow_fill_in());
    assert!(DocumentPrivilege::screen_readers().allow_screen_readers());
    assert!(DocumentPrivilege::assembly().allow_assembly());
}

#[test]
fn property_round_trip() {
    let mut d = DocumentPrivilege::new();
    d.set_allow_print(true);
    d.set_allow_fill_in(true);
    d.set_allow_assembly(true);
    d.set_print_allow_level(2);
    d.set_copy_allow_level(1);
    assert!(d.allow_print());
    assert!(d.allow_fill_in());
    assert!(d.allow_assembly());
    assert!(!d.allow_copy());
    assert_eq!(d.print_allow_level(), 2);
    assert_eq!(d.copy_allow_level(), 1);
    assert_eq!(d.change_allow_level(), 0);
    d.set_allow_print(false);
    assert!(!d.allow_print());
}

#[test]
fn compare_to_orders() {
    let forbid = DocumentPrivilege::forbid_all();
    let allow = DocumentPrivilege::allow_all();
    let print = DocumentPrivilege::print();
    assert_eq!(forbid.compare_to(&forbid), 0);
    assert_eq!(allow.compare_to(&allow), 0);
    assert!(forbid.compare_to(&print) < 0);
    assert!(print.compare_to(&forbid) > 0);
    assert!(print.compare_to(&allow) < 0);
}
