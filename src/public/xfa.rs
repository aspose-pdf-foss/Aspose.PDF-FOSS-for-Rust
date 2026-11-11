//! `Aspose.Pdf.Forms.XFA` — XFA (XML Forms Architecture) form data,
//! per `capabilities/xfa.yaml`.
//!
//! The `System.Xml`-based members (templates / datasets / config /
//! XDP / namespace manager) and the Stream image setter are dropped
//! — only the name-keyed field-value access and the field-name list
//! ship.

use std::collections::BTreeMap;

/// XFA form data — field values keyed by name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct XFA {
    values: BTreeMap<String, String>,
}

impl XFA {
    /// Construct empty XFA form data.
    pub fn new() -> Self {
        Self::default()
    }

    /// The field value for `name`, or an empty string when absent.
    pub fn get_value(&self, name: &str) -> String {
        self.values.get(name).cloned().unwrap_or_default()
    }
    /// Set the field value for `name`.
    pub fn set_value(&mut self, name: &str, value: String) {
        self.values.insert(name.to_string(), value);
    }

    /// The names of all XFA fields.
    pub fn field_names(&self) -> Vec<String> {
        self.values.keys().cloned().collect()
    }
}
