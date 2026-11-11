// Public-API smoke driver — Measure number-format lists.
//
// NumberFormat value entries, the NumberFormatList collection, and
// the six per-axis format properties on Measure. FractionDisplayment
// and the Measure-taking constructors are dropped.

use aspose_pdf_foss::{Measure, NumberFormat, NumberFormatList};

#[test]
fn number_format_value_accessors() {
    let mut nf = NumberFormat::default();
    assert_eq!(nf.unit_label(), "");
    assert_eq!(nf.convresion_factor(), 0.0);
    assert!(!nf.force_denominator());

    nf.set_unit_label("mi".into());
    nf.set_convresion_factor(0.5);
    nf.set_precision(2);
    nf.set_before_text("~".into());
    assert_eq!(nf.unit_label(), "mi");
    assert_eq!(nf.convresion_factor(), 0.5);
    assert_eq!(nf.precision(), 2);
    assert_eq!(nf.before_text(), "~");
}

#[test]
fn number_format_list_collection() {
    let mut list = NumberFormatList::default();
    assert_eq!(list.count(), 0);
    assert!(list.at(0).is_none());

    let mut mi = NumberFormat::default();
    mi.set_unit_label("mi".into());
    list.add(mi);
    assert_eq!(list.count(), 1);
    assert_eq!(list.at(0).unwrap().unit_label(), "mi");

    let mut km = NumberFormat::default();
    km.set_unit_label("km".into());
    list.insert(0, km);
    assert_eq!(list.count(), 2);
    assert_eq!(list.at(0).unwrap().unit_label(), "km");

    list.remove_at(0);
    assert_eq!(list.count(), 1);
    assert_eq!(list.at(0).unwrap().unit_label(), "mi");
}

#[test]
fn measure_format_lists_round_trip() {
    let mut m = Measure::default();
    assert_eq!(m.distance_format().count(), 0);

    let mut list = NumberFormatList::default();
    let mut nf = NumberFormat::default();
    nf.set_unit_label("mi".into());
    list.add(nf);
    m.set_distance_format(list);

    assert_eq!(m.distance_format().count(), 1);
    assert_eq!(m.distance_format().at(0).unwrap().unit_label(), "mi");
}

#[test]
fn number_format_fraction_displayment() {
    use aspose_pdf_foss::FractionStyle;
    let mut nf = NumberFormat::default();
    assert_eq!(nf.fraction_displayment(), FractionStyle::ShowAsDecimal); // ordinal 0
    nf.set_fraction_displayment(FractionStyle::Round);
    assert_eq!(nf.fraction_displayment(), FractionStyle::Round);
}
