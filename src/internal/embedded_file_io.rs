//! Target-side helper: read / write the catalog's `/Names
//! /EmbeddedFiles` name tree (document-level embedded files). Backs the
//! public `Document::embedded_files` save-through. Non-anchored
//! (composes the trailer / xref / objects parsers + the incremental
//! writer).
//!
//! Each entry is a `/Filespec` dictionary (`/F`, `/UF`, `/Desc`,
//! `/AFRelationship`, `/FS`) whose
//! `/EF /F` points at an `/EmbeddedFile` stream carrying the file bytes.
//! On write the content bytes are embedded verbatim. On read the
//! metadata round-trips through the public
//! [`FileSpecification`](crate::FileSpecification), which drops the
//! canonical `Contents` getter — the bytes are reached instead through
//! [`PdfExtractor`](crate::PdfExtractor), whose attachment methods ask
//! [`extract_contents`] for one file at a time rather than decoding
//! every attachment when a document is opened.

use std::collections::HashMap;

use super::objects::{self, Value};
use super::pdf_writer_incremental::{self, DirtyObject};
use super::{trailer, xref};

/// One embedded file's parsed metadata (no content — see module docs).
pub(crate) struct EmbeddedFileData {
    pub key: String,
    pub name: String,
    pub unicode_name: String,
    pub description: String,
    pub mime_type: String,
    /// `/AFRelationship` name (§14.13.5), empty when absent.
    pub af_relationship: String,
    /// `/FS` file-system handler name (§7.11.2 — `URL`, `DOS`, `Mac`,
    /// `Unix`), empty when absent.
    pub file_system: String,
    /// `/EF /F /Params` entries (all optional). `size` is `-1` when the
    /// `/Size` key is absent; the date / checksum strings are the raw
    /// `/Params` values (empty when absent).
    pub size: i64,
    pub creation_date: String,
    pub mod_date: String,
    pub check_sum: String,
    /// `/EP` encrypted-payload descriptor (`Type` / `Subtype` / `Version`),
    /// `None` when the specification carries no `/EP` entry.
    pub encrypted_payload: Option<EncryptedPayloadData>,
}

/// The parsed `/EP` fields of a file-specification dictionary.
#[derive(Clone, Default)]
pub(crate) struct EncryptedPayloadData {
    pub type_: String,
    pub subtype: String,
    pub version: String,
}

/// One embedded file to write. `content` is `Some` for a freshly
/// supplied file (a new `/EmbeddedFile` stream is emitted) and `None`
/// for an entry loaded from the document, whose original `/EF` stream
/// is reused by key (the public surface can't re-read embedded bytes).
pub(crate) struct EmbeddedFileSpec {
    pub key: String,
    pub name: String,
    pub unicode_name: String,
    pub description: String,
    pub mime_type: String,
    /// `/AFRelationship` name, or `None` to leave the entry out.
    pub af_relationship: Option<&'static str>,
    /// `/FS` handler name, empty to leave the entry out.
    pub file_system: String,
    pub content: Option<Vec<u8>>,
}

// ---- Read ------------------------------------------------------------

/// Parse the embedded-file entries from `bytes`, in name-tree order.
pub(crate) fn parse(bytes: &[u8]) -> Vec<EmbeddedFileData> {
    let Ok(tr) = trailer::parse(bytes) else {
        return Vec::new();
    };
    let objs = build_objs(bytes);
    let Some(catalog) = objs.get(&tr.root_id) else {
        return Vec::new();
    };
    let Some(names) = dict_get(catalog, b"Names").and_then(|v| resolve(&objs, &v)) else {
        return Vec::new();
    };
    let Some(ef) = dict_get(&names, b"EmbeddedFiles").and_then(|v| resolve(&objs, &v)) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    collect_tree(&objs, &ef, &mut out);
    out
}

fn collect_tree(objs: &HashMap<u32, Value>, node: &Value, out: &mut Vec<EmbeddedFileData>) {
    if let Some(Value::Array(kids)) = dict_get(node, b"Kids") {
        for kid in &kids {
            if let Some(resolved) = resolve(objs, kid) {
                collect_tree(objs, &resolved, out);
            }
        }
    }
    if let Some(Value::Array(names)) = dict_get(node, b"Names") {
        let mut i = 0;
        while i + 1 < names.len() {
            if let Value::String(key) = &names[i] {
                if let Some(spec) = resolve(objs, &names[i + 1]) {
                    out.push(parse_filespec(objs, &key.clone(), &spec));
                }
            }
            i += 2;
        }
    }
}

fn parse_filespec(objs: &HashMap<u32, Value>, key: &[u8], spec: &Value) -> EmbeddedFileData {
    let name = string_field(spec, b"F");
    let unicode_name = string_field(spec, b"UF");
    let description = string_field(spec, b"Desc");
    let af_relationship = name_field(spec, b"AFRelationship");
    let file_system = name_field(spec, b"FS");
    let ef_stream = dict_get(spec, b"EF")
        .and_then(|ef| dict_get(&ef, b"F"))
        .and_then(|f| resolve(objs, &f));
    let mime_type = ef_stream
        .as_ref()
        .and_then(|stream| dict_get(stream, b"Subtype"))
        .and_then(|s| match s {
            Value::Name(n) => Some(decode_name(&n)),
            _ => None,
        })
        .unwrap_or_default();

    // `/EF /F /Params` — Size (Integer) + CreationDate / ModDate /
    // CheckSum (strings). All optional.
    let params = ef_stream.as_ref().and_then(|stream| dict_get(stream, b"Params"));
    let size = params
        .as_ref()
        .and_then(|p| dict_get(p, b"Size"))
        .and_then(|v| match v {
            Value::Integer(n) => Some(n),
            _ => None,
        })
        .unwrap_or(-1);
    let creation_date = params
        .as_ref()
        .map(|p| string_field(p, b"CreationDate"))
        .unwrap_or_default();
    let mod_date = params
        .as_ref()
        .map(|p| string_field(p, b"ModDate"))
        .unwrap_or_default();
    let check_sum = params
        .as_ref()
        .map(|p| string_field(p, b"CheckSum"))
        .unwrap_or_default();

    // `/EP` — the encrypted-payload dict sits directly on the file spec
    // (a sibling of `/EF`), carrying `/Type` (a name, fixed to
    // `EncryptedPayload`), `/Subtype` (the crypto-filter name) and
    // `/Version` (a string).
    let encrypted_payload = dict_get(spec, b"EP")
        .and_then(|v| resolve(objs, &v))
        .map(|ep| EncryptedPayloadData {
            type_: name_field(&ep, b"Type"),
            subtype: name_field(&ep, b"Subtype"),
            version: version_field(&ep, b"Version"),
        });

    EmbeddedFileData {
        key: String::from_utf8_lossy(key).into_owned(),
        name,
        unicode_name,
        description,
        mime_type,
        af_relationship,
        file_system,
        size,
        creation_date,
        mod_date,
        check_sum,
        encrypted_payload,
    }
}

/// The decoded bytes of the embedded file whose `/F` name is
/// `file_name`, or `None` when no entry has that name, the entry has no
/// `/EF` stream, or its filter chain is one this function cannot undo.
///
/// Reading these is on demand rather than part of `parse`: an
/// attachment can be megabytes, and opening a document is not a reason
/// to decode every one of them.
pub(crate) fn extract_contents(bytes: &[u8], file_name: &str) -> Option<Vec<u8>> {
    let tr = trailer::parse(bytes).ok()?;
    let objs = build_objs(bytes);
    let catalog = objs.get(&tr.root_id)?;
    let names = dict_get(catalog, b"Names").and_then(|v| resolve(&objs, &v))?;
    let tree = dict_get(&names, b"EmbeddedFiles").and_then(|v| resolve(&objs, &v))?;
    let mut specs = Vec::new();
    collect_specs(&objs, &tree, &mut specs);
    for spec in specs {
        if string_field(&spec, b"F") != file_name && string_field(&spec, b"UF") != file_name {
            continue;
        }
        let stream = dict_get(&spec, b"EF")
            .and_then(|ef| dict_get(&ef, b"F"))
            .and_then(|f| resolve(&objs, &f))?;
        let (dict, body) = match stream {
            Value::Stream { dict, body } => (dict, body),
            _ => return None,
        };
        return decode_stream(&dict, &body);
    }
    None
}

/// Collect every file-specification dictionary in a name tree.
fn collect_specs(objs: &HashMap<u32, Value>, node: &Value, out: &mut Vec<Value>) {
    if let Some(Value::Array(kids)) = dict_get(node, b"Kids") {
        for kid in &kids {
            if let Some(resolved) = resolve(objs, kid) {
                collect_specs(objs, &resolved, out);
            }
        }
    }
    if let Some(Value::Array(names)) = dict_get(node, b"Names") {
        let mut i = 1;
        while i < names.len() {
            if let Some(spec) = resolve(objs, &names[i]) {
                out.push(spec);
            }
            i += 2;
        }
    }
}

/// Undo a stream's filter chain, for the filters an attachment is
/// realistically written with. `None` for any other filter: handing
/// back still-encoded bytes as if they were the file would be worse
/// than saying nothing.
fn decode_stream(dict: &[(Vec<u8>, Value)], body: &[u8]) -> Option<Vec<u8>> {
    use crate::internal::{ascii85, asciihex, flate, lzw};
    let filters: Vec<Vec<u8>> = match dict_get(&Value::Dict(dict.to_vec()), b"Filter") {
        None => Vec::new(),
        Some(Value::Name(n)) => vec![n],
        Some(Value::Array(items)) => items
            .into_iter()
            .map(|f| match f {
                Value::Name(n) => Some(n),
                _ => None,
            })
            .collect::<Option<Vec<_>>>()?,
        _ => return None,
    };
    let mut data = body.to_vec();
    for filter in filters {
        data = match filter.as_slice() {
            b"FlateDecode" | b"Fl" => flate::decode(&data).ok()?,
            b"LZWDecode" | b"LZW" => lzw::decode(&data).ok()?,
            b"ASCII85Decode" | b"A85" => ascii85::decode(&data).ok()?,
            b"ASCIIHexDecode" | b"AHx" => asciihex::decode(&data).ok()?,
            _ => return None,
        };
    }
    Some(data)
}

// ---- Write -----------------------------------------------------------

/// Append an incremental update writing `specs` into the catalog's
/// `/Names /EmbeddedFiles` name tree, preserving any sibling `/Names`
/// branch (e.g. `/Dests`). Returns `None` when the document can't be
/// parsed.
pub(crate) fn append(bytes: &[u8], specs: &[EmbeddedFileSpec]) -> Option<Vec<u8>> {
    let tr = trailer::parse(bytes).ok()?;
    let objs = build_objs(bytes);
    let mut catalog = match objs.get(&tr.root_id)? {
        Value::Dict(d) => d.clone(),
        _ => return None,
    };

    // Original /EF dicts keyed by name-tree key, so loaded entries
    // keep referencing their existing embedded stream.
    let existing_ef = existing_ef_by_key(&objs, &catalog);

    let mut next_id = tr.size as u32;
    let mut dirty: Vec<DirtyObject> = Vec::with_capacity(specs.len() + 1);

    // Name-tree leaf entries in lexicographic key order (PDF §7.9.6).
    let mut sorted: Vec<&EmbeddedFileSpec> = specs.iter().collect();
    sorted.sort_by(|a, b| a.key.cmp(&b.key));

    let mut names_array: Vec<Value> = Vec::with_capacity(sorted.len() * 2);
    for spec in sorted {
        let mut filespec: Vec<(Vec<u8>, Value)> = vec![
            (b"Type".to_vec(), Value::Name(b"Filespec".to_vec())),
            (b"F".to_vec(), Value::String(spec.name.clone().into_bytes())),
        ];
        if !spec.unicode_name.is_empty() {
            filespec.push((b"UF".to_vec(), Value::String(spec.unicode_name.clone().into_bytes())));
        }
        if !spec.description.is_empty() {
            filespec.push((b"Desc".to_vec(), Value::String(spec.description.clone().into_bytes())));
        }
        // §14.13.5: the relationship the file bears to the document, as
        // a name. §7.11.2: the file-system handler whose rules the `/F`
        // name follows (`URL` for a uniform resource locator); absent
        // means the standard PDF file-name grammar.
        if let Some(rel) = spec.af_relationship {
            filespec.push((b"AFRelationship".to_vec(), Value::Name(rel.as_bytes().to_vec())));
        }
        if !spec.file_system.is_empty() {
            filespec.push((b"FS".to_vec(), Value::Name(encode_name(&spec.file_system))));
        }

        let ef = match &spec.content {
            Some(bytes) => {
                let stream_id = next_id;
                next_id += 1;
                let mut stream_dict: Vec<(Vec<u8>, Value)> =
                    vec![(b"Type".to_vec(), Value::Name(b"EmbeddedFile".to_vec()))];
                if !spec.mime_type.is_empty() {
                    stream_dict
                        .push((b"Subtype".to_vec(), Value::Name(encode_name(&spec.mime_type))));
                }
                stream_dict.push((
                    b"Params".to_vec(),
                    Value::Dict(vec![(b"Size".to_vec(), Value::Integer(bytes.len() as i64))]),
                ));
                dirty.push(DirtyObject {
                    id: stream_id,
                    generation: 0,
                    value: Value::Stream { dict: stream_dict, body: bytes.clone() },
                });
                Some(Value::Dict(vec![(b"F".to_vec(), Value::Ref { id: stream_id, gen: 0 })]))
            }
            // Loaded entry — reuse its original /EF stream reference.
            None => existing_ef.get(&spec.key).cloned(),
        };
        if let Some(ef) = ef {
            filespec.push((b"EF".to_vec(), ef));
        }

        names_array.push(Value::String(spec.key.clone().into_bytes()));
        names_array.push(Value::Dict(filespec));
    }

    let ef_tree = Value::Dict(vec![(b"Names".to_vec(), Value::Array(names_array))]);

    // Merge into the existing /Names dict so a sibling /Dests branch
    // survives.
    let mut names_dict = match dict_get(&Value::Dict(catalog.clone()), b"Names")
        .and_then(|v| resolve(&objs, &v))
    {
        Some(Value::Dict(d)) => d,
        _ => Vec::new(),
    };
    set_entry(&mut names_dict, b"EmbeddedFiles", ef_tree);
    set_entry(&mut catalog, b"Names", Value::Dict(names_dict));

    dirty.push(DirtyObject { id: tr.root_id, generation: 0, value: Value::Dict(catalog) });
    pdf_writer_incremental::append_incremental(bytes, &dirty).ok()
}

/// Map each existing embedded-file key to its original `/EF` dict, so a
/// loaded entry can keep referencing its embedded stream on rewrite.
fn existing_ef_by_key(
    objs: &HashMap<u32, Value>,
    catalog: &[(Vec<u8>, Value)],
) -> HashMap<String, Value> {
    let mut map = HashMap::new();
    let Some(ef_tree) = dict_get(&Value::Dict(catalog.to_vec()), b"Names")
        .and_then(|v| resolve(objs, &v))
        .and_then(|names| dict_get(&names, b"EmbeddedFiles"))
        .and_then(|v| resolve(objs, &v))
    else {
        return map;
    };
    collect_ef(objs, &ef_tree, &mut map);
    map
}

fn collect_ef(objs: &HashMap<u32, Value>, node: &Value, out: &mut HashMap<String, Value>) {
    if let Some(Value::Array(kids)) = dict_get(node, b"Kids") {
        for kid in &kids {
            if let Some(resolved) = resolve(objs, kid) {
                collect_ef(objs, &resolved, out);
            }
        }
    }
    if let Some(Value::Array(names)) = dict_get(node, b"Names") {
        let mut i = 0;
        while i + 1 < names.len() {
            if let Value::String(key) = &names[i] {
                if let Some(spec) = resolve(objs, &names[i + 1]) {
                    if let Some(ef) = dict_get(&spec, b"EF") {
                        out.insert(String::from_utf8_lossy(key).into_owned(), ef);
                    }
                }
            }
            i += 2;
        }
    }
}

// ---- Shared helpers --------------------------------------------------

fn build_objs(bytes: &[u8]) -> HashMap<u32, Value> {
    let mut objs = HashMap::new();
    if let Ok(table) = xref::parse(bytes) {
        for entry in &table.entries {
            if let xref::Entry::InUse { id, offset, .. } = *entry {
                if let Ok(o) = objects::parse_object(bytes, offset as usize) {
                    objs.insert(id, o.value);
                }
            }
        }
    }
    objs
}

fn resolve(objs: &HashMap<u32, Value>, v: &Value) -> Option<Value> {
    match v {
        Value::Ref { id, .. } => objs.get(id).cloned(),
        other => Some(other.clone()),
    }
}

fn dict_get(v: &Value, key: &[u8]) -> Option<Value> {
    let entries = match v {
        Value::Dict(e) => e,
        Value::Stream { dict, .. } => dict,
        _ => return None,
    };
    entries.iter().find(|(k, _)| k.as_slice() == key).map(|(_, val)| val.clone())
}

fn set_entry(entries: &mut Vec<(Vec<u8>, Value)>, key: &[u8], value: Value) {
    if let Some(slot) = entries.iter_mut().find(|(k, _)| k.as_slice() == key) {
        slot.1 = value;
    } else {
        entries.push((key.to_vec(), value));
    }
}

fn string_field(v: &Value, key: &[u8]) -> String {
    match dict_get(v, key) {
        Some(Value::String(b)) => String::from_utf8_lossy(&b).into_owned(),
        _ => String::new(),
    }
}

/// Read a name-valued field, decoding `#xx` escapes. Empty when absent or
/// not a name.
fn name_field(v: &Value, key: &[u8]) -> String {
    match dict_get(v, key) {
        Some(Value::Name(n)) => decode_name(&n),
        _ => String::new(),
    }
}

/// Read the `/Version` field, which a document may store as a string or a
/// number. Numbers are rendered without a trailing `.0`. Empty when absent.
fn version_field(v: &Value, key: &[u8]) -> String {
    match dict_get(v, key) {
        Some(Value::String(b)) => String::from_utf8_lossy(&b).into_owned(),
        Some(Value::Integer(n)) => n.to_string(),
        Some(Value::Real(r)) => {
            let s = r.to_string();
            s.strip_suffix(".0").map(|t| t.to_string()).unwrap_or(s)
        }
        _ => String::new(),
    }
}

/// Encode a MIME type as a PDF name (`/` → `#2F`, per §7.3.5).
fn encode_name(s: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len());
    for &b in s.as_bytes() {
        if (0x21..=0x7E).contains(&b) && !matches!(b, b'/' | b'#' | b'(' | b')' | b'<' | b'>') {
            out.push(b);
        } else {
            out.push(b'#');
            out.push(hex_nibble(b >> 4));
            out.push(hex_nibble(b & 0x0F));
        }
    }
    out
}

/// Decode a PDF name body's `#XX` escapes back to bytes.
fn decode_name(b: &[u8]) -> String {
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'#' && i + 2 < b.len() {
            if let (Some(hi), Some(lo)) = (hex_val(b[i + 1]), hex_val(b[i + 2])) {
                out.push((hi << 4) | lo);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_nibble(n: u8) -> u8 {
    if n < 10 {
        b'0' + n
    } else {
        b'A' + (n - 10)
    }
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}
