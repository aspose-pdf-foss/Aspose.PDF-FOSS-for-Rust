//! `Aspose.Pdf.Metadata` — typed accessor over a `Document`'s
//! XMP `/Metadata` packet (ISO 16684-1 / PDF 32000-1:2008
//! §14.3.2).
//!
//! Read or set XMP properties keyed by their `prefix:name` form
//! (e.g. `"dc:title"`, `"pdf:Producer"`). The packet itself is
//! parsed by `crate::internal::xmp`; this type is the
//! public-API wrapper that surfaces the canonical .NET
//! `Metadata` member set, adapted to Rust naming and
//! [`Result<T, E>`](crate::Error) error handling.
//!
//! v1 carries the string-keyed core (
//! [`add`](Metadata::add) /
//! [`insert`](Metadata::insert) /
//! [`remove`](Metadata::remove) /
//! [`contains_key`](Metadata::contains_key) /
//! [`get`](Metadata::get) /
//! [`keys`](Metadata::keys) /
//! [`values`](Metadata::values) /
//! [`len`](Metadata::len) /
//! [`clear`](Metadata::clear)
//! ) plus the namespace registration helpers. Dropped from
//! canonical for v1: the `KeyValuePair`-overloads of
//! `Add` / `Contains` / `Remove`, `CopyTo`, the PDF/A
//! `ExtensionFields` property, and BCL-typed `SyncRoot` /
//! `NamespaceManager` / `IsSynchronized` properties.

use std::collections::BTreeMap;

use crate::internal::xmp;

use super::error::Error;
use super::xmp_value::XmpValue;

/// Well-known XMP namespace URIs — used as a fallback when the
/// caller hasn't registered a prefix explicitly. Covers the
/// prefixes a PDF `/Metadata` stream typically carries.
fn default_namespace_uris(prefix: &str) -> Option<&'static str> {
    Some(match prefix {
        "dc" => "http://purl.org/dc/elements/1.1/",
        "pdf" => "http://ns.adobe.com/pdf/1.3/",
        "xmp" => "http://ns.adobe.com/xap/1.0/",
        "xmpMM" => "http://ns.adobe.com/xap/1.0/mm/",
        "xmpRights" => "http://ns.adobe.com/xap/1.0/rights/",
        "pdfaid" => "http://www.aiim.org/pdfa/ns/id/",
        _ => return None,
    })
}

/// Typed accessor over a [`Document`](crate::Document)'s
/// `/Metadata` XMP packet.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Metadata {
    entries: BTreeMap<String, XmpValue>,
    prefix_to_uri: BTreeMap<String, String>,
    uri_to_prefix: BTreeMap<String, String>,
    schema_descriptions: BTreeMap<String, String>,
    /// v1.1 write-side flag — read by `Document::save` to decide
    /// whether to emit a fresh `/Metadata` stream. Source-side
    /// population (`from_xmp_packet`) keeps it false.
    dirty: bool,
}

impl Metadata {
    /// Construct an empty `Metadata`. Equivalent to a document
    /// with no `/Metadata` indirect.
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct a [`Metadata`] from the raw bytes of an XMP
    /// packet (post filter-decoding). Empty input yields an
    /// empty map. Properties are loaded as
    /// [`XmpValue::from_string`] — the foundation XMP primitive
    /// flattens typed arrays / `rdf:Alt` to canonical strings.
    pub(crate) fn from_xmp_packet(packet: &[u8]) -> Result<Self, Error> {
        let mut m = Self::new();
        if packet.is_empty() {
            return Ok(m);
        }
        let parsed = xmp::parse(packet)?;
        for (k, v) in parsed.properties {
            m.entries.insert(k, XmpValue::from_string(v));
        }
        // Hydrate the prefix → URI registry from the parsed
        // packet so a subsequent save round-trips custom
        // namespaces without falling back to the built-in
        // default namespace map. Bypasses MarkDirty by
        // writing fields directly.
        for (prefix, uri) in parsed.namespace_uris {
            m.prefix_to_uri.insert(prefix.clone(), uri.clone());
            m.uri_to_prefix.insert(uri, prefix);
        }
        Ok(m)
    }

    /// Register a `prefix → namespace_uri` binding. Overwrites
    /// any existing binding for the same prefix or URI.
    pub fn register_namespace_uri(
        &mut self,
        prefix: impl Into<String>,
        namespace_uri: impl Into<String>,
    ) {
        let prefix = prefix.into();
        let uri = namespace_uri.into();
        self.prefix_to_uri.insert(prefix.clone(), uri.clone());
        self.uri_to_prefix.insert(uri, prefix);
        self.dirty = true;
    }

    /// Register a `prefix → namespace_uri` binding with an
    /// associated schema description (PDF/A schema metadata).
    pub fn register_namespace_uri_with_description(
        &mut self,
        prefix: impl Into<String>,
        namespace_uri: impl Into<String>,
        schema_description: impl Into<String>,
    ) {
        let prefix = prefix.into();
        self.register_namespace_uri(prefix.clone(), namespace_uri);
        self.schema_descriptions
            .insert(prefix, schema_description.into());
    }

    /// Resolve a prefix to its registered namespace URI. Returns
    /// `""` when the prefix has no binding (matches the canonical
    /// "no entry → empty string" surface).
    pub fn namespace_uri_by_prefix(&self, prefix: &str) -> &str {
        self.prefix_to_uri
            .get(prefix)
            .map(String::as_str)
            .unwrap_or("")
    }

    /// Resolve a namespace URI to its registered prefix. Returns
    /// `""` when the URI has no binding.
    pub fn prefix_by_namespace_uri(&self, namespace_uri: &str) -> &str {
        self.uri_to_prefix
            .get(namespace_uri)
            .map(String::as_str)
            .unwrap_or("")
    }

    /// Add a `(key, value)` pair. Returns
    /// [`Error::DuplicateMetadataKey`] when `key` is already
    /// present. For silent replacement use
    /// [`Metadata::insert`].
    pub fn add(
        &mut self,
        key: impl Into<String>,
        value: XmpValue,
    ) -> Result<(), Error> {
        let key = key.into();
        if self.entries.contains_key(&key) {
            return Err(Error::DuplicateMetadataKey(key));
        }
        self.entries.insert(key, value);
        self.dirty = true;
        Ok(())
    }

    /// Insert a `(key, value)` pair, replacing any existing
    /// value. Returns the old value if one was present (mirrors
    /// the canonical indexer setter).
    pub fn insert(
        &mut self,
        key: impl Into<String>,
        value: XmpValue,
    ) -> Option<XmpValue> {
        let old = self.entries.insert(key.into(), value);
        self.dirty = true;
        old
    }

    /// Remove every property and every namespace binding.
    pub fn clear(&mut self) {
        let was_non_empty = !self.entries.is_empty()
            || !self.prefix_to_uri.is_empty();
        self.entries.clear();
        self.prefix_to_uri.clear();
        self.uri_to_prefix.clear();
        self.schema_descriptions.clear();
        if was_non_empty {
            self.dirty = true;
        }
    }

    /// True iff `key` is present in the property map.
    pub fn contains_key(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }

    /// Remove the entry under `key`, if any. Returns `true` when
    /// an entry was removed.
    pub fn remove(&mut self, key: &str) -> bool {
        let removed = self.entries.remove(key).is_some();
        if removed {
            self.dirty = true;
        }
        removed
    }

    /// Borrow the value under `key`, if any.
    pub fn get(&self, key: &str) -> Option<&XmpValue> {
        self.entries.get(key)
    }

    /// Iterate over property keys in sorted order.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }

    /// Iterate over property values in key-sorted order.
    pub fn values(&self) -> impl Iterator<Item = &XmpValue> {
        self.entries.values()
    }

    /// Iterate over `(key, value)` pairs in key-sorted order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &XmpValue)> {
        self.entries.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Number of entries in the property map.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True iff [`len`](Self::len) is zero.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Always `false` — Metadata's property map is variable-size.
    pub fn is_fixed_size(&self) -> bool {
        false
    }

    /// Always `false` — Metadata's property map accepts writes.
    pub fn is_read_only(&self) -> bool {
        false
    }

    /// Read by `Document::save` to decide whether to emit a
    /// fresh `/Metadata` stream update. crate-private.
    pub(crate) fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// crate-private — reserved for tests; Document::save does
    /// NOT reset the flag after a successful save (matches the
    /// existing `dirty` semantics on Document).
    #[allow(dead_code)]
    pub(crate) fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    /// Serialise this Metadata to an XMP packet (ISO 16684-1).
    /// Emits a minimal RDF/XML envelope: `<?xpacket?>` framing,
    /// `<x:xmpmeta>` root, one `<rdf:Description>` containing
    /// one element per entry. Each key splits on `:` into
    /// (prefix, local-name); prefix resolves via per-instance
    /// registry or the built-in `default_namespace_uris`
    /// fallback. Unknown prefixes return [`Error::InvalidPdf`].
    pub(crate) fn to_xmp_packet(&self) -> Result<Vec<u8>, Error> {
        let mut prefixes_used: Vec<&str> = Vec::new();
        let mut seen: std::collections::BTreeSet<&str> =
            std::collections::BTreeSet::new();
        for key in self.entries.keys() {
            let colon = key.find(':').ok_or_else(|| Error::InvalidPdf(
                format!("Metadata key '{key}' is missing a \
                    namespace prefix (expected 'prefix:local-name')")))?;
            if colon == 0 {
                return Err(Error::InvalidPdf(format!(
                    "Metadata key '{key}' has empty prefix")));
            }
            let prefix = &key[..colon];
            if seen.insert(prefix) {
                prefixes_used.push(prefix);
            }
        }
        let mut prefix_uris: Vec<(String, String)> = Vec::new();
        for prefix in prefixes_used {
            let uri = self.prefix_to_uri.get(prefix)
                .map(String::as_str)
                .or_else(|| default_namespace_uris(prefix))
                .ok_or_else(|| Error::InvalidPdf(format!(
                    "Metadata: prefix '{prefix}' has no registered \
                    namespace URI; call register_namespace_uri \
                    before save")))?;
            prefix_uris.push((prefix.to_string(), uri.to_string()));
        }

        let mut sb = String::with_capacity(512);
        sb.push_str(
            "<?xpacket begin=\"\u{feff}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n");
        sb.push_str("<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\n");
        sb.push_str(" <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n");
        sb.push_str("  <rdf:Description rdf:about=\"\"");
        for (prefix, uri) in &prefix_uris {
            sb.push_str("\n   xmlns:");
            sb.push_str(prefix);
            sb.push_str("=\"");
            sb.push_str(&xml_attr_escape(uri));
            sb.push('"');
        }
        sb.push_str(">\n");
        for (key, value) in &self.entries {
            sb.push_str("   <");
            sb.push_str(key);
            sb.push('>');
            sb.push_str(&xml_text_escape(&value.to_string_value()));
            sb.push_str("</");
            sb.push_str(key);
            sb.push_str(">\n");
        }
        sb.push_str("  </rdf:Description>\n");
        sb.push_str(" </rdf:RDF>\n");
        sb.push_str("</x:xmpmeta>\n");
        sb.push_str("<?xpacket end=\"w\"?>\n");
        Ok(sb.into_bytes())
    }
}

fn xml_text_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn xml_attr_escape(text: &str) -> String {
    xml_text_escape(text).replace('"', "&quot;")
}
