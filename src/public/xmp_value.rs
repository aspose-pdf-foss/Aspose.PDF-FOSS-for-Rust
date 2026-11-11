//! `Aspose.Pdf.XmpValue` — tagged-union value carried in a
//! [`Metadata`](crate::Metadata) map.
//!
//! v1 covers the four scalar shapes a PDF `/Metadata` packet
//! actually exercises: string, signed 32-bit integer,
//! double-precision float, and ordered arrays of [`XmpValue`].
//!
//! The canonical .NET surface advertises richer shapes
//! (`DateTime`, `XmpField`, raw XML node, named-value pairs,
//! structures); those are out of v1 scope. Their `is_*` predicates
//! are still surfaced and always return `false`, so callers
//! relying on the shape probe remain source-compatible.

use std::fmt;

use super::error::Error;

#[derive(Debug, Clone, PartialEq)]
enum Kind {
    String(String),
    Integer(i32),
    Double(f64),
    Array(Vec<XmpValue>),
}

/// Tagged-union value used by [`Metadata`](crate::Metadata).
#[derive(Debug, Clone, PartialEq)]
pub struct XmpValue {
    kind: Kind,
}

impl XmpValue {
    /// Construct a string-typed value.
    pub fn from_string(value: impl Into<String>) -> Self {
        Self { kind: Kind::String(value.into()) }
    }

    /// Construct a signed 32-bit integer value.
    pub fn from_integer(value: i32) -> Self {
        Self { kind: Kind::Integer(value) }
    }

    /// Construct a double-precision float value.
    pub fn from_double(value: f64) -> Self {
        Self { kind: Kind::Double(value) }
    }

    /// Construct an array of `XmpValue`s. Preserves insertion
    /// order.
    pub fn from_array(items: Vec<XmpValue>) -> Self {
        Self { kind: Kind::Array(items) }
    }

    /// String rendering shared by [`Display`](fmt::Display).
    /// Integer / Double use invariant (US-en) formatting;
    /// arrays serialise their items joined by a single tab
    /// (matches the canonical `XmpField` and the Rust
    /// foundation `xmp::to_canonical` separator).
    pub fn to_string_value(&self) -> String {
        match &self.kind {
            Kind::String(s) => s.clone(),
            Kind::Integer(n) => n.to_string(),
            Kind::Double(d) => format_double_invariant(*d),
            Kind::Array(items) => {
                let parts: Vec<String> =
                    items.iter().map(|v| v.to_string_value()).collect();
                parts.join("\t")
            }
        }
    }

    /// Convert to a signed 32-bit integer. Integers return
    /// directly; doubles truncate toward zero; strings parse;
    /// arrays return
    /// [`Error::XmpValueConversion`].
    pub fn to_integer(&self) -> Result<i32, Error> {
        match &self.kind {
            Kind::Integer(n) => Ok(*n),
            Kind::Double(d) => Ok(*d as i32),
            Kind::String(s) => s.trim().parse::<i32>().map_err(|_| {
                Error::XmpValueConversion(format!(
                    "cannot parse string {s:?} as integer"
                ))
            }),
            Kind::Array(_) => Err(Error::XmpValueConversion(
                "array variant is not convertible to integer".into(),
            )),
        }
    }

    /// Convert to a double. Doubles return directly; integers
    /// widen; strings parse; arrays return
    /// [`Error::XmpValueConversion`].
    pub fn to_double(&self) -> Result<f64, Error> {
        match &self.kind {
            Kind::Double(d) => Ok(*d),
            Kind::Integer(n) => Ok(*n as f64),
            Kind::String(s) => s.trim().parse::<f64>().map_err(|_| {
                Error::XmpValueConversion(format!(
                    "cannot parse string {s:?} as double"
                ))
            }),
            Kind::Array(_) => Err(Error::XmpValueConversion(
                "array variant is not convertible to double".into(),
            )),
        }
    }

    /// Borrow the inner array slice. Returns
    /// [`Error::XmpValueConversion`]
    /// when the variant is not [`is_array`](Self::is_array).
    pub fn to_array(&self) -> Result<&[XmpValue], Error> {
        match &self.kind {
            Kind::Array(items) => Ok(items),
            _ => Err(Error::XmpValueConversion(
                "value is not an array".into(),
            )),
        }
    }

    /// True iff this value carries a string.
    pub fn is_string(&self) -> bool {
        matches!(self.kind, Kind::String(_))
    }

    /// True iff this value carries a signed 32-bit integer.
    pub fn is_integer(&self) -> bool {
        matches!(self.kind, Kind::Integer(_))
    }

    /// True iff this value carries a double.
    pub fn is_double(&self) -> bool {
        matches!(self.kind, Kind::Double(_))
    }

    /// True iff this value carries an array.
    pub fn is_array(&self) -> bool {
        matches!(self.kind, Kind::Array(_))
    }

    /// `DateTime` variant is out of v1 scope — always returns
    /// `false`.
    pub fn is_date_time(&self) -> bool {
        false
    }

    /// `XmpField` variant is out of v1 scope — always returns
    /// `false`.
    pub fn is_field(&self) -> bool {
        false
    }

    /// Named-value variant is out of v1 scope — always returns
    /// `false`.
    pub fn is_named_value(&self) -> bool {
        false
    }

    /// Raw XML-node variant is out of v1 scope — always returns
    /// `false`.
    pub fn is_raw(&self) -> bool {
        false
    }

    /// Named-values (collection) variant is out of v1 scope —
    /// always returns `false`.
    pub fn is_named_values(&self) -> bool {
        false
    }

    /// Structured-value variant is out of v1 scope — always
    /// returns `false`.
    pub fn is_structure(&self) -> bool {
        false
    }
}

impl fmt::Display for XmpValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_string_value())
    }
}

impl From<String> for XmpValue {
    fn from(s: String) -> Self {
        XmpValue::from_string(s)
    }
}

impl From<&str> for XmpValue {
    fn from(s: &str) -> Self {
        XmpValue::from_string(s)
    }
}

impl From<i32> for XmpValue {
    fn from(n: i32) -> Self {
        XmpValue::from_integer(n)
    }
}

impl From<f64> for XmpValue {
    fn from(d: f64) -> Self {
        XmpValue::from_double(d)
    }
}

// C# `ToString("R")` and Rust `f64::to_string()` both emit the
// shortest decimal that round-trips through the f64 precision —
// "3" for `3.0`, "3.5" for `3.5`. Match the canonical surface
// without a custom formatter.
fn format_double_invariant(d: f64) -> String {
    d.to_string()
}
