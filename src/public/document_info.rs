//! `Aspose.Pdf.DocumentInfo` — typed accessor over a
//! `Document`'s `/Info` dictionary (PDF 32000-1:2008 §14.3.3).
//!
//! Predefined keys are exposed as named accessors;
//! arbitrary keys are managed via [`add`](DocumentInfo::add) /
//! [`remove`](DocumentInfo::remove) / [`clear`](DocumentInfo::clear) /
//! [`clear_custom_data`](DocumentInfo::clear_custom_data).
//!
//! Mutations through this type stay in-memory: they are visible
//! to subsequent getter calls on the same instance, but do NOT
//! mark the owning [`Document`](crate::Document) for incremental
//! save. The persisted-on-save path is
//! [`Document::set_title`](crate::Document::set_title); future
//! beats add typed `set_<key>` shortcuts for the rest of the
//! predefined surface.

use super::error::Error;

/// Predefined keys exposed as named typed accessors. Order
/// matches the canonical .NET enumeration. Custom keys land
/// after these in insertion order.
const PREDEFINED_KEYS: &[&str] = &[
    "Title",
    "Author",
    "Subject",
    "Keywords",
    "Creator",
    "Producer",
    "Trapped",
    "CreationDate",
    "ModDate",
    "CreationTimeZone",
    "ModTimeZone",
];

/// Typed accessor over a [`Document`](crate::Document)'s
/// `/Info` dictionary.
///
/// `DocumentInfo` carries a list of `(key, value)` pairs in
/// insertion order. Reading a key that is absent returns the
/// empty string `""` (the canonical "unset" representation).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DocumentInfo {
    entries: Vec<(String, String)>,
}

impl DocumentInfo {
    /// Construct an empty `DocumentInfo`. Equivalent to a
    /// document with no `/Info` dictionary.
    pub fn new() -> Self {
        Self::default()
    }

    /// True iff `key` backs one of the predefined named
    /// accessors below.
    pub fn is_predefined_key(key: &str) -> bool {
        PREDEFINED_KEYS.contains(&key)
    }

    /// Add a custom `(key, value)` pair. Returns
    /// [`Error::DuplicateInfoKey`] when `key` is already
    /// present.
    ///
    /// To set a predefined key, prefer the typed
    /// `set_<name>` methods — they overwrite rather than error.
    pub fn add(
        &mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<(), Error> {
        let key = key.into();
        if self.entries.iter().any(|(k, _)| *k == key) {
            return Err(Error::DuplicateInfoKey(key));
        }
        self.entries.push((key, value.into()));
        Ok(())
    }

    /// Remove the entry under `key`, if any. No-op when absent.
    pub fn remove(&mut self, key: &str) {
        self.entries.retain(|(k, _)| k != key);
    }

    /// Remove every entry — both predefined and custom.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Remove only the custom (non-predefined) keys; leaves
    /// the predefined named entries intact.
    pub fn clear_custom_data(&mut self) {
        self.entries
            .retain(|(k, _)| Self::is_predefined_key(k));
    }

    /// Read-only view of every `(key, value)` pair in
    /// insertion order.
    pub fn entries(&self) -> &[(String, String)] {
        &self.entries
    }

    fn get(&self, key: &str) -> &str {
        self.entries
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
            .unwrap_or("")
    }

    pub(crate) fn set_entry(
        &mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) {
        let key = key.into();
        let value = value.into();
        if let Some(slot) = self.entries.iter_mut().find(|(k, _)| *k == key) {
            slot.1 = value;
        } else {
            self.entries.push((key, value));
        }
    }
}

// ============================================================
// Typed accessors for the predefined keys.
// ============================================================

macro_rules! predefined_accessor {
    ($key:literal, $getter:ident, $setter:ident, $doc:literal) => {
        impl DocumentInfo {
            #[doc = $doc]
            ///
            /// Returns `""` when the entry is absent.
            pub fn $getter(&self) -> &str {
                self.get($key)
            }

            #[doc = $doc]
            #[doc = ""]
            #[doc = "Overwrites any existing value for this key. The mutation"]
            #[doc = "stays in-memory; persistence requires the corresponding"]
            #[doc = "`Document::set_<key>` convenience or a future typed setter."]
            pub fn $setter(&mut self, value: impl Into<String>) {
                self.set_entry($key, value);
            }
        }
    };
}

predefined_accessor!("Title", title, set_title, "PDF `/Info /Title`.");
predefined_accessor!("Author", author, set_author, "PDF `/Info /Author`.");
predefined_accessor!("Subject", subject, set_subject, "PDF `/Info /Subject`.");
predefined_accessor!(
    "Keywords",
    keywords,
    set_keywords,
    "PDF `/Info /Keywords`."
);
predefined_accessor!("Creator", creator, set_creator, "PDF `/Info /Creator`.");
predefined_accessor!(
    "Producer",
    producer,
    set_producer,
    "PDF `/Info /Producer`."
);
predefined_accessor!(
    "Trapped",
    trapped,
    set_trapped,
    "PDF `/Info /Trapped` — conventionally `True` / `False` / `Unknown`."
);
predefined_accessor!(
    "CreationDate",
    creation_date,
    set_creation_date,
    "PDF `/Info /CreationDate` — typically `D:YYYYMMDDHHmmSSOHH'mm'`."
);
predefined_accessor!(
    "ModDate",
    mod_date,
    set_mod_date,
    "PDF `/Info /ModDate` — typically `D:YYYYMMDDHHmmSSOHH'mm'`."
);
