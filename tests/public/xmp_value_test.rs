// Public-API smoke driver — XmpValue tagged-union variants and
// the conversion helpers (`to_*`) + variant probes (`is_*`).

use aspose_pdf_foss::{Error, XmpValue};

#[test]
fn string_value_round_trips() {
    let s = XmpValue::from_string("hello");
    assert!(s.is_string());
    assert!(!s.is_integer());
    assert_eq!(s.to_string_value(), "hello");
    assert_eq!(s.to_string(), "hello");
}

#[test]
fn integer_value_round_trips() {
    let i = XmpValue::from_integer(42);
    assert!(i.is_integer());
    assert_eq!(i.to_integer().unwrap(), 42);
    assert_eq!(i.to_double().unwrap(), 42.0);
    assert_eq!(i.to_string_value(), "42");
}

#[test]
fn double_value_round_trips() {
    let d = XmpValue::from_double(3.5);
    assert!(d.is_double());
    assert_eq!(d.to_double().unwrap(), 3.5);
    assert_eq!(d.to_integer().unwrap(), 3);
    assert_eq!(d.to_string_value(), "3.5");
}

#[test]
fn array_value_round_trips() {
    let arr = XmpValue::from_array(vec![
        XmpValue::from_string("Alice"),
        XmpValue::from_string("Bob"),
    ]);
    assert!(arr.is_array());
    let elems = arr.to_array().unwrap();
    assert_eq!(elems.len(), 2);
    assert_eq!(elems[0].to_string_value(), "Alice");
    assert_eq!(elems[1].to_string_value(), "Bob");
    // Array stringification joins items with a tab — matches
    // the foundation xmp canonical separator.
    assert_eq!(arr.to_string_value(), "Alice\tBob");
}

#[test]
fn string_parses_as_integer() {
    let from_string = XmpValue::from_string("123");
    assert_eq!(from_string.to_integer().unwrap(), 123);
}

#[test]
fn string_parses_as_double() {
    let from_string = XmpValue::from_string("2.5");
    assert_eq!(from_string.to_double().unwrap(), 2.5);
}

#[test]
fn string_non_numeric_returns_conversion_error() {
    let s = XmpValue::from_string("not-a-number");
    match s.to_integer() {
        Err(Error::XmpValueConversion(_)) => {}
        other => panic!("expected XmpValueConversion, got {other:?}"),
    }
}

#[test]
fn array_to_integer_rejects() {
    let arr = XmpValue::from_array(vec![XmpValue::from_string("x")]);
    match arr.to_integer() {
        Err(Error::XmpValueConversion(_)) => {}
        other => panic!("expected XmpValueConversion, got {other:?}"),
    }
}

#[test]
fn string_to_array_rejects() {
    let s = XmpValue::from_string("scalar");
    match s.to_array() {
        Err(Error::XmpValueConversion(_)) => {}
        other => panic!("expected XmpValueConversion, got {other:?}"),
    }
}

#[test]
fn v1_only_supports_string_integer_double_array() {
    let any = XmpValue::from_string("x");
    assert!(!any.is_date_time());
    assert!(!any.is_field());
    assert!(!any.is_named_value());
    assert!(!any.is_raw());
    assert!(!any.is_named_values());
    assert!(!any.is_structure());
}

#[test]
fn from_str_and_from_i32_conversions() {
    let s: XmpValue = "hello".into();
    assert!(s.is_string());
    let n: XmpValue = 7i32.into();
    assert!(n.is_integer());
    let d: XmpValue = 1.25f64.into();
    assert!(d.is_double());
}
