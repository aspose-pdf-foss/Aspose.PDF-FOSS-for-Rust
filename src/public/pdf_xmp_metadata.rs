//! `Aspose.Pdf.Facades.PdfXmpMetadata` — a dictionary-style facade over
//! the bound document's XMP metadata, per
//! `capabilities/pdf_xmp_metadata.yaml`.
//!
//! Extends [`SaveableFacade`](crate::SaveableFacade). The string-keyed
//! dictionary surface (`add` / `remove` / `contains` / `contains_key` /
//! `try_get_value` / the indexer `get` + `set` / `keys` / `values` /
//! `count` / `clear`) and the namespace-URI helpers wire through the
//! REAL foundation [`Metadata`](crate::Metadata) on the bound document
//! (`Document::metadata` / `metadata_mut` — parsed on load, written as an
//! incremental `/Metadata` update on save).
//!
//! Dropped (outside the shipped subset, mirroring the C++ reference):
//! the `DefaultMetadataProperties`-keyed overloads (need the DMP <-> XMP
//! key table), the `Object`-valued `Add`, the `XmpPdfAExtensionObject`
//! `Add` + `ExtensionFields` (types not in the catalog), `GetXmpMetadata`
//! (the byte-packet serialisation is Document-internal), and the legacy
//! .NET collection idioms (`GetEnumerator` / `CopyTo` / `KeyValuePair`
//! `Add`/`Contains`/`Remove` / `IsSynchronized` / `SyncRoot`).

use crate::public::document::Document;
use crate::public::error::Error;
use crate::public::i_facade::{impl_facade, FacadeBase};
use crate::public::i_saveable_facade::impl_saveable_facade;
use crate::public::xmp_value::XmpValue;

/// A dictionary-style facade over a document's XMP metadata.
#[derive(Debug, Default)]
pub struct PdfXmpMetadata {
    base: FacadeBase,
}

impl PdfXmpMetadata {
    /// Construct an unbound facade.
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct and bind to an in-memory [`Document`].
    pub fn with_document(document: Document) -> Self {
        Self { base: FacadeBase { document: Some(document) } }
    }

    // ---- Namespace registry ----

    /// Register a `prefix -> namespaceURI` binding on the bound
    /// document's metadata (no-op when unbound).
    pub fn register_namespace_uri(
        &mut self,
        prefix: impl Into<String>,
        namespace_uri: impl Into<String>,
    ) {
        if let Some(d) = self.base.document.as_mut() {
            d.metadata_mut().register_namespace_uri(prefix, namespace_uri);
        }
    }

    /// The namespace URI bound to `prefix` (empty when absent / unbound).
    pub fn namespace_uri_by_prefix(&self, prefix: &str) -> String {
        self.base
            .document
            .as_ref()
            .map(|d| d.metadata().namespace_uri_by_prefix(prefix).to_string())
            .unwrap_or_default()
    }

    /// The prefix bound to `namespace_uri` (empty when absent / unbound).
    pub fn prefix_by_namespace_uri(&self, namespace_uri: &str) -> String {
        self.base
            .document
            .as_ref()
            .map(|d| d.metadata().prefix_by_namespace_uri(namespace_uri).to_string())
            .unwrap_or_default()
    }

    // ---- Dictionary surface (real — wired to Document metadata) ----

    /// Add a `key -> value` entry. Errors when no document is bound or
    /// the key is already present (mirrors `IDictionary.Add`).
    pub fn add(
        &mut self,
        key: impl Into<String>,
        value: XmpValue,
    ) -> Result<(), Error> {
        match self.base.document.as_mut() {
            Some(d) => d.metadata_mut().add(key, value),
            None => Err(Error::InvalidPdf("facade has no bound document".into())),
        }
    }

    /// Remove every entry (no-op when unbound).
    pub fn clear(&mut self) {
        if let Some(d) = self.base.document.as_mut() {
            d.metadata_mut().clear();
        }
    }

    /// Whether an entry exists under `key`.
    pub fn contains(&self, key: &str) -> bool {
        self.contains_key(key)
    }

    /// Remove the entry under `key`. Returns `true` when one was removed.
    pub fn remove(&mut self, key: &str) -> bool {
        self.base.document.as_mut().is_some_and(|d| d.metadata_mut().remove(key))
    }

    /// Whether an entry exists under `key` (`IDictionary.ContainsKey`).
    pub fn contains_key(&self, key: &str) -> bool {
        self.base.document.as_ref().is_some_and(|d| d.metadata().contains_key(key))
    }

    /// The value under `key`, if present (`IDictionary.TryGetValue`).
    pub fn try_get_value(&self, key: &str) -> Option<&XmpValue> {
        self.base.document.as_ref().and_then(|d| d.metadata().get(key))
    }

    /// The property keys (empty when unbound), in sorted order.
    pub fn keys(&self) -> Vec<String> {
        self.base
            .document
            .as_ref()
            .map(|d| d.metadata().keys().map(str::to_string).collect())
            .unwrap_or_default()
    }

    /// The property values (empty when unbound), in key-sorted order.
    pub fn values(&self) -> Vec<XmpValue> {
        self.base
            .document
            .as_ref()
            .map(|d| d.metadata().values().cloned().collect())
            .unwrap_or_default()
    }

    /// The value under `key`, if present (the `this[key]` getter).
    pub fn get(&self, key: &str) -> Option<&XmpValue> {
        self.try_get_value(key)
    }

    /// Set the value under `key`, replacing any existing entry (the
    /// `this[key]` setter; no-op when unbound).
    pub fn set(&mut self, key: impl Into<String>, value: XmpValue) {
        if let Some(d) = self.base.document.as_mut() {
            d.metadata_mut().insert(key, value);
        }
    }

    /// Always `false` — the XMP property map is variable-size.
    pub fn is_fixed_size(&self) -> bool {
        false
    }

    /// Always `false` — the XMP property map accepts writes.
    pub fn is_read_only(&self) -> bool {
        false
    }

    /// The number of entries (0 when unbound).
    pub fn count(&self) -> i32 {
        self.base.document.as_ref().map_or(0, |d| d.metadata().len() as i32)
    }
}

impl_facade!(PdfXmpMetadata);
impl_saveable_facade!(PdfXmpMetadata);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::public::i_saveable_facade::ISaveableFacade;

    fn with_info_pdf() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/metadata/with_info.pdf")
    }

    fn bound() -> PdfXmpMetadata {
        PdfXmpMetadata::with_document(Document::open(with_info_pdf()).unwrap())
    }

    #[test]
    fn add_contains_count_through_bound_doc() {
        let mut xmp = bound();
        xmp.add("dc:creator", XmpValue::from_string("LibForge")).unwrap();
        assert!(xmp.contains("dc:creator"));
        assert!(xmp.contains_key("dc:creator"));
        assert!(xmp.count() >= 1);
        assert!(!xmp.is_read_only());
        assert!(!xmp.is_fixed_size());
    }

    #[test]
    fn try_get_value_and_indexer() {
        let mut xmp = bound();
        xmp.add("dc:title", XmpValue::from_string("Hello XMP")).unwrap();

        let got = xmp.try_get_value("dc:title").unwrap();
        assert_eq!(got.to_string_value(), "Hello XMP");
        assert_eq!(xmp.get("dc:title").unwrap().to_string_value(), "Hello XMP");

        // Indexer assignment replaces.
        xmp.set("dc:title", XmpValue::from_string("Updated"));
        assert_eq!(xmp.get("dc:title").unwrap().to_string_value(), "Updated");
    }

    #[test]
    fn keys_values_remove_clear() {
        let mut xmp = bound();
        xmp.add("dc:a", XmpValue::from_string("1")).unwrap();
        xmp.add("dc:b", XmpValue::from_string("2")).unwrap();

        assert!(xmp.keys().len() >= 2);
        assert!(xmp.values().len() >= 2);

        assert!(xmp.remove("dc:a"));
        assert!(!xmp.contains("dc:a"));

        xmp.clear();
        assert_eq!(xmp.count(), 0);
    }

    #[test]
    fn namespace_registry() {
        let mut xmp = bound();
        let uri = "http://purl.org/dc/elements/1.1/";
        xmp.register_namespace_uri("dc", uri);
        assert_eq!(xmp.namespace_uri_by_prefix("dc"), uri);
        assert_eq!(xmp.prefix_by_namespace_uri(uri), "dc");
    }

    #[test]
    fn save_then_reload_persists() {
        let out = std::env::temp_dir().join("aspose_rust_xmpfacade_roundtrip.pdf");
        {
            let mut xmp = bound();
            xmp.register_namespace_uri("dc", "http://purl.org/dc/elements/1.1/");
            xmp.add("dc:title", XmpValue::from_string("Facade Title")).unwrap();
            xmp.save(&out).unwrap();
        }
        let reloaded = Document::open(&out).unwrap();
        assert_eq!(
            reloaded.metadata().get("dc:title").unwrap().to_string_value(),
            "Facade Title"
        );
        let _ = std::fs::remove_file(&out);
    }

    #[test]
    fn unbound_reads_safe_mutations_error() {
        let mut xmp = PdfXmpMetadata::new();
        assert_eq!(xmp.count(), 0);
        assert!(!xmp.contains("k"));
        assert!(xmp.keys().is_empty());
        assert!(xmp.add("k", XmpValue::from_string("v")).is_err());
    }
}
