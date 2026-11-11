// XMP metadata parser (PDF 32000-1:2008 §14.3.2 / ISO 16684-1).
// Whole-buffer adapter: input is the raw XMP packet bytes
// (typically the contents of the document's /Metadata stream
// after filter decoding); output is a sorted property map ready
// to compare against the gate's `.canonical` sidecar.
//
// v1 scope: simple-text properties only — direct text under
// <rdf:Description> child elements, x-default rdf:Alt
// flattening, tab-joined rdf:Bag/Seq, and attribute-form
// properties on <rdf:Description>. Typed XMP (Date, GPSCoord,
// nested struct, qualified properties) is out of scope per the
// anchor.

use std::collections::BTreeMap;
use std::fmt;

/// Errors returned by [`parse`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Input is empty.
    EmptyInput,
    /// Input is not valid UTF-8 / declares UTF-16 (v1 only
    /// reads UTF-8 packets).
    NotUtf8,
    /// XML structure is broken — unmatched tag, unterminated
    /// element / processing instruction, broken entity ref.
    MalformedXml,
    /// `<rdf:RDF>` root element was not found.
    RdfRootMissing,
    /// `<?xpacket begin=...?>` was present but malformed.
    MalformedXpacketMarker,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::EmptyInput =>
                write!(f, "xmp: empty input"),
            Error::NotUtf8 =>
                write!(f, "xmp: input is not valid UTF-8"),
            Error::MalformedXml =>
                write!(f, "xmp: malformed XML"),
            Error::RdfRootMissing =>
                write!(f, "xmp: <rdf:RDF> root element missing"),
            Error::MalformedXpacketMarker =>
                write!(f, "xmp: malformed xpacket marker"),
        }
    }
}

impl std::error::Error for Error {}

// ============================================================
// Public API
// ============================================================

/// Output of `parse`: properties + namespace URI map. Both
/// are sorted (BTreeMap-backed → BTreeMap iteration order).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Parsed {
    /// `prefix:local-name → value` properties harvested from
    /// the rdf:Description (attribute-form and element-form).
    pub properties: Vec<(String, String)>,
    /// `prefix → uri` map harvested from every `xmlns:prefix`
    /// declaration in the packet. Lets the public Metadata
    /// wrapper populate its prefix registry so a subsequent
    /// save round-trips custom namespaces without falling back
    /// to the built-in default namespace map.
    pub namespace_uris: BTreeMap<String, String>,
}

/// Parse a raw XMP packet into a [`Parsed`] structure with
/// sorted properties and the captured namespace URI map.
pub fn parse(input: &[u8]) -> Result<Parsed, Error> {
    if input.is_empty() {
        return Err(Error::EmptyInput);
    }
    let s = std::str::from_utf8(input).map_err(|_| Error::NotUtf8)?;

    // Strip xpacket markers if present. Both markers are XML
    // processing instructions: <?xpacket begin=...?> and
    // <?xpacket end="..."?>. They are optional — many tools
    // strip them; the parser silently tolerates either form.
    let body = strip_xpacket(s)?;

    // Parse the body into an Element tree.
    let mut p = XmlParser::new(body);
    p.skip_prolog()?;
    let root = p.parse_element()?;

    // Locate <rdf:RDF> — either the root itself or a descendant.
    let rdf = find_element(&root, "rdf:RDF")
        .ok_or(Error::RdfRootMissing)?;

    // First <rdf:Description> child. Multiple Descriptions are
    // out of scope for v1 per the anchor; real PDFs use one.
    let desc = rdf
        .children
        .iter()
        .filter_map(node_as_element)
        .find(|e| e.tag == "rdf:Description");

    // Harvest every xmlns:prefix → uri declaration across the
    // parsed element tree before walking for properties.
    // First-declaration-wins (matches python's setdefault).
    let mut namespace_uris: BTreeMap<String, String> = BTreeMap::new();
    collect_namespace_uris(&root, &mut namespace_uris);

    let mut props: BTreeMap<String, String> = BTreeMap::new();
    if let Some(desc) = desc {
        // Attribute-form properties: any attribute on the
        // Description whose name carries a prefix and is not
        // `rdf:about` / `xmlns:*` (also `xml:*` is a reserved
        // namespace and is skipped).
        for (k, v) in &desc.attrs {
            if k == "rdf:about"
                || k.starts_with("xmlns:")
                || k == "xmlns"
                || k.starts_with("xml:")
            {
                continue;
            }
            if k.contains(':') {
                props.insert(k.clone(), v.clone());
            }
        }
        // Element-form properties.
        for child in desc.children.iter().filter_map(node_as_element) {
            if !child.tag.contains(':') {
                continue;
            }
            let value = extract_property_value(child);
            props.insert(child.tag.clone(), value);
        }
    }

    Ok(Parsed {
        properties: props.into_iter().collect(),
        namespace_uris,
    })
}

fn collect_namespace_uris(
    elem: &Element,
    into: &mut BTreeMap<String, String>,
) {
    for (k, v) in &elem.attrs {
        if !k.starts_with("xmlns:") {
            continue;
        }
        let prefix = &k["xmlns:".len()..];
        if prefix.is_empty() {
            continue;
        }
        if into.contains_key(prefix) {
            continue;
        }
        into.insert(prefix.to_string(), v.clone());
    }
    for child in elem.children.iter().filter_map(node_as_element) {
        collect_namespace_uris(child, into);
    }
}

/// Emit `props` in the freeze-gate canonical form:
/// `<key>\t<value>\n` per line. Empty list → empty bytes.
#[allow(dead_code)] // retained for a future-phase public-API binding
pub fn to_canonical(props: &[(String, String)]) -> Vec<u8> {
    let mut out = Vec::new();
    for (k, v) in props {
        out.extend_from_slice(k.as_bytes());
        out.push(b'\t');
        out.extend_from_slice(v.as_bytes());
        out.push(b'\n');
    }
    out
}

// ============================================================
// xpacket marker stripping
// ============================================================

fn strip_xpacket(s: &str) -> Result<&str, Error> {
    let bytes = s.as_bytes();
    let begin_marker = b"<?xpacket begin=";
    let end_marker = b"<?xpacket end=";
    let mut start = 0usize;
    let mut end = bytes.len();
    if let Some(p) = find_subslice(bytes, begin_marker) {
        // Locate `?>` after the marker — that ends the PI.
        let after = p + begin_marker.len();
        let close_at = find_subslice(&bytes[after..], b"?>")
            .ok_or(Error::MalformedXpacketMarker)?;
        start = after + close_at + 2;
    }
    if let Some(p) = find_subslice(&bytes[start..], end_marker) {
        end = start + p;
    }
    if start > end {
        return Err(Error::MalformedXpacketMarker);
    }
    std::str::from_utf8(&bytes[start..end]).map_err(|_| Error::NotUtf8)
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|w| w == needle)
}

// ============================================================
// XML tree
// ============================================================

#[derive(Debug)]
struct Element {
    tag: String,
    attrs: Vec<(String, String)>,
    children: Vec<Node>,
}

#[derive(Debug)]
enum Node {
    Element(Element),
    Text(String),
}

fn node_as_element(n: &Node) -> Option<&Element> {
    if let Node::Element(e) = n { Some(e) } else { None }
}

fn find_element<'a>(root: &'a Element, tag: &str) -> Option<&'a Element> {
    if root.tag == tag {
        return Some(root);
    }
    for child in root.children.iter().filter_map(node_as_element) {
        if let Some(e) = find_element(child, tag) {
            return Some(e);
        }
    }
    None
}

fn extract_property_value(elem: &Element) -> String {
    // Look for an rdf:Alt / rdf:Bag / rdf:Seq child first.
    for child in elem.children.iter().filter_map(node_as_element) {
        match child.tag.as_str() {
            "rdf:Alt" => return extract_alt(child),
            "rdf:Bag" => return extract_bag_or_seq(child, true),
            "rdf:Seq" => return extract_bag_or_seq(child, false),
            _ => {}
        }
    }
    // Otherwise: the element's direct text content.
    extract_text(elem)
}

fn extract_alt(elem: &Element) -> String {
    let mut x_default: Option<String> = None;
    let mut first: Option<String> = None;
    for child in elem.children.iter().filter_map(node_as_element) {
        if child.tag != "rdf:li" {
            continue;
        }
        let text = extract_text(child);
        if first.is_none() {
            first = Some(text.clone());
        }
        let lang = child
            .attrs
            .iter()
            .find(|(k, _)| k == "xml:lang")
            .map(|(_, v)| v.as_str());
        if lang == Some("x-default") {
            x_default = Some(text);
            break;
        }
    }
    x_default.or(first).unwrap_or_default()
}

fn extract_bag_or_seq(elem: &Element, sort_lex: bool) -> String {
    let mut items: Vec<String> = elem
        .children
        .iter()
        .filter_map(node_as_element)
        .filter(|e| e.tag == "rdf:li")
        .map(extract_text)
        .collect();
    if sort_lex {
        items.sort();
    }
    items.join("\t")
}

fn extract_text(elem: &Element) -> String {
    // For simple-text properties the element's children are a
    // single Text node (possibly wrapped in incidental
    // whitespace from XML pretty-printing). Collapse
    // whitespace-only text nodes between elements; otherwise
    // keep user-meaningful whitespace.
    let mut buf = String::new();
    for child in &elem.children {
        if let Node::Text(t) = child {
            buf.push_str(t);
        }
    }
    collapse_whitespace(&buf)
}

fn collapse_whitespace(s: &str) -> String {
    // Trim outer whitespace; collapse internal runs of
    // whitespace into a single ASCII space. Per the anchor:
    // "collapse internal newlines + leading/trailing whitespace
    // inside text content to a single space (XML's default
    // attribute-value normalization is NOT applied to element
    // text — we preserve user-meaningful whitespace where the
    // spec preserves it)".
    let trimmed = s.trim();
    let mut out = String::with_capacity(trimmed.len());
    let mut prev_ws = false;
    for c in trimmed.chars() {
        if c.is_whitespace() {
            if !prev_ws {
                out.push(' ');
            }
            prev_ws = true;
        } else {
            out.push(c);
            prev_ws = false;
        }
    }
    out
}

// ============================================================
// XML parser — minimal RDF/XML subset
// ============================================================

struct XmlParser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> XmlParser<'a> {
    fn new(input: &'a str) -> Self {
        XmlParser { bytes: input.as_bytes(), pos: 0 }
    }

    fn skip_prolog(&mut self) -> Result<(), Error> {
        // Skip leading whitespace, XML declarations
        // (<?xml ... ?>), processing instructions, and
        // comments until we land on the first element open.
        loop {
            self.skip_whitespace();
            if self.pos >= self.bytes.len() {
                return Err(Error::MalformedXml);
            }
            if self.starts_with(b"<?") {
                self.skip_processing_instruction()?;
                continue;
            }
            if self.starts_with(b"<!--") {
                self.skip_comment()?;
                continue;
            }
            if self.starts_with(b"<!") {
                // <!DOCTYPE ...> etc — skip up to matching '>'.
                self.skip_until_byte(b'>')?;
                self.pos += 1;
                continue;
            }
            return Ok(());
        }
    }

    fn parse_element(&mut self) -> Result<Element, Error> {
        if self.pos >= self.bytes.len() || self.bytes[self.pos] != b'<' {
            return Err(Error::MalformedXml);
        }
        self.pos += 1;
        let tag = self.read_name()?;
        let mut attrs: Vec<(String, String)> = Vec::new();
        let self_closed;
        loop {
            self.skip_whitespace();
            if self.pos >= self.bytes.len() {
                return Err(Error::MalformedXml);
            }
            match self.bytes[self.pos] {
                b'>' => {
                    self.pos += 1;
                    self_closed = false;
                    break;
                }
                b'/' => {
                    self.pos += 1;
                    if self.pos >= self.bytes.len()
                        || self.bytes[self.pos] != b'>'
                    {
                        return Err(Error::MalformedXml);
                    }
                    self.pos += 1;
                    self_closed = true;
                    break;
                }
                _ => {
                    let aname = self.read_name()?;
                    self.skip_whitespace();
                    if self.pos >= self.bytes.len()
                        || self.bytes[self.pos] != b'='
                    {
                        return Err(Error::MalformedXml);
                    }
                    self.pos += 1;
                    self.skip_whitespace();
                    let avalue = self.read_attr_value()?;
                    attrs.push((aname, avalue));
                }
            }
        }

        if self_closed {
            return Ok(Element { tag, attrs, children: Vec::new() });
        }

        let mut children: Vec<Node> = Vec::new();
        loop {
            // Read text up to the next '<'.
            let text_start = self.pos;
            while self.pos < self.bytes.len() && self.bytes[self.pos] != b'<' {
                self.pos += 1;
            }
            if text_start < self.pos {
                let raw = std::str::from_utf8(&self.bytes[text_start..self.pos])
                    .map_err(|_| Error::MalformedXml)?;
                let decoded = decode_entities(raw)?;
                children.push(Node::Text(decoded));
            }
            if self.pos >= self.bytes.len() {
                return Err(Error::MalformedXml);
            }
            // self.bytes[self.pos] == '<'
            if self.starts_with(b"</") {
                self.pos += 2;
                let close_name = self.read_name()?;
                self.skip_whitespace();
                if self.pos >= self.bytes.len()
                    || self.bytes[self.pos] != b'>'
                {
                    return Err(Error::MalformedXml);
                }
                self.pos += 1;
                if close_name != tag {
                    return Err(Error::MalformedXml);
                }
                return Ok(Element { tag, attrs, children });
            }
            if self.starts_with(b"<?") {
                self.skip_processing_instruction()?;
                continue;
            }
            if self.starts_with(b"<!--") {
                self.skip_comment()?;
                continue;
            }
            // Nested element.
            let child = self.parse_element()?;
            children.push(Node::Element(child));
        }
    }

    fn read_name(&mut self) -> Result<String, Error> {
        let start = self.pos;
        while self.pos < self.bytes.len() {
            let b = self.bytes[self.pos];
            if is_xml_ws(b) || b == b'>' || b == b'/' || b == b'=' {
                break;
            }
            self.pos += 1;
        }
        if start == self.pos {
            return Err(Error::MalformedXml);
        }
        Ok(std::str::from_utf8(&self.bytes[start..self.pos])
            .map_err(|_| Error::MalformedXml)?
            .to_string())
    }

    fn read_attr_value(&mut self) -> Result<String, Error> {
        if self.pos >= self.bytes.len() {
            return Err(Error::MalformedXml);
        }
        let quote = self.bytes[self.pos];
        if quote != b'"' && quote != b'\'' {
            return Err(Error::MalformedXml);
        }
        self.pos += 1;
        let start = self.pos;
        while self.pos < self.bytes.len() && self.bytes[self.pos] != quote {
            self.pos += 1;
        }
        if self.pos >= self.bytes.len() {
            return Err(Error::MalformedXml);
        }
        let raw = std::str::from_utf8(&self.bytes[start..self.pos])
            .map_err(|_| Error::MalformedXml)?;
        let decoded = decode_entities(raw)?;
        self.pos += 1;
        Ok(decoded)
    }

    fn starts_with(&self, prefix: &[u8]) -> bool {
        self.pos + prefix.len() <= self.bytes.len()
            && &self.bytes[self.pos..self.pos + prefix.len()] == prefix
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.bytes.len() && is_xml_ws(self.bytes[self.pos]) {
            self.pos += 1;
        }
    }

    fn skip_processing_instruction(&mut self) -> Result<(), Error> {
        // self.bytes[pos..] starts with "<?"
        self.pos += 2;
        while self.pos + 1 < self.bytes.len() {
            if self.bytes[self.pos] == b'?'
                && self.bytes[self.pos + 1] == b'>'
            {
                self.pos += 2;
                return Ok(());
            }
            self.pos += 1;
        }
        Err(Error::MalformedXml)
    }

    fn skip_comment(&mut self) -> Result<(), Error> {
        // self.bytes[pos..] starts with "<!--"
        self.pos += 4;
        while self.pos + 2 < self.bytes.len() {
            if &self.bytes[self.pos..self.pos + 3] == b"-->" {
                self.pos += 3;
                return Ok(());
            }
            self.pos += 1;
        }
        Err(Error::MalformedXml)
    }

    fn skip_until_byte(&mut self, b: u8) -> Result<(), Error> {
        while self.pos < self.bytes.len() && self.bytes[self.pos] != b {
            self.pos += 1;
        }
        if self.pos >= self.bytes.len() {
            return Err(Error::MalformedXml);
        }
        Ok(())
    }
}

fn is_xml_ws(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r')
}

fn decode_entities(s: &str) -> Result<String, Error> {
    if !s.contains('&') {
        return Ok(s.to_string());
    }
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'&' {
            let semi = (i + 1..bytes.len()).find(|&j| bytes[j] == b';');
            let semi = semi.ok_or(Error::MalformedXml)?;
            let ent = &bytes[i + 1..semi];
            match ent {
                b"amp" => out.push('&'),
                b"lt" => out.push('<'),
                b"gt" => out.push('>'),
                b"quot" => out.push('"'),
                b"apos" => out.push('\''),
                e if e.starts_with(b"#x") || e.starts_with(b"#X") => {
                    let hex = std::str::from_utf8(&e[2..])
                        .map_err(|_| Error::MalformedXml)?;
                    let cp = u32::from_str_radix(hex, 16)
                        .map_err(|_| Error::MalformedXml)?;
                    out.push(char::from_u32(cp).ok_or(Error::MalformedXml)?);
                }
                e if e.starts_with(b"#") => {
                    let dec = std::str::from_utf8(&e[1..])
                        .map_err(|_| Error::MalformedXml)?;
                    let cp: u32 = dec.parse().map_err(|_| Error::MalformedXml)?;
                    out.push(char::from_u32(cp).ok_or(Error::MalformedXml)?);
                }
                _ => return Err(Error::MalformedXml),
            }
            i = semi + 1;
        } else {
            // UTF-8 chars are multi-byte; we always work at the
            // byte level here, but `out.push` accepts char only.
            // Take the next char from the rest of the string by
            // decoding from `i`.
            let rest = &s[i..];
            let c = rest.chars().next().ok_or(Error::MalformedXml)?;
            out.push(c);
            i += c.len_utf8();
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canonical_str(input: &[u8]) -> String {
        let parsed = parse(input).expect("parse");
        String::from_utf8(to_canonical(&parsed.properties))
            .expect("utf8 canonical")
    }

    #[test]
    fn empty_input_errors() {
        assert_eq!(parse(b"").unwrap_err(), Error::EmptyInput);
    }

    #[test]
    fn empty_description_yields_empty_canonical() {
        let xmp = br#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description rdf:about=""/>
  </rdf:RDF>
</x:xmpmeta>
<?xpacket end="r"?>"#;
        assert_eq!(canonical_str(xmp), "");
    }

    #[test]
    fn alt_picks_x_default() {
        let xmp = br#"<rdf:RDF xmlns:rdf="x"><rdf:Description xmlns:dc="x">
  <dc:title>
    <rdf:Alt>
      <rdf:li xml:lang="fr">Titre</rdf:li>
      <rdf:li xml:lang="x-default">Title</rdf:li>
    </rdf:Alt>
  </dc:title>
</rdf:Description></rdf:RDF>"#;
        assert_eq!(canonical_str(xmp), "dc:title\tTitle\n");
    }

    #[test]
    fn alt_falls_back_to_first_li_without_x_default() {
        let xmp = br#"<rdf:RDF xmlns:rdf="x"><rdf:Description xmlns:dc="x">
  <dc:title>
    <rdf:Alt>
      <rdf:li xml:lang="en">First</rdf:li>
      <rdf:li xml:lang="fr">Second</rdf:li>
    </rdf:Alt>
  </dc:title>
</rdf:Description></rdf:RDF>"#;
        assert_eq!(canonical_str(xmp), "dc:title\tFirst\n");
    }

    #[test]
    fn seq_joined_with_tab_in_document_order() {
        let xmp = br#"<rdf:RDF xmlns:rdf="x"><rdf:Description xmlns:dc="x">
  <dc:creator>
    <rdf:Seq>
      <rdf:li>Alice</rdf:li>
      <rdf:li>Bob</rdf:li>
    </rdf:Seq>
  </dc:creator>
</rdf:Description></rdf:RDF>"#;
        assert_eq!(canonical_str(xmp), "dc:creator\tAlice\tBob\n");
    }

    #[test]
    fn bag_sorted_lexicographically() {
        let xmp = br#"<rdf:RDF xmlns:rdf="x"><rdf:Description xmlns:dc="x">
  <dc:subject>
    <rdf:Bag>
      <rdf:li>zebra</rdf:li>
      <rdf:li>apple</rdf:li>
    </rdf:Bag>
  </dc:subject>
</rdf:Description></rdf:RDF>"#;
        assert_eq!(canonical_str(xmp), "dc:subject\tapple\tzebra\n");
    }

    #[test]
    fn attribute_form_extracted() {
        let xmp = br#"<rdf:RDF xmlns:rdf="x">
  <rdf:Description rdf:about="" xmlns:pdf="x"
      pdf:Producer="Aspose.PDF" pdf:Keywords="a, b"/>
</rdf:RDF>"#;
        let s = canonical_str(xmp);
        assert!(s.contains("pdf:Producer\tAspose.PDF\n"));
        assert!(s.contains("pdf:Keywords\ta, b\n"));
    }

    #[test]
    fn keys_emitted_sorted() {
        let xmp = br#"<rdf:RDF xmlns:rdf="x"><rdf:Description xmlns:zz="x" xmlns:dc="x">
  <zz:b>2</zz:b>
  <dc:a>1</dc:a>
</rdf:Description></rdf:RDF>"#;
        // Keys should appear in lex order: dc:a then zz:b.
        let s = canonical_str(xmp);
        let dc_pos = s.find("dc:a").unwrap();
        let zz_pos = s.find("zz:b").unwrap();
        assert!(dc_pos < zz_pos, "expected dc:a before zz:b in {s:?}");
    }

    #[test]
    fn missing_rdf_root_errors() {
        let xmp = br#"<x:xmpmeta xmlns:x="x"><foo/></x:xmpmeta>"#;
        assert_eq!(parse(xmp).unwrap_err(), Error::RdfRootMissing);
    }
}
