// PDF text extractor — pure function from raw PDF bytes to one
// UTF-8 plaintext string per page in /Pages-tree preorder.
//
// Composes the entire foundation read-side text path:
//   * `xref` + `objects`     — structural parse + object dump
//   * `flate`                — /Contents and /ToUnicode stream
//                              decompression
//   * `content_stream`       — page operator parse
//   * `to_unicode`           — /ToUnicode CMap decode
//   * `encoding_tables` + `agl` — /Encoding+AGL fallback path
//   * `crypt_filter`         — V=4 R=4 AES-128 stream decrypt
//                              (parse_with_key only)
//
// Scope (v1):
//   * Tj / TJ / `'` / `"` text-showing operators only.
//   * Decoder priority: /ToUnicode > /Encoding+AGL > Latin-1.
//   * /Differences override applied on top of /BaseEncoding.
//   * /Filter chain restricted to /FlateDecode (single Name or
//     all-FlateDecode Array).
//   * Pages-tree preorder; inline streams + array of refs both
//     accepted for /Contents.
//   * Out-of-scope: coordinate-aware reading order, Form XObjects
//     (Do), kerning-driven space synthesis, SymbolEncoding /
//     ZapfDingbatsEncoding / MacExpertEncoding, Type3 fonts,
//     CIDFontType2 with /CIDToGIDMap, vertical writing mode.

use std::collections::BTreeMap;
use std::fmt;

use super::content_stream;
use super::crypt_filter;
use super::encoding_tables;
use super::flate;
use super::objects::{self, IndirectObject, Value};
use super::to_unicode;
use super::xref::{self, Entry};

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    Objects(objects::Error),
    Xref(xref::Error),
    Flate(flate::Error),
    ContentStream(content_stream::Error),
    ToUnicode(to_unicode::Error),
    CryptFilter(crypt_filter::Error),
    MissingCatalog,
    CatalogMissingPages,
    PagesNodeNotDict,
    PagesNodeMissingType,
    PagesNodeUnknownType(Vec<u8>),
    PagesNodeMissingKids,
    KidsNotArray,
    ContentsNotStreamOrArray,
    ContentsArrayElementNotStream,
    ResourcesNotDict,
    FontResourcesNotDict,
    FontEntryNotDict,
    ToUnicodeNotStream,
    UnsupportedFilter(Vec<u8>),
    FilterArrayElementNotName,
    FilterNotNameOrArray,
    UnresolvedRef { id: u32, gen: u16 },
    RefChainTooDeep,
    DifferencesSlotOutOfRange { slot: i64 },
    DifferencesEntryWrongType,
    DifferencesNameBeforeInteger,
    EncryptedRequiresKey,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Objects(e) => write!(f, "text_extractor: {e}"),
            Error::Xref(e) => write!(f, "text_extractor: {e}"),
            Error::Flate(e) => write!(f, "text_extractor: {e}"),
            Error::ContentStream(e) => write!(f, "text_extractor: {e}"),
            Error::ToUnicode(e) => write!(f, "text_extractor: {e}"),
            Error::CryptFilter(e) => write!(f, "text_extractor: {e}"),
            Error::MissingCatalog =>
                write!(f, "text_extractor: PDF has no /Type /Catalog object"),
            Error::CatalogMissingPages =>
                write!(f, "text_extractor: catalog missing /Pages entry"),
            Error::PagesNodeNotDict =>
                write!(f, "text_extractor: page-tree node is not a dictionary"),
            Error::PagesNodeMissingType =>
                write!(f, "text_extractor: page-tree node missing /Type"),
            Error::PagesNodeUnknownType(t) => write!(
                f, "text_extractor: page-tree node /Type /{} unrecognised",
                String::from_utf8_lossy(t)),
            Error::PagesNodeMissingKids =>
                write!(f, "text_extractor: intermediate /Pages node missing /Kids"),
            Error::KidsNotArray =>
                write!(f, "text_extractor: /Kids is not an array"),
            Error::ContentsNotStreamOrArray =>
                write!(f, "text_extractor: /Contents is neither a Stream nor an Array"),
            Error::ContentsArrayElementNotStream =>
                write!(f, "text_extractor: /Contents array element is not a Stream"),
            Error::ResourcesNotDict =>
                write!(f, "text_extractor: /Resources is not a dictionary"),
            Error::FontResourcesNotDict =>
                write!(f, "text_extractor: /Resources/Font is not a dictionary"),
            Error::FontEntryNotDict =>
                write!(f, "text_extractor: /Resources/Font/<name> is not a dictionary"),
            Error::ToUnicodeNotStream =>
                write!(f, "text_extractor: /ToUnicode is referenced but not a Stream"),
            Error::UnsupportedFilter(name) => write!(
                f, "text_extractor: unsupported /Filter /{} (v1 supports /FlateDecode only)",
                String::from_utf8_lossy(name)),
            Error::FilterArrayElementNotName =>
                write!(f, "text_extractor: /Filter array contains a non-Name"),
            Error::FilterNotNameOrArray =>
                write!(f, "text_extractor: /Filter is neither a Name nor an Array"),
            Error::UnresolvedRef { id, gen } => write!(
                f, "text_extractor: unresolved indirect reference {id} {gen} R"),
            Error::RefChainTooDeep =>
                write!(f, "text_extractor: indirect-reference chain exceeded depth 16"),
            Error::DifferencesSlotOutOfRange { slot } => write!(
                f, "text_extractor: /Differences slot {slot} out of byte range"),
            Error::DifferencesEntryWrongType =>
                write!(f, "text_extractor: /Differences entry is neither integer nor Name"),
            Error::DifferencesNameBeforeInteger =>
                write!(f, "text_extractor: /Differences begins with a Name; expected integer slot first"),
            Error::EncryptedRequiresKey =>
                write!(f, "text_extractor: encrypted PDF — call parse_with_key with the file_encryption_key"),
        }
    }
}

impl std::error::Error for Error {}

impl From<objects::Error> for Error {
    fn from(e: objects::Error) -> Self { Error::Objects(e) }
}
impl From<xref::Error> for Error {
    fn from(e: xref::Error) -> Self { Error::Xref(e) }
}
impl From<flate::Error> for Error {
    fn from(e: flate::Error) -> Self { Error::Flate(e) }
}
impl From<content_stream::Error> for Error {
    fn from(e: content_stream::Error) -> Self { Error::ContentStream(e) }
}
impl From<to_unicode::Error> for Error {
    fn from(e: to_unicode::Error) -> Self { Error::ToUnicode(e) }
}
impl From<crypt_filter::Error> for Error {
    fn from(e: crypt_filter::Error) -> Self { Error::CryptFilter(e) }
}

// ============================================================
// Public API
// ============================================================

pub fn parse(input: &[u8]) -> Result<Vec<String>, Error> {
    let ctx = build_context(input, None, false)?;
    extract_all_pages(&ctx)
}

#[allow(dead_code)] // retained for a future-phase public-API binding
pub fn parse_with_key(input: &[u8], file_key: &[u8]) -> Result<Vec<String>, Error> {
    let ctx = build_context(input, Some(file_key.to_vec()), true)?;
    extract_all_pages(&ctx)
}

#[allow(dead_code)] // retained for a future-phase public-API binding
pub fn to_canonical(pages: &[String]) -> String {
    if pages.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    for (i, page) in pages.iter().enumerate() {
        if i > 0 {
            out.push('\u{000C}');
        }
        out.push_str(page);
    }
    out.push('\n');
    out
}

// ============================================================
// Context + dump
// ============================================================

struct ExtractContext<'a> {
    #[allow(dead_code)] // retained borrow of the source buffer
    source: &'a [u8],
    by_ref: BTreeMap<(u32, u16), IndirectObject>,
    file_key: Option<Vec<u8>>,
}

fn build_context<'a>(
    input: &'a [u8],
    file_key: Option<Vec<u8>>,
    allow_encrypted: bool,
) -> Result<ExtractContext<'a>, Error> {
    let by_ref = build_dump(input)?;
    let encrypted = detect_encryption(&by_ref);
    if encrypted && !allow_encrypted {
        return Err(Error::EncryptedRequiresKey);
    }
    Ok(ExtractContext { source: input, by_ref, file_key })
}

fn build_dump(input: &[u8]) -> Result<BTreeMap<(u32, u16), IndirectObject>, Error> {
    let table = xref::parse(input)?;
    let mut dump = BTreeMap::new();
    for entry in &table.entries {
        if let Entry::InUse { id, gen, offset } = *entry {
            // Skip /ObjStm-compressed entries (Compressed variant)
            // — the v1 fixture corpus is classical-trailer PDFs;
            // missing compressed objects surface as
            // UnresolvedRef downstream.
            let obj = objects::parse_object(input, offset as usize)?;
            dump.insert((id, gen), obj);
        }
    }
    Ok(dump)
}

// ============================================================
// Pages-tree walk
// ============================================================

fn extract_all_pages(ctx: &ExtractContext<'_>) -> Result<Vec<String>, Error> {
    let catalog_id = find_catalog(&ctx.by_ref)?;
    let catalog = ctx.by_ref.get(&catalog_id).expect("catalog id from dump");
    let catalog_dict = match &catalog.value {
        Value::Dict(entries) => entries,
        _ => return Err(Error::PagesNodeNotDict),
    };
    let pages_value = lookup(catalog_dict, b"Pages")
        .ok_or(Error::CatalogMissingPages)?;
    let mut page_refs: Vec<(u32, u16)> = Vec::new();
    walk_pages_tree(ctx, pages_value, &mut page_refs)?;

    let mut pages = Vec::with_capacity(page_refs.len());
    for r in &page_refs {
        pages.push(extract_page(ctx, *r)?);
    }
    Ok(pages)
}

fn find_catalog(
    by_ref: &BTreeMap<(u32, u16), IndirectObject>,
) -> Result<(u32, u16), Error> {
    for ((id, gen), obj) in by_ref {
        if let Value::Dict(entries) = &obj.value {
            if let Some(Value::Name(t)) = lookup(entries, b"Type") {
                if t == b"Catalog" {
                    return Ok((*id, *gen));
                }
            }
        }
    }
    Err(Error::MissingCatalog)
}

fn walk_pages_tree(
    ctx: &ExtractContext<'_>,
    node: &Value,
    out_refs: &mut Vec<(u32, u16)>,
) -> Result<(), Error> {
    let (resolved, owning_ref) = resolve_with_owning_ref(ctx, node)?;
    let dict = match resolved {
        Value::Dict(ref entries) => entries,
        _ => return Err(Error::PagesNodeNotDict),
    };
    let type_val = lookup(dict, b"Type").ok_or(Error::PagesNodeMissingType)?;
    let type_name = match type_val {
        Value::Name(b) => b,
        _ => return Err(Error::PagesNodeMissingType),
    };
    match type_name.as_slice() {
        b"Page" => {
            let r = owning_ref.ok_or(Error::PagesNodeNotDict)?;
            out_refs.push(r);
            Ok(())
        }
        b"Pages" => {
            let kids_value = lookup(dict, b"Kids")
                .ok_or(Error::PagesNodeMissingKids)?;
            let (kids_resolved, _) = resolve_with_owning_ref(ctx, kids_value)?;
            let kids = match kids_resolved {
                Value::Array(items) => items,
                _ => return Err(Error::KidsNotArray),
            };
            for kid in &kids {
                walk_pages_tree(ctx, kid, out_refs)?;
            }
            Ok(())
        }
        other => Err(Error::PagesNodeUnknownType(other.to_vec())),
    }
}

// ============================================================
// Per-page extraction
// ============================================================

fn extract_page(
    ctx: &ExtractContext<'_>,
    page_ref: (u32, u16),
) -> Result<String, Error> {
    let page = ctx.by_ref.get(&page_ref)
        .ok_or(Error::UnresolvedRef { id: page_ref.0, gen: page_ref.1 })?;
    let page_dict = match &page.value {
        Value::Dict(entries) => entries.clone(),
        _ => return Err(Error::PagesNodeNotDict),
    };

    let fonts = resolve_inherited_fonts(ctx, &page_dict)?;
    let content_bytes = assemble_contents(ctx, &page_dict)?;
    let ops = content_stream::parse(&content_bytes)?;
    run_page_state_machine(ctx, &ops, fonts.as_ref())
}

#[allow(clippy::type_complexity)] // Option<PDF dict entries> is inherent here
fn resolve_inherited_fonts(
    ctx: &ExtractContext<'_>,
    page_dict: &[(Vec<u8>, Value)],
) -> Result<Option<Vec<(Vec<u8>, Value)>>, Error> {
    let mut current: Vec<(Vec<u8>, Value)> = page_dict.to_vec();
    loop {
        if let Some(res_v) = lookup(&current, b"Resources") {
            let (resolved, _) = resolve_with_owning_ref(ctx, res_v)?;
            let resources = match resolved {
                Value::Dict(entries) => entries,
                _ => return Err(Error::ResourcesNotDict),
            };
            if let Some(font_v) = lookup(&resources, b"Font") {
                let (font_resolved, _) = resolve_with_owning_ref(ctx, font_v)?;
                let fonts = match font_resolved {
                    Value::Dict(entries) => entries,
                    _ => return Err(Error::FontResourcesNotDict),
                };
                return Ok(Some(fonts));
            }
            return Ok(None);
        }
        let parent_v = match lookup(&current, b"Parent") {
            Some(v) => v,
            None => return Ok(None),
        };
        let (parent_resolved, _) = resolve_with_owning_ref(ctx, parent_v)?;
        current = match parent_resolved {
            Value::Dict(entries) => entries,
            _ => return Err(Error::PagesNodeNotDict),
        };
    }
}

fn assemble_contents(
    ctx: &ExtractContext<'_>,
    page_dict: &[(Vec<u8>, Value)],
) -> Result<Vec<u8>, Error> {
    let contents_v = match lookup(page_dict, b"Contents") {
        Some(v) => v,
        None => return Ok(Vec::new()),
    };
    let (resolved, owning_ref) = resolve_with_owning_ref(ctx, contents_v)?;
    match resolved {
        Value::Stream { dict, body } => {
            decode_stream_payload(ctx, &dict, body, owning_ref)
        }
        Value::Array(items) => {
            let mut out = Vec::new();
            for item in &items {
                let (res, item_ref) = resolve_with_owning_ref(ctx, item)?;
                let (dict, body) = match res {
                    Value::Stream { dict, body } => (dict, body),
                    _ => return Err(Error::ContentsArrayElementNotStream),
                };
                let bytes = decode_stream_payload(
                    ctx, &dict, body, item_ref)?;
                out.extend_from_slice(&bytes);
                out.push(b'\n');
            }
            Ok(out)
        }
        _ => Err(Error::ContentsNotStreamOrArray),
    }
}

// ============================================================
// Stream decoding
// ============================================================

fn decode_stream_payload(
    ctx: &ExtractContext<'_>,
    dict: &[(Vec<u8>, Value)],
    body: Vec<u8>,
    src_ref: Option<(u32, u16)>,
) -> Result<Vec<u8>, Error> {
    // Body bytes already bounds-checked at parse time.
    let mut raw = body;

    if let (Some(key), Some(r)) = (ctx.file_key.as_ref(), src_ref) {
        raw = crypt_filter::decrypt(
            crypt_filter::Method::AesV2,
            key,
            r.0,
            r.1 as u32,
            &raw,
        )?;
    }

    if let Some(filter_v) = lookup(dict, b"Filter") {
        let (filter_resolved, _) = resolve_with_owning_ref(ctx, filter_v)?;
        match filter_resolved {
            Value::Name(name) => {
                if name != b"FlateDecode" {
                    return Err(Error::UnsupportedFilter(name));
                }
                raw = flate::decode(&raw)?;
            }
            Value::Array(items) => {
                for f in &items {
                    let (fr, _) = resolve_with_owning_ref(ctx, f)?;
                    match fr {
                        Value::Name(name) => {
                            if name != b"FlateDecode" {
                                return Err(Error::UnsupportedFilter(name));
                            }
                            raw = flate::decode(&raw)?;
                        }
                        _ => return Err(Error::FilterArrayElementNotName),
                    }
                }
            }
            _ => return Err(Error::FilterNotNameOrArray),
        }
    }

    Ok(raw)
}

// ============================================================
// Page state machine
// ============================================================

#[derive(Default)]
struct TextState {
    cmap: Option<to_unicode::ToUnicode>,
    encoding: Option<encoding_tables::Encoding>,
    differences: Vec<(u32, Vec<u8>)>,
}

fn run_page_state_machine(
    ctx: &ExtractContext<'_>,
    ops: &[content_stream::Operation],
    fonts: Option<&Vec<(Vec<u8>, Value)>>,
) -> Result<String, Error> {
    let mut state = TextState::default();
    let mut page = String::new();
    let mut block_emitted = false;

    for op in ops {
        match op.op.as_str() {
            "BT" => {
                block_emitted = false;
            }
            "ET" => {
                if block_emitted && !page.ends_with('\n') {
                    page.push('\n');
                }
            }
            "Tf" => {
                state = configure_font(ctx, op, fonts)?;
            }
            "Tj" => {
                if let Some(content_stream::Value::String(bytes)) =
                    op.operands.first()
                {
                    decode_and_append(&state, bytes, &mut page);
                    block_emitted = true;
                }
            }
            "'" => {
                if !page.is_empty() && !page.ends_with('\n') {
                    page.push('\n');
                }
                if let Some(content_stream::Value::String(bytes)) =
                    op.operands.first()
                {
                    decode_and_append(&state, bytes, &mut page);
                    block_emitted = true;
                }
            }
            "\"" => {
                if !page.is_empty() && !page.ends_with('\n') {
                    page.push('\n');
                }
                if let Some(content_stream::Value::String(bytes)) =
                    op.operands.get(2)
                {
                    decode_and_append(&state, bytes, &mut page);
                    block_emitted = true;
                }
            }
            "TJ" => {
                if let Some(content_stream::Value::Array(items)) =
                    op.operands.first()
                {
                    for item in items {
                        if let content_stream::Value::String(bytes) = item {
                            decode_and_append(&state, bytes, &mut page);
                            block_emitted = true;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    Ok(trim_trailing_whitespace(&page))
}

fn configure_font(
    ctx: &ExtractContext<'_>,
    tf: &content_stream::Operation,
    fonts: Option<&Vec<(Vec<u8>, Value)>>,
) -> Result<TextState, Error> {
    let mut state = TextState::default();
    let font_name = match tf.operands.first() {
        Some(content_stream::Value::Name(n)) => n,
        _ => return Ok(state),
    };
    let fonts = match fonts {
        Some(f) => f,
        None => return Ok(state),
    };
    let font_entry = match lookup(fonts, font_name) {
        Some(v) => v,
        None => return Ok(state),
    };
    let (resolved, _) = resolve_with_owning_ref(ctx, font_entry)?;
    let font_dict = match resolved {
        Value::Dict(entries) => entries,
        _ => return Err(Error::FontEntryNotDict),
    };

    if let Some(tu_v) = lookup(&font_dict, b"ToUnicode") {
        let (tu_resolved, tu_ref) = resolve_with_owning_ref(ctx, tu_v)?;
        let (dict, body) = match tu_resolved {
            Value::Stream { dict, body } => (dict, body),
            _ => return Err(Error::ToUnicodeNotStream),
        };
        let bytes = decode_stream_payload(
            ctx, &dict, body, tu_ref)?;
        state.cmap = Some(to_unicode::parse(&bytes)?);
        return Ok(state);
    }

    if let Some(enc_v) = lookup(&font_dict, b"Encoding") {
        let (enc_resolved, _) = resolve_with_owning_ref(ctx, enc_v)?;
        match enc_resolved {
            Value::Name(name) => {
                if let Ok(s) = std::str::from_utf8(&name) {
                    state.encoding = encoding_tables::from_name(s);
                }
            }
            Value::Dict(enc_dict) => {
                if let Some(base_v) = lookup(&enc_dict, b"BaseEncoding") {
                    let (base_res, _) = resolve_with_owning_ref(ctx, base_v)?;
                    if let Value::Name(name) = base_res {
                        if let Ok(s) = std::str::from_utf8(&name) {
                            state.encoding = encoding_tables::from_name(s);
                        }
                    }
                }
                if let Some(diff_v) = lookup(&enc_dict, b"Differences") {
                    let (diff_res, _) = resolve_with_owning_ref(ctx, diff_v)?;
                    if let Value::Array(items) = diff_res {
                        state.differences = parse_differences(&items)?;
                    }
                }
            }
            _ => {}
        }
    }
    Ok(state)
}

fn parse_differences(items: &[Value]) -> Result<Vec<(u32, Vec<u8>)>, Error> {
    let mut out = Vec::new();
    let mut cursor: i64 = 0;
    let mut cursor_set = false;
    for item in items {
        match item {
            Value::Integer(n) => {
                if *n < 0 || *n > 255 {
                    return Err(Error::DifferencesSlotOutOfRange { slot: *n });
                }
                cursor = *n;
                cursor_set = true;
            }
            Value::Name(n) => {
                if !cursor_set {
                    return Err(Error::DifferencesNameBeforeInteger);
                }
                if !(0..=255).contains(&cursor) {
                    return Err(Error::DifferencesSlotOutOfRange { slot: cursor });
                }
                out.push((cursor as u32, n.clone()));
                cursor += 1;
            }
            _ => return Err(Error::DifferencesEntryWrongType),
        }
    }
    Ok(out)
}

// ============================================================
// Byte → string decoder
// ============================================================

fn decode_and_append(state: &TextState, bytes: &[u8], out: &mut String) {
    if let Some(cmap) = &state.cmap {
        decode_with_cmap(cmap, bytes, out);
        return;
    }
    if state.encoding.is_some() || !state.differences.is_empty() {
        decode_with_encoding(state.encoding, &state.differences, bytes, out);
        return;
    }
    for &b in bytes {
        out.push(b as char);
    }
}

fn decode_with_cmap(cmap: &to_unicode::ToUnicode, bytes: &[u8], out: &mut String) {
    let mut i = 0;
    while i < bytes.len() {
        let width = resolve_code_width(cmap, bytes, i);
        if width == 0 {
            out.push(bytes[i] as char);
            i += 1;
            continue;
        }
        let mut code: u32 = 0;
        for j in 0..width {
            code = (code << 8) | (bytes[i + j] as u32);
        }
        let mapping = cmap.mappings.iter()
            .find(|m| m.byte_count as usize == width && m.char_code == code);
        match mapping {
            Some(m) => {
                for &cu in &m.codepoints {
                    if let Some(c) = char::from_u32(cu as u32) {
                        out.push(c);
                    }
                }
            }
            None => {
                for j in 0..width {
                    out.push(bytes[i + j] as char);
                }
            }
        }
        i += width;
    }
}

fn resolve_code_width(cmap: &to_unicode::ToUnicode, bytes: &[u8], offset: usize) -> usize {
    for &width in &[1usize, 2usize] {
        if offset + width > bytes.len() {
            continue;
        }
        let mut code: u32 = 0;
        for j in 0..width {
            code = (code << 8) | (bytes[offset + j] as u32);
        }
        for entry in &cmap.codespace {
            if entry.byte_count as usize != width {
                continue;
            }
            if code >= entry.start && code <= entry.end {
                return width;
            }
        }
    }
    0
}

fn decode_with_encoding(
    encoding: Option<encoding_tables::Encoding>,
    differences: &[(u32, Vec<u8>)],
    bytes: &[u8],
    out: &mut String,
) {
    for &b in bytes {
        let mut glyph: &[u8] = b"";
        for (slot, name) in differences {
            if *slot == b as u32 {
                glyph = name.as_slice();
                break;
            }
        }
        if glyph.is_empty() {
            if let Some(enc) = encoding {
                glyph = encoding_tables::byte_to_glyph_name(enc, b).as_bytes();
            }
        }
        if glyph.is_empty() {
            out.push(b as char);
            continue;
        }
        let glyph_str = match std::str::from_utf8(glyph) {
            Ok(s) => s,
            Err(_) => {
                out.push(b as char);
                continue;
            }
        };
        let cps = super::agl::lookup(glyph_str);
        if cps.is_empty() {
            out.push(b as char);
            continue;
        }
        for cp in cps {
            if let Some(c) = char::from_u32(cp) {
                out.push(c);
            }
        }
    }
}

// ============================================================
// Encryption discovery
// ============================================================

fn detect_encryption(by_ref: &BTreeMap<(u32, u16), IndirectObject>) -> bool {
    for obj in by_ref.values() {
        if let Value::Dict(entries) = &obj.value {
            let filter_is_standard = matches!(
                lookup(entries, b"Filter"),
                Some(Value::Name(n)) if n == b"Standard"
            );
            if filter_is_standard
                && lookup(entries, b"V").is_some()
                && lookup(entries, b"R").is_some()
            {
                return true;
            }
        }
    }
    false
}

// ============================================================
// Helpers
// ============================================================

/// Resolve `v` to its underlying value, dereferencing Refs up to
/// depth 16. Returns the resolved value AND the source Ref's
/// (id, gen) when the input was an indirect reference (used for
/// crypt_filter::decrypt obj_num/gen). Direct values yield None.
fn resolve_with_owning_ref(
    ctx: &ExtractContext<'_>,
    v: &Value,
) -> Result<(Value, Option<(u32, u16)>), Error> {
    let mut current = v.clone();
    let mut owning: Option<(u32, u16)> = None;
    for _ in 0..16 {
        match current {
            Value::Ref { id, gen } => {
                owning = Some((id, gen));
                let resolved = ctx.by_ref.get(&(id, gen))
                    .ok_or(Error::UnresolvedRef { id, gen })?;
                current = resolved.value.clone();
            }
            _ => return Ok((current, owning)),
        }
    }
    Err(Error::RefChainTooDeep)
}

fn lookup<'a>(dict: &'a [(Vec<u8>, Value)], key: &[u8]) -> Option<&'a Value> {
    dict.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

fn trim_trailing_whitespace(s: &str) -> String {
    let trimmed = s.trim_end_matches([' ', '\t', '\n', '\r', '\u{000C}']);
    trimmed.to_string()
}

// ============================================================
// Positioned extraction (text-fragment absorber support)
// ============================================================
//
// A coordinate-aware pass over the page content that the plain
// `parse` path deliberately omits (see the module header / spec).
// It tracks the CTM (q/Q/cm), the text + line matrices (Tm/Td/TD/
// T*), and the text-state scalars (Tf size, Tc, Tw, Tz), and emits
// one `PositionedRun` per text-showing operator carrying the run's
// decoded text, its device-space origin, font size, and the
// cumulative x of each character boundary (so a phrase search can
// carve a sub-rectangle out of a run). Glyph advances come from the
// font's /Widths array, falling back to the Standard-14 metric
// table and then a default advance. Simple (1-byte) fonts only;
// rotation / skew in the matrices is ignored (origin is exact, the
// advance uses the combined x-scale).

/// One text-showing operator's positioned output.
#[derive(Debug, Clone)]
pub(crate) struct PositionedRun {
    pub text: String,
    pub x: f64,
    pub y: f64,
    pub font_size: f64,
    /// Cumulative device-space x from `x`, one entry per character
    /// boundary; `char_x.len() == text.chars().count() + 1`.
    pub char_x: Vec<f64>,
}

/// Affine matrix `[a b c d e f]` (PDF row-vector convention:
/// `x' = a*x + c*y + e`, `y' = b*x + d*y + f`).
type Matrix = [f64; 6];

const IDENTITY: Matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

fn mat_mul(m: &Matrix, n: &Matrix) -> Matrix {
    [
        m[0] * n[0] + m[1] * n[2],
        m[0] * n[1] + m[1] * n[3],
        m[2] * n[0] + m[3] * n[2],
        m[2] * n[1] + m[3] * n[3],
        m[4] * n[0] + m[5] * n[2] + n[4],
        m[4] * n[1] + m[5] * n[3] + n[5],
    ]
}

fn mat_apply(m: &Matrix, x: f64, y: f64) -> (f64, f64) {
    (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5])
}

/// Per-page positioned runs in /Pages-tree preorder.
pub(crate) fn parse_positioned(
    input: &[u8],
) -> Result<Vec<Vec<PositionedRun>>, Error> {
    let ctx = build_context(input, None, false)?;
    let catalog_id = find_catalog(&ctx.by_ref)?;
    let catalog = ctx.by_ref.get(&catalog_id).expect("catalog id from dump");
    let catalog_dict = match &catalog.value {
        Value::Dict(entries) => entries,
        _ => return Err(Error::PagesNodeNotDict),
    };
    let pages_value =
        lookup(catalog_dict, b"Pages").ok_or(Error::CatalogMissingPages)?;
    let mut page_refs: Vec<(u32, u16)> = Vec::new();
    walk_pages_tree(&ctx, pages_value, &mut page_refs)?;

    let mut pages = Vec::with_capacity(page_refs.len());
    for r in &page_refs {
        pages.push(positioned_page(&ctx, *r)?);
    }
    Ok(pages)
}

/// Per-font width info for the positioned pass.
struct PosFont {
    state: TextState,
    base_font: String,
    first_char: i64,
    widths: Vec<u16>,
}

impl PosFont {
    /// Advance width (1000-em) of a 1-byte code.
    fn glyph_width(&self, code: u8) -> f64 {
        let c = code as i64;
        if c >= self.first_char {
            let idx = (c - self.first_char) as usize;
            if idx < self.widths.len() {
                return self.widths[idx] as f64;
            }
        }
        if let Some(w) = super::standard14_metrics::width(&self.base_font, code) {
            if w != 0 {
                return w as f64;
            }
        }
        // Unknown glyph — a middling advance keeps the run width
        // non-degenerate without claiming a specific metric.
        500.0
    }
}

fn positioned_page(
    ctx: &ExtractContext<'_>,
    page_ref: (u32, u16),
) -> Result<Vec<PositionedRun>, Error> {
    let page = ctx
        .by_ref
        .get(&page_ref)
        .ok_or(Error::UnresolvedRef { id: page_ref.0, gen: page_ref.1 })?;
    let page_dict = match &page.value {
        Value::Dict(entries) => entries.clone(),
        _ => return Err(Error::PagesNodeNotDict),
    };
    let fonts = resolve_inherited_fonts(ctx, &page_dict)?;
    let content_bytes = assemble_contents(ctx, &page_dict)?;
    let ops = content_stream::parse(&content_bytes)?;
    positioned_state_machine(ctx, &ops, fonts.as_ref())
}

fn positioned_state_machine(
    ctx: &ExtractContext<'_>,
    ops: &[content_stream::Operation],
    fonts: Option<&Vec<(Vec<u8>, Value)>>,
) -> Result<Vec<PositionedRun>, Error> {
    let mut runs = Vec::new();
    let mut ctm = IDENTITY;
    let mut ctm_stack: Vec<Matrix> = Vec::new();
    let mut tm = IDENTITY;
    let mut lm = IDENTITY;
    let mut font: Option<PosFont> = None;
    let mut font_size = 0.0_f64;
    let mut tc = 0.0_f64; // char spacing
    let mut tw = 0.0_f64; // word spacing
    let mut th = 100.0_f64; // horizontal scaling %
    let mut leading = 0.0_f64;

    let num = |v: Option<&content_stream::Value>| -> f64 {
        match v {
            Some(content_stream::Value::Integer(n)) => *n as f64,
            Some(content_stream::Value::Real(r)) => *r,
            _ => 0.0,
        }
    };

    for op in ops {
        let a = &op.operands;
        match op.op.as_str() {
            "q" => ctm_stack.push(ctm),
            "Q" => {
                if let Some(m) = ctm_stack.pop() {
                    ctm = m;
                }
            }
            "cm" => {
                let m: Matrix = [
                    num(a.first()),
                    num(a.get(1)),
                    num(a.get(2)),
                    num(a.get(3)),
                    num(a.get(4)),
                    num(a.get(5)),
                ];
                ctm = mat_mul(&m, &ctm);
            }
            "BT" => {
                tm = IDENTITY;
                lm = IDENTITY;
            }
            "Tf" => {
                font = Some(configure_pos_font(ctx, op, fonts)?);
                font_size = num(a.get(1));
            }
            "Tc" => tc = num(a.first()),
            "Tw" => tw = num(a.first()),
            "Tz" => th = num(a.first()),
            "TL" => leading = num(a.first()),
            "Td" => {
                let m = [1.0, 0.0, 0.0, 1.0, num(a.first()), num(a.get(1))];
                lm = mat_mul(&m, &lm);
                tm = lm;
            }
            "TD" => {
                leading = -num(a.get(1));
                let m = [1.0, 0.0, 0.0, 1.0, num(a.first()), num(a.get(1))];
                lm = mat_mul(&m, &lm);
                tm = lm;
            }
            "Tm" => {
                lm = [
                    num(a.first()),
                    num(a.get(1)),
                    num(a.get(2)),
                    num(a.get(3)),
                    num(a.get(4)),
                    num(a.get(5)),
                ];
                tm = lm;
            }
            "T*" => {
                let m = [1.0, 0.0, 0.0, 1.0, 0.0, -leading];
                lm = mat_mul(&m, &lm);
                tm = lm;
            }
            "'" => {
                let m = [1.0, 0.0, 0.0, 1.0, 0.0, -leading];
                lm = mat_mul(&m, &lm);
                tm = lm;
                if let Some(content_stream::Value::String(bytes)) = a.first() {
                    show_text(
                        bytes, &font, font_size, tc, tw, th, &ctm, &mut tm,
                        &mut runs,
                    );
                }
            }
            "\"" => {
                tw = num(a.first());
                tc = num(a.get(1));
                let m = [1.0, 0.0, 0.0, 1.0, 0.0, -leading];
                lm = mat_mul(&m, &lm);
                tm = lm;
                if let Some(content_stream::Value::String(bytes)) = a.get(2) {
                    show_text(
                        bytes, &font, font_size, tc, tw, th, &ctm, &mut tm,
                        &mut runs,
                    );
                }
            }
            "Tj" => {
                if let Some(content_stream::Value::String(bytes)) = a.first() {
                    show_text(
                        bytes, &font, font_size, tc, tw, th, &ctm, &mut tm,
                        &mut runs,
                    );
                }
            }
            "TJ" => {
                if let Some(content_stream::Value::Array(items)) = a.first() {
                    show_tj_array(
                        items, &font, font_size, tc, tw, th, &ctm, &mut tm,
                        &mut runs,
                    );
                }
            }
            _ => {}
        }
    }
    Ok(runs)
}

/// The x-scale factor of `text_matrix * CTM` — converts a text-space
/// advance into a device-space advance (rotation / skew ignored).
fn x_scale(tm: &Matrix, ctm: &Matrix) -> f64 {
    let m = mat_mul(tm, ctm);
    (m[0] * m[0] + m[1] * m[1]).sqrt()
}

#[allow(clippy::too_many_arguments)]
fn show_text(
    bytes: &[u8],
    font: &Option<PosFont>,
    font_size: f64,
    tc: f64,
    tw: f64,
    th: f64,
    ctm: &Matrix,
    tm: &mut Matrix,
    runs: &mut Vec<PositionedRun>,
) {
    let scale = x_scale(tm, ctm);
    let (ox, oy) = {
        let m = mat_mul(tm, ctm);
        mat_apply(&m, 0.0, 0.0)
    };
    let mut char_x = vec![0.0_f64];
    let mut text = String::new();
    let mut advance_textspace = 0.0_f64; // un-scaled, for tm update
    let mut cum_device = 0.0_f64;

    for &code in bytes {
        // Decode this single code to its character(s).
        let mut piece = String::new();
        match font {
            Some(f) => decode_and_append(&f.state, &[code], &mut piece),
            None => piece.push(code as char),
        }
        let gw = font.as_ref().map_or(500.0, |f| f.glyph_width(code));
        let tx = (gw / 1000.0 * font_size + tc + if code == b' ' { tw } else { 0.0 })
            * (th / 100.0);
        advance_textspace += tx;
        let dev_adv = tx * scale;

        let nchars = piece.chars().count().max(1);
        let per = dev_adv / nchars as f64;
        for ch in piece.chars() {
            text.push(ch);
            cum_device += per;
            char_x.push(cum_device);
        }
    }

    // Advance the text matrix by the run's total text-space width.
    let adv = [1.0, 0.0, 0.0, 1.0, advance_textspace, 0.0];
    *tm = mat_mul(&adv, tm);

    if !text.is_empty() {
        runs.push(PositionedRun { text, x: ox, y: oy, font_size, char_x });
    }
}

#[allow(clippy::too_many_arguments)]
fn show_tj_array(
    items: &[content_stream::Value],
    font: &Option<PosFont>,
    font_size: f64,
    tc: f64,
    tw: f64,
    th: f64,
    ctm: &Matrix,
    tm: &mut Matrix,
    runs: &mut Vec<PositionedRun>,
) {
    // A TJ array becomes one run; numeric entries adjust the inter-
    // glyph advance (tx -= n/1000 * size * th).
    let scale = x_scale(tm, ctm);
    let (ox, oy) = {
        let m = mat_mul(tm, ctm);
        mat_apply(&m, 0.0, 0.0)
    };
    let mut char_x = vec![0.0_f64];
    let mut text = String::new();
    let mut advance_textspace = 0.0_f64;
    let mut cum_device = 0.0_f64;

    for item in items {
        match item {
            content_stream::Value::String(bytes) => {
                for &code in bytes {
                    let mut piece = String::new();
                    match font {
                        Some(f) => decode_and_append(&f.state, &[code], &mut piece),
                        None => piece.push(code as char),
                    }
                    let gw = font.as_ref().map_or(500.0, |f| f.glyph_width(code));
                    let tx = (gw / 1000.0 * font_size
                        + tc
                        + if code == b' ' { tw } else { 0.0 })
                        * (th / 100.0);
                    advance_textspace += tx;
                    let nchars = piece.chars().count().max(1);
                    let per = tx * scale / nchars as f64;
                    for ch in piece.chars() {
                        text.push(ch);
                        cum_device += per;
                        char_x.push(cum_device);
                    }
                }
            }
            content_stream::Value::Integer(n) => {
                let tx = -(*n as f64) / 1000.0 * font_size * (th / 100.0);
                advance_textspace += tx;
                cum_device += tx * scale;
            }
            content_stream::Value::Real(r) => {
                let tx = -*r / 1000.0 * font_size * (th / 100.0);
                advance_textspace += tx;
                cum_device += tx * scale;
            }
            _ => {}
        }
    }

    let adv = [1.0, 0.0, 0.0, 1.0, advance_textspace, 0.0];
    *tm = mat_mul(&adv, tm);

    if !text.is_empty() {
        runs.push(PositionedRun { text, x: ox, y: oy, font_size, char_x });
    }
}

/// Resolve a `Tf` font into decode state + width info.
fn configure_pos_font(
    ctx: &ExtractContext<'_>,
    tf: &content_stream::Operation,
    fonts: Option<&Vec<(Vec<u8>, Value)>>,
) -> Result<PosFont, Error> {
    let state = configure_font(ctx, tf, fonts)?;
    let mut pos = PosFont {
        state,
        base_font: String::new(),
        first_char: 0,
        widths: Vec::new(),
    };

    let font_name = match tf.operands.first() {
        Some(content_stream::Value::Name(n)) => n,
        _ => return Ok(pos),
    };
    let fonts = match fonts {
        Some(f) => f,
        None => return Ok(pos),
    };
    let font_entry = match lookup(fonts, font_name) {
        Some(v) => v,
        None => return Ok(pos),
    };
    let (resolved, _) = resolve_with_owning_ref(ctx, font_entry)?;
    let font_dict = match resolved {
        Value::Dict(entries) => entries,
        _ => return Ok(pos),
    };

    if let Some(Value::Name(bf)) = lookup(&font_dict, b"BaseFont") {
        pos.base_font = String::from_utf8_lossy(bf).into_owned();
    }
    if let Some(fc_v) = lookup(&font_dict, b"FirstChar") {
        if let (Value::Integer(fc), _) = resolve_with_owning_ref(ctx, fc_v)? {
            pos.first_char = fc;
        }
    }
    if let Some(w_v) = lookup(&font_dict, b"Widths") {
        if let (Value::Array(items), _) = resolve_with_owning_ref(ctx, w_v)? {
            pos.widths = items
                .iter()
                .map(|it| match it {
                    Value::Integer(n) => (*n).clamp(0, u16::MAX as i64) as u16,
                    Value::Real(r) => r.round().clamp(0.0, u16::MAX as f64) as u16,
                    _ => 0,
                })
                .collect();
        }
    }
    Ok(pos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_canonical_empty_pages_is_empty() {
        assert_eq!(to_canonical(&[]), "");
    }


    #[test]
    fn to_canonical_single_page_appends_newline() {
        assert_eq!(
            to_canonical(&["hello".to_string()]),
            "hello\n",
        );
    }

    #[test]
    fn to_canonical_joins_pages_with_form_feed() {
        let pages = vec!["one".to_string(), "two".to_string()];
        let canon = to_canonical(&pages);
        assert_eq!(canon.as_bytes(),
                   &[b'o', b'n', b'e', 0x0C, b't', b'w', b'o', 0x0A]);
    }

    #[test]
    fn parse_differences_preorder() {
        let items = vec![
            Value::Integer(128),
            Value::Name(b"Euro".to_vec()),
            Value::Name(b"quotesinglbase".to_vec()),
            Value::Integer(153),
            Value::Name(b"trademark".to_vec()),
        ];
        let result = parse_differences(&items).unwrap();
        assert_eq!(result, vec![
            (128, b"Euro".to_vec()),
            (129, b"quotesinglbase".to_vec()),
            (153, b"trademark".to_vec()),
        ]);
    }

    #[test]
    fn parse_differences_rejects_oob_slot() {
        let items = vec![Value::Integer(-1)];
        let err = parse_differences(&items).unwrap_err();
        assert!(matches!(err, Error::DifferencesSlotOutOfRange { .. }));
    }

    #[test]
    fn parse_differences_rejects_name_first() {
        let items = vec![Value::Name(b"Euro".to_vec())];
        let err = parse_differences(&items).unwrap_err();
        assert!(matches!(err, Error::DifferencesNameBeforeInteger));
    }
}
