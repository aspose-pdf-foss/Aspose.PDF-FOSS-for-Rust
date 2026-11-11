//! Document-wide encrypt / decrypt rewrite, per `document_encrypt.yaml`
//! and the Rust overlay beside it.
//!
//! Three entry points answer the anchor's contract: `encrypt` turns a
//! plaintext buffer into an encrypted one, `decrypt` reverses it against
//! a password, and `extract` reports the `/Encrypt` parameters without
//! authenticating. No cryptographic primitive work happens here — every
//! cipher / hash / key-derivation call delegates to `rc4`, `aes_cbc`,
//! `digest`, `crypt_filter`, `encrypt_parser` and `encrypt_writer`. This
//! module owns the document-wide bookkeeping: which objects carry
//! encryptable content, which key each object gets, and what the
//! `/Encrypt` object and trailer `/ID` look like.
//!
//! Recorded in the overlay: the salt / IV / R>=5 file-key inputs that
//! `encrypt_writer` expects from a CSPRNG come from a deterministic
//! SHA-256 counter generator seeded from the document bytes and
//! passwords, and `authenticate` / `permissions` / `is_encrypted` /
//! `recover_key` are target-side additions serving the public
//! `Document` and `PdfFileSecurity` surfaces.

use super::digest::sha256;
use super::encrypt_parser::{self, EncryptDict};
use super::encrypt_writer::{self, EncryptOutput};
use super::objects::{self, Value};
use super::{crypt_filter, trailer, xref};

/// The encryption parameters recovered from a PDF's `/Encrypt` dict.
pub(crate) struct EncryptInfo {
    pub enc: EncryptDict,
    pub file_id: Vec<u8>,
    pub p: i32,
}

/// Whether `bytes` is an encrypted PDF (`/Encrypt` present in the
/// trailer).
pub(crate) fn is_encrypted(bytes: &[u8]) -> bool {
    trailer::parse(bytes).map(|t| t.encrypt_id != 0).unwrap_or(false)
}

/// Extract the `/Encrypt` parameters from `bytes`, or `None` when the
/// document is not encrypted / the dict is unreadable.
pub(crate) fn extract(bytes: &[u8]) -> Option<EncryptInfo> {
    let tr = trailer::parse(bytes).ok()?;
    if tr.encrypt_id == 0 {
        return None;
    }
    let table = xref::parse(bytes).ok()?;
    let offset = table.entries.iter().find_map(|e| match e {
        xref::Entry::InUse { id, offset, .. } if *id == tr.encrypt_id => Some(*offset),
        _ => None,
    })?;
    let obj = objects::parse_object(bytes, offset as usize).ok()?;
    let dict = match obj.value {
        Value::Dict(d) => d,
        Value::Stream { dict, .. } => dict,
        _ => return None,
    };

    let v = dict_int(&dict, b"V").unwrap_or(0) as u32;
    let r = dict_int(&dict, b"R").unwrap_or(0) as u32;
    let length_bits = dict_int(&dict, b"Length").unwrap_or(40) as u32;
    let p = dict_int(&dict, b"P").unwrap_or(0) as i32;
    let o = dict_bytes(&dict, b"O")?;
    let u = dict_bytes(&dict, b"U")?;
    let oe = dict_bytes(&dict, b"OE").unwrap_or_default();
    let ue = dict_bytes(&dict, b"UE").unwrap_or_default();
    let perms = dict_bytes(&dict, b"Perms").unwrap_or_default();
    let encrypt_metadata = dict_bool(&dict, b"EncryptMetadata").unwrap_or(true);

    let enc = EncryptDict { v, r, length_bits, p, o, u, oe, ue, perms, encrypt_metadata };
    let file_id = extract_file_id(bytes).unwrap_or_default();
    Some(EncryptInfo { enc, file_id, p })
}

/// Authenticate `password` against the document's encryption.
///
/// * `Ok(false)` — the document is not encrypted (password ignored).
/// * `Ok(true)` — encrypted and the password is correct.
/// * `Err(())` — encrypted but the password is wrong / the dict is
///   unusable.
pub(crate) fn authenticate(bytes: &[u8], password: &[u8]) -> Result<bool, ()> {
    let Some(info) = extract(bytes) else {
        return Ok(false);
    };
    match encrypt_parser::authenticate(&info.enc, password, &info.file_id) {
        Ok(res) if res.authenticated => Ok(true),
        _ => Err(()),
    }
}

/// The document permission flags (`/Encrypt /P`), or `None` when plain.
pub(crate) fn permissions(bytes: &[u8]) -> Option<i32> {
    extract(bytes).map(|i| i.p)
}

/// Recover the document encryption key + per-object cipher method by
/// authenticating `password`. Returns `None` when the document is not
/// encrypted or the password is wrong.
pub(crate) fn recover_key(
    bytes: &[u8],
    password: &[u8],
) -> Option<(Vec<u8>, crypt_filter::Method)> {
    let info = extract(bytes)?;
    let res = encrypt_parser::authenticate(&info.enc, password, &info.file_id).ok()?;
    if !res.authenticated {
        return None;
    }
    // Per-object cipher: V<4 RC4, V=4 AES-128, V≥5 AES-256. (Files this
    // lib writes always pair V with its native cipher; for arbitrary
    // V=4 RC4 files the /CF /CFM would refine this — out of v1 scope.)
    let method = match info.enc.v {
        v if v >= 5 => crypt_filter::Method::AesV3,
        4 => crypt_filter::Method::AesV2,
        _ => crypt_filter::Method::V2,
    };
    Some((res.file_key, method))
}

fn dict_get<'a>(dict: &'a [(Vec<u8>, Value)], key: &[u8]) -> Option<&'a Value> {
    dict.iter().find(|(k, _)| k.as_slice() == key).map(|(_, v)| v)
}
fn dict_int(dict: &[(Vec<u8>, Value)], key: &[u8]) -> Option<i64> {
    match dict_get(dict, key)? {
        Value::Integer(n) => Some(*n),
        _ => None,
    }
}
fn dict_bytes(dict: &[(Vec<u8>, Value)], key: &[u8]) -> Option<Vec<u8>> {
    match dict_get(dict, key)? {
        Value::String(b) => Some(b.clone()),
        _ => None,
    }
}
fn dict_bool(dict: &[(Vec<u8>, Value)], key: &[u8]) -> Option<bool> {
    match dict_get(dict, key)? {
        Value::Bool(b) => Some(*b),
        _ => None,
    }
}

/// Extract the first `/ID` hex string from the trailer.
fn extract_file_id(bytes: &[u8]) -> Option<Vec<u8>> {
    let pos = find_last(bytes, b"/ID")?;
    let region = &bytes[pos..];
    let lt = region.iter().position(|&b| b == b'<')?;
    let gt = region[lt + 1..].iter().position(|&b| b == b'>')? + lt + 1;
    decode_hex(&region[lt + 1..gt])
}

fn find_last(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    (0..=haystack.len() - needle.len())
        .rev()
        .find(|&i| &haystack[i..i + needle.len()] == needle)
}

fn decode_hex(hex: &[u8]) -> Option<Vec<u8>> {
    let digits: Vec<u8> = hex.iter().copied().filter(|b| !b.is_ascii_whitespace()).collect();
    let mut out = Vec::with_capacity(digits.len() / 2);
    let mut iter = digits.chunks_exact(2);
    for pair in &mut iter {
        let hi = (pair[0] as char).to_digit(16)?;
        let lo = (pair[1] as char).to_digit(16)?;
        out.push(((hi << 4) | lo) as u8);
    }
    Some(out)
}


/// The staged-encryption parameters captured by `Document::encrypt`.
pub(crate) struct Params {
    pub user_password: String,
    pub owner_password: String,
    pub permissions: i32,
    /// `(v, r, length_bits, method)` selected from the crypto algorithm.
    pub variant: (u32, u32, u32, crypt_filter::Method),
    pub encrypt_metadata: bool,
}

/// Produce an encrypted PDF from plaintext `bytes` under `params`.
pub(crate) fn encrypt(bytes: &[u8], params: &Params) -> Option<Vec<u8>> {
    let (v, r, length_bits, method) = params.variant;

    // File /ID[0]: reuse the source's, else derive a stable 16 bytes.
    let file_id = source_file_id(bytes)
        .unwrap_or_else(|| sha256(bytes)[..16].to_vec());

    // Deterministic salt (CSPRNG stand-in).
    let mut prng = Prng::new(bytes, params.user_password.as_bytes(), params.owner_password.as_bytes());
    let salt = if r >= 5 { prng.bytes(68) } else { prng.bytes(16) };

    let out: EncryptOutput = encrypt_writer::build_encrypt_dict(
        params.user_password.as_bytes(),
        params.owner_password.as_bytes(),
        params.permissions,
        v,
        r,
        length_bits,
        &file_id,
        params.encrypt_metadata,
        &salt,
    )
    .ok()?;

    // Parse every in-use object.
    let table = xref::parse(bytes).ok()?;
    let tr = trailer::parse(bytes).ok()?;
    let mut objs: Vec<(u32, u16, Value)> = Vec::new();
    let mut max_id = 0u32;
    for entry in &table.entries {
        if let xref::Entry::InUse { id, gen, offset } = *entry {
            if let Ok(obj) = objects::parse_object(bytes, offset as usize) {
                max_id = max_id.max(id);
                objs.push((id, gen, obj.value));
            }
        }
    }
    if objs.is_empty() {
        return None;
    }

    // Encrypt strings + stream bodies of every object.
    for (id, gen, value) in &mut objs {
        encrypt_value(value, method, &out.file_key, *id, *gen as u32, &mut prng);
    }

    // The fresh /Encrypt object (its own strings are NOT encrypted).
    let encrypt_id = max_id + 1;
    let encrypt_obj = build_encrypt_object(&out, v, r, length_bits, params);

    Some(serialize(&objs, encrypt_id, &encrypt_obj, tr.root_id, tr.info_id, &file_id))
}

fn source_file_id(bytes: &[u8]) -> Option<Vec<u8>> {
    extract(bytes).map(|i| i.file_id).filter(|f| !f.is_empty())
}

/// Produce a plaintext PDF from an encrypted document, authenticating
/// `password`. Decrypts every string + stream body and re-serialises
/// without the `/Encrypt` object. Returns `None` when the document is
/// not encrypted / the password is wrong.
pub(crate) fn decrypt(bytes: &[u8], password: &[u8]) -> Option<Vec<u8>> {
    let (file_key, method) = recover_key(bytes, password)?;
    let table = xref::parse(bytes).ok()?;
    let tr = trailer::parse(bytes).ok()?;

    let mut objs: Vec<(u32, u16, Value)> = Vec::new();
    for entry in &table.entries {
        if let xref::Entry::InUse { id, gen, offset } = *entry {
            if id == tr.encrypt_id {
                continue; // drop the /Encrypt object
            }
            if let Ok(obj) = objects::parse_object(bytes, offset as usize) {
                objs.push((id, gen, obj.value));
            }
        }
    }
    if objs.is_empty() {
        return None;
    }
    for (id, gen, value) in &mut objs {
        decrypt_value(value, method, &file_key, *id, *gen as u32);
    }
    Some(serialize_plain(&objs, tr.root_id, tr.info_id))
}

fn decrypt_value(value: &mut Value, method: crypt_filter::Method, file_key: &[u8], id: u32, gen: u32) {
    match value {
        Value::String(b) => {
            if let Ok(plain) = crypt_filter::decrypt(method, file_key, id, gen, b) {
                *b = plain;
            }
        }
        Value::Array(items) => {
            for it in items {
                decrypt_value(it, method, file_key, id, gen);
            }
        }
        Value::Dict(entries) => {
            for (_, v) in entries {
                decrypt_value(v, method, file_key, id, gen);
            }
        }
        Value::Stream { dict, body } => {
            for (_, v) in dict {
                decrypt_value(v, method, file_key, id, gen);
            }
            if let Ok(plain) = crypt_filter::decrypt(method, file_key, id, gen, body) {
                *body = plain;
            }
        }
        _ => {}
    }
}

fn serialize_plain(objs: &[(u32, u16, Value)], root_id: u32, info_id: u32) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");
    let max_id = objs.iter().map(|(id, _, _)| *id).max().unwrap_or(0);
    let mut offsets: Vec<Option<usize>> = vec![None; (max_id + 1) as usize];
    for (id, gen, value) in objs {
        emit_obj(&mut out, *id, *gen, value, &mut offsets);
    }
    let xref_at = out.len();
    let size = max_id + 1;
    out.extend_from_slice(format!("xref\n0 {size}\n").as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for id in 1..size {
        match offsets[id as usize] {
            Some(off) => out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes()),
            None => out.extend_from_slice(b"0000000000 65535 f \n"),
        }
    }
    out.extend_from_slice(b"trailer\n<< /Size ");
    out.extend_from_slice(size.to_string().as_bytes());
    out.extend_from_slice(format!(" /Root {root_id} 0 R").as_bytes());
    if info_id != 0 {
        out.extend_from_slice(format!(" /Info {info_id} 0 R").as_bytes());
    }
    out.extend_from_slice(b" >>\nstartxref\n");
    out.extend_from_slice(xref_at.to_string().as_bytes());
    out.extend_from_slice(b"\n%%EOF\n");
    out
}

fn encrypt_value(
    value: &mut Value,
    method: crypt_filter::Method,
    file_key: &[u8],
    id: u32,
    gen: u32,
    prng: &mut Prng,
) {
    match value {
        Value::String(b) => {
            *b = crypt(method, file_key, id, gen, b, prng);
        }
        Value::Array(items) => {
            for it in items {
                encrypt_value(it, method, file_key, id, gen, prng);
            }
        }
        Value::Dict(entries) => {
            for (_, v) in entries {
                encrypt_value(v, method, file_key, id, gen, prng);
            }
        }
        Value::Stream { dict, body } => {
            for (_, v) in dict {
                encrypt_value(v, method, file_key, id, gen, prng);
            }
            *body = crypt(method, file_key, id, gen, body, prng);
        }
        _ => {}
    }
}

fn crypt(
    method: crypt_filter::Method,
    file_key: &[u8],
    id: u32,
    gen: u32,
    plaintext: &[u8],
    prng: &mut Prng,
) -> Vec<u8> {
    let iv = if matches!(method, crypt_filter::Method::V2) {
        Vec::new()
    } else {
        prng.bytes(16)
    };
    crypt_filter::encrypt(method, file_key, id, gen, plaintext, &iv)
        .unwrap_or_else(|_| plaintext.to_vec())
}

fn build_encrypt_object(
    out: &EncryptOutput,
    v: u32,
    r: u32,
    length_bits: u32,
    params: &Params,
) -> Value {
    let d = &out.encrypt_dict;
    let mut entries: Vec<(Vec<u8>, Value)> = vec![
        (b"Filter".to_vec(), Value::Name(b"Standard".to_vec())),
        (b"V".to_vec(), Value::Integer(v as i64)),
        (b"R".to_vec(), Value::Integer(r as i64)),
        (b"Length".to_vec(), Value::Integer(length_bits as i64)),
        (b"P".to_vec(), Value::Integer(d.p as i64)),
        (b"O".to_vec(), Value::String(d.o.clone())),
        (b"U".to_vec(), Value::String(d.u.clone())),
    ];
    if v >= 5 {
        entries.push((b"OE".to_vec(), Value::String(d.oe.clone())));
        entries.push((b"UE".to_vec(), Value::String(d.ue.clone())));
        entries.push((b"Perms".to_vec(), Value::String(d.perms.clone())));
    }
    if !params.encrypt_metadata {
        entries.push((b"EncryptMetadata".to_vec(), Value::Bool(false)));
    }
    if v >= 4 {
        let (cfm, cf_len): (&[u8], i64) =
            if v == 4 { (b"AESV2", 16) } else { (b"AESV3", 32) };
        let std_cf = Value::Dict(vec![
            (b"CFM".to_vec(), Value::Name(cfm.to_vec())),
            (b"Length".to_vec(), Value::Integer(cf_len)),
            (b"AuthEvent".to_vec(), Value::Name(b"DocOpen".to_vec())),
        ]);
        entries.push((b"CF".to_vec(), Value::Dict(vec![(b"StdCF".to_vec(), std_cf)])));
        entries.push((b"StmF".to_vec(), Value::Name(b"StdCF".to_vec())));
        entries.push((b"StrF".to_vec(), Value::Name(b"StdCF".to_vec())));
    }
    Value::Dict(entries)
}

fn serialize(
    objs: &[(u32, u16, Value)],
    encrypt_id: u32,
    encrypt_obj: &Value,
    root_id: u32,
    info_id: u32,
    file_id: &[u8],
) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");

    let max_id = encrypt_id;
    let mut offsets: Vec<Option<usize>> = vec![None; (max_id + 1) as usize];

    for (id, gen, value) in objs {
        emit_obj(&mut out, *id, *gen, value, &mut offsets);
    }
    emit_obj(&mut out, encrypt_id, 0, encrypt_obj, &mut offsets);

    // Cross-reference table.
    let xref_at = out.len();
    let size = max_id + 1;
    out.extend_from_slice(format!("xref\n0 {size}\n").as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for id in 1..size {
        match offsets[id as usize] {
            Some(off) => out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes()),
            None => out.extend_from_slice(b"0000000000 65535 f \n"),
        }
    }

    // Trailer.
    let id_hex = hex(file_id);
    out.extend_from_slice(b"trailer\n<< /Size ");
    out.extend_from_slice(size.to_string().as_bytes());
    out.extend_from_slice(format!(" /Root {root_id} 0 R").as_bytes());
    if info_id != 0 {
        out.extend_from_slice(format!(" /Info {info_id} 0 R").as_bytes());
    }
    out.extend_from_slice(format!(" /Encrypt {encrypt_id} 0 R").as_bytes());
    out.extend_from_slice(format!(" /ID [<{id_hex}><{id_hex}>] >>\n").as_bytes());
    out.extend_from_slice(b"startxref\n");
    out.extend_from_slice(xref_at.to_string().as_bytes());
    out.extend_from_slice(b"\n%%EOF\n");
    out
}

pub(crate) fn emit_obj(out: &mut Vec<u8>, id: u32, gen: u16, value: &Value, offsets: &mut [Option<usize>]) {
    offsets[id as usize] = Some(out.len());
    out.extend_from_slice(format!("{id} {gen} obj\n").as_bytes());
    serialize_value(out, value);
    out.extend_from_slice(b"\nendobj\n");
}

pub(crate) fn serialize_value(out: &mut Vec<u8>, value: &Value) {
    match value {
        Value::Null => out.extend_from_slice(b"null"),
        Value::Bool(b) => out.extend_from_slice(if *b { b"true" } else { b"false" }),
        Value::Integer(n) => out.extend_from_slice(n.to_string().as_bytes()),
        Value::Real(r) => out.extend_from_slice(format!("{r}").as_bytes()),
        Value::Name(n) => {
            out.push(b'/');
            out.extend_from_slice(n);
        }
        Value::String(s) => {
            // Emit as a hex string — robust for the binary ciphertext
            // an encrypted string carries.
            out.push(b'<');
            out.extend_from_slice(hex(s).as_bytes());
            out.push(b'>');
        }
        Value::Array(items) => {
            out.push(b'[');
            for (i, it) in items.iter().enumerate() {
                if i > 0 {
                    out.push(b' ');
                }
                serialize_value(out, it);
            }
            out.push(b']');
        }
        Value::Dict(entries) => serialize_dict(out, entries),
        Value::Ref { id, gen } => out.extend_from_slice(format!("{id} {gen} R").as_bytes()),
        Value::Stream { dict, body } => {
            let mut d = dict.clone();
            set_entry(&mut d, b"Length", Value::Integer(body.len() as i64));
            serialize_dict(out, &d);
            out.extend_from_slice(b"\nstream\n");
            out.extend_from_slice(body);
            out.extend_from_slice(b"\nendstream");
        }
    }
}

fn serialize_dict(out: &mut Vec<u8>, entries: &[(Vec<u8>, Value)]) {
    out.extend_from_slice(b"<<");
    for (k, v) in entries {
        out.push(b' ');
        out.push(b'/');
        out.extend_from_slice(k);
        out.push(b' ');
        serialize_value(out, v);
    }
    out.extend_from_slice(b" >>");
}

fn set_entry(entries: &mut Vec<(Vec<u8>, Value)>, key: &[u8], value: Value) {
    if let Some(slot) = entries.iter_mut().find(|(k, _)| k.as_slice() == key) {
        slot.1 = value;
    } else {
        entries.push((key.to_vec(), value));
    }
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// Deterministic SHA-256 counter generator — a CSPRNG stand-in for the
/// salt / IV / R≥5 file-key bytes the writer expects from randomness.
struct Prng {
    state: [u8; 32],
    counter: u64,
}

impl Prng {
    fn new(a: &[u8], b: &[u8], c: &[u8]) -> Self {
        let mut seed = Vec::with_capacity(a.len() + b.len() + c.len());
        seed.extend_from_slice(a);
        seed.extend_from_slice(b);
        seed.extend_from_slice(c);
        Prng { state: sha256(&seed), counter: 0 }
    }

    fn bytes(&mut self, n: usize) -> Vec<u8> {
        let mut out = Vec::with_capacity(n);
        while out.len() < n {
            let mut input = self.state.to_vec();
            input.extend_from_slice(&self.counter.to_le_bytes());
            self.counter += 1;
            out.extend_from_slice(&sha256(&input));
        }
        out.truncate(n);
        out
    }
}
