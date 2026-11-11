// Incremental PDF update writer (PDF 32000-1:2008 §7.5.6).
// Pure function from (original_bytes, dirty_objects) to a new
// byte buffer that is the original verbatim followed by an
// incremental update section: re-emitted indirect objects, a
// fresh classical xref, and a new trailer whose /Prev points at
// the original startxref so xref::parse follows the chain back.
//
// v1 scope:
//   * Classical xref + classical trailer only.
//   * Dirty value MUST NOT be a Stream — caller materialises a
//     non-stream replacement (e.g. an updated /Info dict).
//   * No encryption on the appended section (original may be
//     encrypted; appended bytes are plaintext).
//   * Duplicate dirty ids reject up front.
//
// Composes `lexer`, `xref`, `objects`. The Rust objects API
// only exposes `parse_object(input, offset)`; to materialise
// the original trailer dict for /Root + /Info + /ID, this
// primitive wraps the trailer's `<<...>>` bytes in a synthetic
// `1 0 obj ... endobj` buffer and calls parse_object on it.

use std::fmt;

use super::objects::{self, IndirectObject, Value};

#[derive(Debug, Clone, PartialEq)]
pub struct DirtyObject {
    pub id: u32,
    pub generation: u16,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    Objects(objects::Error),
    DuplicateDirtyId(u32),
    // A Stream-variant Value appeared as a direct child of
    // another array / dict (PDF §7.3.8.1 forbids direct
    // streams). The u32 is the enclosing dirty object's id.
    NonTopLevelStream(u32),
    MissingStartxref,
    StartxrefHasNoOffset,
    MissingTrailerKeyword,
    TrailerDictOpenNotFound,
    TrailerDictCloseNotFound,
    TrailerNotDict,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Objects(e) =>
                write!(f, "pdf_writer_incremental: {e}"),
            Error::DuplicateDirtyId(id) =>
                write!(f, "pdf_writer_incremental: duplicate dirty id {id}"),
            Error::NonTopLevelStream(id) => write!(
                f, "pdf_writer_incremental: stream value inside dirty object {id} \
                appears as a direct child (PDF §7.3.8.1)"),
            Error::MissingStartxref =>
                write!(f, "pdf_writer_incremental: original missing startxref"),
            Error::StartxrefHasNoOffset =>
                write!(f, "pdf_writer_incremental: startxref has no integer offset"),
            Error::MissingTrailerKeyword =>
                write!(f, "pdf_writer_incremental: original missing trailer keyword"),
            Error::TrailerDictOpenNotFound =>
                write!(f, "pdf_writer_incremental: trailer keyword not followed by <<"),
            Error::TrailerDictCloseNotFound =>
                write!(f, "pdf_writer_incremental: trailer dict is unterminated"),
            Error::TrailerNotDict =>
                write!(f, "pdf_writer_incremental: trailer is not a dictionary"),
        }
    }
}

impl std::error::Error for Error {}

impl From<objects::Error> for Error {
    fn from(e: objects::Error) -> Self { Error::Objects(e) }
}

/// Append an incremental update section to `original`.
///
/// `dirty` is a list of (id, generation, value) entries; each
/// entry replaces (or adds) the corresponding indirect object.
/// Duplicate ids reject. Stream-typed values reject.
pub fn append_incremental(
    original: &[u8],
    dirty: &[DirtyObject],
) -> Result<Vec<u8>, Error> {
    validate_dirty(dirty)?;

    let orig_startxref = find_original_startxref(original)?;
    let trailer_dict = parse_original_trailer(original)?;
    let orig_size = read_size(&trailer_dict);

    // Sort dirty by id so consecutive runs collapse into one
    // xref subsection.
    let mut sorted: Vec<&DirtyObject> = dirty.iter().collect();
    sorted.sort_by_key(|d| d.id);

    let mut out: Vec<u8> = Vec::with_capacity(original.len() + 256);
    out.extend_from_slice(original);

    // Ensure the original ends with a newline so the appended
    // section starts on a fresh line.
    if !out.is_empty() && out.last() != Some(&b'\n') && out.last() != Some(&b'\r') {
        out.push(b'\n');
    }

    // Emit each dirty body, recording byte offsets. v1.1: a
    // top-level Stream value is emitted with /Length auto-
    // injected; other variants go through serialize_value.
    let mut offsets: Vec<(u32, u16, usize)> = Vec::with_capacity(sorted.len());
    for d in &sorted {
        let off = out.len();
        offsets.push((d.id, d.generation, off));
        write_ascii(&mut out, &format!("{} {} obj\n", d.id, d.generation));
        match &d.value {
            Value::Stream { dict, body } => {
                serialize_stream(&mut out, dict, body);
            }
            other => serialize_value(&mut out, other),
        }
        write_ascii(&mut out, "\nendobj\n");
    }

    // Emit the xref subsections.
    let new_xref_offset = out.len();
    write_ascii(&mut out, "xref\n0 1\n0000000000 65535 f \n");

    let mut max_id: u32 = 0;
    let mut i = 0;
    while i < offsets.len() {
        let mut j = i + 1;
        while j < offsets.len()
            && offsets[j].0 == offsets[j - 1].0 + 1
        {
            j += 1;
        }
        let first_id = offsets[i].0;
        let run_len = j - i;
        write_ascii(&mut out, &format!("{first_id} {run_len}\n"));
        for &(_, gen, off) in &offsets[i..j] {
            write_ascii(&mut out, &format!("{off:010} {gen:05} n \n"));
        }
        if offsets[j - 1].0 > max_id {
            max_id = offsets[j - 1].0;
        }
        i = j;
    }

    // Compute new /Size: max(orig_size, max_dirty_id+1).
    let new_size = std::cmp::max(orig_size, max_id as i64 + 1);

    // Emit the new trailer. We rebuild it inline rather than
    // re-serialising the parsed dict so /Size + /Prev win
    // even if the original carried different values.
    write_ascii(&mut out, &format!("trailer\n<< /Size {new_size} /Prev {orig_startxref}"));
    for key in [b"Root".as_slice(), b"Info".as_slice(), b"ID".as_slice()] {
        if let Some(v) = lookup(&trailer_dict, key) {
            write_ascii(&mut out, " /");
            out.extend_from_slice(key);
            write_ascii(&mut out, " ");
            serialize_value(&mut out, v);
        }
    }
    write_ascii(&mut out, " >>\n");
    write_ascii(&mut out, &format!("startxref\n{new_xref_offset}\n%%EOF\n"));

    Ok(out)
}

// ============================================================
// Validation
// ============================================================

fn validate_dirty(dirty: &[DirtyObject]) -> Result<(), Error> {
    let mut seen: Vec<u32> = Vec::with_capacity(dirty.len());
    for d in dirty {
        if seen.contains(&d.id) {
            return Err(Error::DuplicateDirtyId(d.id));
        }
        seen.push(d.id);
        // v1.1: top-level Stream is OK (emitted via
        // serialize_stream). Nested Streams reject — PDF
        // §7.3.8.1 forbids direct streams. Recurse the value
        // graph EXCEPT for the top-level Stream wrapper itself.
        match &d.value {
            Value::Stream { dict, .. } => {
                check_no_nested_stream_in_dict(dict, d.id)?;
            }
            other => check_no_nested_stream(other, d.id)?,
        }
    }
    Ok(())
}

fn check_no_nested_stream(v: &Value, owner_id: u32) -> Result<(), Error> {
    match v {
        Value::Stream { .. } => Err(Error::NonTopLevelStream(owner_id)),
        Value::Array(items) => {
            for item in items {
                check_no_nested_stream(item, owner_id)?;
            }
            Ok(())
        }
        Value::Dict(entries) => check_no_nested_stream_in_dict(entries, owner_id),
        _ => Ok(()),
    }
}

fn check_no_nested_stream_in_dict(
    entries: &[(Vec<u8>, Value)],
    owner_id: u32,
) -> Result<(), Error> {
    for (_k, v) in entries {
        check_no_nested_stream(v, owner_id)?;
    }
    Ok(())
}

// ============================================================
// Original trailer parsing
// ============================================================

fn find_original_startxref(input: &[u8]) -> Result<u64, Error> {
    let pos = find_last(input, b"startxref")
        .ok_or(Error::MissingStartxref)?;
    let mut p = pos + b"startxref".len();
    while p < input.len() && is_ws_or_eol(input[p]) {
        p += 1;
    }
    let mut n: u64 = 0;
    let mut any = false;
    while p < input.len() && input[p].is_ascii_digit() {
        n = n * 10 + (input[p] - b'0') as u64;
        any = true;
        p += 1;
    }
    if !any {
        return Err(Error::StartxrefHasNoOffset);
    }
    Ok(n)
}

fn parse_original_trailer(input: &[u8]) -> Result<Vec<(Vec<u8>, Value)>, Error> {
    let tr = find_last(input, b"trailer")
        .ok_or(Error::MissingTrailerKeyword)?;

    // Locate `<<` after `trailer`.
    let mut i = tr + b"trailer".len();
    while i + 1 < input.len() {
        if input[i] == b'<' && input[i + 1] == b'<' {
            break;
        }
        i += 1;
    }
    if i + 1 >= input.len() || !(input[i] == b'<' && input[i + 1] == b'<') {
        return Err(Error::TrailerDictOpenNotFound);
    }
    let dict_start = i;

    // Walk to matching `>>`, accounting for nesting.
    let mut depth: i32 = 0;
    let mut j = i;
    let mut close: Option<usize> = None;
    while j + 1 < input.len() {
        if input[j] == b'<' && input[j + 1] == b'<' {
            depth += 1;
            j += 2;
            continue;
        }
        if input[j] == b'>' && input[j + 1] == b'>' {
            depth -= 1;
            j += 2;
            if depth == 0 {
                close = Some(j);
                break;
            }
            continue;
        }
        j += 1;
    }
    let dict_end = close.ok_or(Error::TrailerDictCloseNotFound)?;
    let dict_bytes = &input[dict_start..dict_end];

    // Wrap in `1 0 obj <<...>> endobj` and reparse via the
    // frozen objects::parse_object. The wrapper goes through
    // the lexer normally — no xref required.
    let mut wrap: Vec<u8> = Vec::with_capacity(dict_bytes.len() + 32);
    write_ascii(&mut wrap, "1 0 obj\n");
    wrap.extend_from_slice(dict_bytes);
    write_ascii(&mut wrap, "\nendobj\n");
    let obj: IndirectObject = objects::parse_object(&wrap, 0)?;
    match obj.value {
        Value::Dict(entries) => Ok(entries),
        _ => Err(Error::TrailerNotDict),
    }
}

fn read_size(trailer: &[(Vec<u8>, Value)]) -> i64 {
    match lookup(trailer, b"Size") {
        Some(Value::Integer(n)) => *n,
        _ => 0,
    }
}

// ============================================================
// Value serialiser — inverse of objects::parse_object
// ============================================================

fn serialize_value(out: &mut Vec<u8>, v: &Value) {
    match v {
        Value::Null => write_ascii(out, "null"),
        Value::Bool(b) => write_ascii(out, if *b { "true" } else { "false" }),
        Value::Integer(n) => write_ascii(out, &n.to_string()),
        Value::Real(x) => write_ascii(out, &format_real(*x)),
        Value::Name(bytes) => serialize_name(out, bytes),
        Value::String(bytes) => serialize_literal_string(out, bytes),
        Value::Array(items) => {
            if items.is_empty() {
                write_ascii(out, "[ ]");
            } else {
                write_ascii(out, "[ ");
                for (idx, item) in items.iter().enumerate() {
                    if idx > 0 {
                        out.push(b' ');
                    }
                    serialize_value(out, item);
                }
                write_ascii(out, " ]");
            }
        }
        Value::Dict(entries) => serialize_dict(out, entries),
        Value::Ref { id, gen } => {
            write_ascii(out, &format!("{id} {gen} R"));
        }
        Value::Stream { .. } => {
            // Unreachable: validate_dirty pre-rejects nested
            // Streams with Error::NonTopLevelStream, and the
            // top-level Stream branch dispatches to
            // serialize_stream directly (never reaching here).
            unreachable!("nested Stream should have been rejected by validate_dirty");
        }
    }
}

// Emit a top-level stream object: augmented header (with
// /Length overridden to body.len() for honesty) + the
// `\nstream\n<body>\nendstream` framing.
fn serialize_stream(out: &mut Vec<u8>, dict: &[(Vec<u8>, Value)], body: &[u8]) {
    let mut augmented: Vec<(Vec<u8>, Value)> =
        Vec::with_capacity(dict.len() + 1);
    let mut length_set = false;
    for (k, v) in dict {
        if k == b"Length" {
            augmented.push((k.clone(),
                Value::Integer(body.len() as i64)));
            length_set = true;
        } else {
            augmented.push((k.clone(), v.clone()));
        }
    }
    if !length_set {
        augmented.push((b"Length".to_vec(),
            Value::Integer(body.len() as i64)));
    }
    serialize_dict(out, &augmented);
    write_ascii(out, "\nstream\n");
    out.extend_from_slice(body);
    write_ascii(out, "\nendstream");
}

// PDF §7.3.5 — `/` + bytes; escape outside 0x21..0x7E and
// the delimiters `()<>[]{}/%#` as `#XX` (uppercase hex).
fn serialize_name(out: &mut Vec<u8>, bytes: &[u8]) {
    out.push(b'/');
    for &b in bytes {
        let printable = (0x21..=0x7E).contains(&b);
        let delim = matches!(b,
            b'(' | b')' | b'<' | b'>' | b'[' | b']' |
            b'{' | b'}' | b'/' | b'%' | b'#');
        if printable && !delim {
            out.push(b);
        } else {
            out.push(b'#');
            out.push(hex_nibble(b >> 4));
            out.push(hex_nibble(b & 0x0F));
        }
    }
}

fn hex_nibble(n: u8) -> u8 {
    if n < 10 { b'0' + n } else { b'A' + (n - 10) }
}

// PDF §7.3.4.2 literal-string form: `(...)` with the standard
// backslash escapes for `\\`, `(`, `)`, and the C-style EOL
// shorthands; non-printable bytes go to octal `\DDD`.
fn serialize_literal_string(out: &mut Vec<u8>, bytes: &[u8]) {
    out.push(b'(');
    for &b in bytes {
        match b {
            b'\\' => { out.push(b'\\'); out.push(b'\\'); }
            b'('  => { out.push(b'\\'); out.push(b'('); }
            b')'  => { out.push(b'\\'); out.push(b')'); }
            b'\n' => { out.push(b'\\'); out.push(b'n'); }
            b'\r' => { out.push(b'\\'); out.push(b'r'); }
            b'\t' => { out.push(b'\\'); out.push(b't'); }
            0x08  => { out.push(b'\\'); out.push(b'b'); }
            0x0C  => { out.push(b'\\'); out.push(b'f'); }
            _ if (0x20..=0x7E).contains(&b) => out.push(b),
            _ => {
                out.push(b'\\');
                out.push(b'0' + ((b >> 6) & 0x07));
                out.push(b'0' + ((b >> 3) & 0x07));
                out.push(b'0' + (b & 0x07));
            }
        }
    }
    out.push(b')');
}

// PDF §7.3.7 — `<<\n  /key value\n  /key value\n>>`.
// Empty dict: `<< >>`.
fn serialize_dict(out: &mut Vec<u8>, entries: &[(Vec<u8>, Value)]) {
    if entries.is_empty() {
        write_ascii(out, "<< >>");
        return;
    }
    write_ascii(out, "<<\n");
    for (k, v) in entries {
        write_ascii(out, "  ");
        serialize_name(out, k);
        out.push(b' ');
        serialize_value(out, v);
        out.push(b'\n');
    }
    write_ascii(out, ">>");
}

// Format an f64 as a PDF real: shortest fixed-point form with a
// `.`, no scientific notation, at least one digit on each side.
// Matches the C++ FormatReal contract (`%.6f` then trim trailing
// zeros to one fractional digit minimum).
fn format_real(x: f64) -> String {
    let mut s = format!("{x:.6}");
    if let Some(dot) = s.find('.') {
        let mut end = s.len();
        while end > dot + 2 && s.as_bytes()[end - 1] == b'0' {
            end -= 1;
        }
        s.truncate(end);
    } else {
        // %.6 always emits a decimal, but be defensive against
        // future format changes.
        s.push_str(".0");
    }
    s
}

// ============================================================
// Helpers
// ============================================================

fn write_ascii(out: &mut Vec<u8>, s: &str) {
    out.extend_from_slice(s.as_bytes());
}

fn lookup<'a>(dict: &'a [(Vec<u8>, Value)], key: &[u8]) -> Option<&'a Value> {
    dict.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

fn find_last(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    let mut i = haystack.len() - needle.len() + 1;
    while i > 0 {
        let pos = i - 1;
        if &haystack[pos..pos + needle.len()] == needle {
            return Some(pos);
        }
        i -= 1;
    }
    None
}

fn is_ws_or_eol(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0C | 0x00)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_simple_pdf(objs: &[&[u8]], trailer_inner: &str) -> Vec<u8> {
        let header = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n";
        let mut out = Vec::from(&header[..]);
        let mut offsets = Vec::new();
        for (i, body) in objs.iter().enumerate() {
            offsets.push(out.len());
            out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
            out.extend_from_slice(body);
            out.extend_from_slice(b"\nendobj\n");
        }
        let xref_pos = out.len();
        out.extend_from_slice(format!("xref\n0 {}\n", objs.len() + 1).as_bytes());
        out.extend_from_slice(b"0000000000 65535 f \n");
        for off in offsets {
            out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
        }
        out.extend_from_slice(format!("trailer\n<< {trailer_inner} >>\n").as_bytes());
        out.extend_from_slice(format!("startxref\n{xref_pos}\n%%EOF\n").as_bytes());
        out
    }

    #[test]
    fn duplicate_id_rejected() {
        let pdf = build_simple_pdf(
            &[b"<< /Type /Catalog /Pages 2 0 R >>",
              b"<< /Type /Pages /Kids [] /Count 0 >>"],
            "/Size 3 /Root 1 0 R");
        let dirty = vec![
            DirtyObject { id: 1, generation: 0, value: Value::Integer(0) },
            DirtyObject { id: 1, generation: 0, value: Value::Integer(1) },
        ];
        assert_eq!(
            append_incremental(&pdf, &dirty),
            Err(Error::DuplicateDirtyId(1)));
    }

    #[test]
    fn top_level_stream_round_trips() {
        // v1.1: top-level Stream is emitted with /Length auto-
        // injected. Re-parse object 3 directly via parse_object
        // (offset = location of "3 0 obj" in the appended tail).
        let pdf = build_simple_pdf(
            &[b"<< /Type /Catalog /Pages 2 0 R >>",
              b"<< /Type /Pages /Kids [] /Count 0 >>"],
            "/Size 3 /Root 1 0 R");
        let xmp_body = b"<?xpacket begin=\"\"?>\
<x:xmpmeta xmlns:x=\"adobe:ns:meta/\"/>\
<?xpacket end=\"w\"?>".to_vec();
        let dirty = vec![DirtyObject {
            id: 3,
            generation: 0,
            value: Value::Stream {
                dict: vec![
                    (b"Type".to_vec(), Value::Name(b"Metadata".to_vec())),
                    (b"Subtype".to_vec(), Value::Name(b"XML".to_vec())),
                ],
                body: xmp_body.clone(),
            },
        }];
        let updated = append_incremental(&pdf, &dirty).expect("append");
        let obj3_off = find_subslice(&updated, b"3 0 obj")
            .expect("obj 3 header in appended tail");
        let parsed = objects::parse_object(&updated, obj3_off)
            .expect("parse obj 3");
        match parsed.value {
            Value::Stream { dict, body } => {
                assert_eq!(body, xmp_body);
                let length = dict.iter()
                    .find_map(|(k, v)| if k == b"Length" {
                        if let Value::Integer(n) = v { Some(*n) } else { None }
                    } else { None })
                    .expect("/Length in stream dict");
                assert_eq!(length, xmp_body.len() as i64);
            }
            _ => panic!("expected Stream"),
        }
    }

    fn find_subslice(hay: &[u8], needle: &[u8]) -> Option<usize> {
        if needle.is_empty() || hay.len() < needle.len() { return None; }
        for i in 0..=(hay.len() - needle.len()) {
            if &hay[i..i + needle.len()] == needle { return Some(i); }
        }
        None
    }

    #[test]
    fn nested_stream_rejected() {
        // §7.3.8.1 forbids direct streams. A Stream value nested
        // inside a dict (not as the top-level DirtyObject.value)
        // returns NonTopLevelStream.
        let pdf = build_simple_pdf(
            &[b"<< /Type /Catalog /Pages 2 0 R >>",
              b"<< /Type /Pages /Kids [] /Count 0 >>"],
            "/Size 3 /Root 1 0 R");
        let nested = Value::Stream {
            dict: vec![],
            body: vec![0x78],
        };
        let outer = Value::Dict(vec![
            (b"Inner".to_vec(), nested),
        ]);
        let dirty = vec![DirtyObject {
            id: 3,
            generation: 0,
            value: outer,
        }];
        assert_eq!(
            append_incremental(&pdf, &dirty),
            Err(Error::NonTopLevelStream(3)));
    }

    #[test]
    fn missing_startxref_rejected() {
        let pdf = b"not a pdf at all";
        let dirty: Vec<DirtyObject> = vec![];
        assert_eq!(
            append_incremental(pdf, &dirty),
            Err(Error::MissingStartxref));
    }

    #[test]
    fn name_serialise_escapes_delimiters() {
        let mut out = Vec::new();
        serialize_name(&mut out, b"Title (test)");
        // Space inside a name escapes as #20; `(` as #28; `)` as #29.
        assert_eq!(out, b"/Title#20#28test#29");
    }

    #[test]
    fn literal_string_escapes_paren_and_backslash() {
        let mut out = Vec::new();
        serialize_literal_string(&mut out, b"a(b)c\\d\ne");
        assert_eq!(out, b"(a\\(b\\)c\\\\d\\ne)");
    }

    #[test]
    fn empty_array_renders_with_inner_space() {
        let mut out = Vec::new();
        serialize_value(&mut out, &Value::Array(vec![]));
        assert_eq!(out, b"[ ]");
    }

    #[test]
    fn empty_dict_renders_inline() {
        let mut out = Vec::new();
        serialize_value(&mut out, &Value::Dict(vec![]));
        assert_eq!(out, b"<< >>");
    }

    #[test]
    fn format_real_trims_trailing_zeros() {
        assert_eq!(format_real(5.0), "5.0");
        assert_eq!(format_real(0.5), "0.5");
        assert_eq!(format_real(1.25), "1.25");
        assert_eq!(format_real(-0.125), "-0.125");
    }
}
