// Minimal SVG → single-page PDF importer (target-side helper, no
// SpecDev anchor — like standard14_metrics). Reads the root <svg>
// width / height (falling back to viewBox) for the page size, and
// renders the basic vector primitives (rect / circle / line) with
// solid fill / stroke colours into the page content stream. Text,
// transforms, curves, gradients, and units other than user-space /
// px are out of scope (the cluster's deferred surface).
//
// SVG's y-axis runs top-down; PDF's runs bottom-up, so y is flipped
// against the page height when emitting content operators.

/// Parse SVG bytes and produce a one-page PDF document. The page is
/// sized to the SVG (defaulting to US-Letter when no size is found).
pub fn to_pdf(svg: &[u8]) -> Vec<u8> {
    let text = String::from_utf8_lossy(svg);
    let (width, height) = page_size(&text).unwrap_or((612.0, 792.0));
    let content = render_shapes(&text, height);
    build_single_page_pdf(width, height, &content)
}

/// The numeric value of attribute `name` on the first occurrence of
/// `<tag`, stripping a trailing `px` unit.
fn attr(s: &str, tag: &str, name: &str) -> Option<f64> {
    let tag_pos = s.find(&format!("<{tag}"))?;
    let rest = &s[tag_pos..];
    let end = rest.find('>').map(|e| tag_pos + e).unwrap_or(s.len());
    attr_in(&s[tag_pos..end], name)
}

/// The numeric value of attribute `name` within an element slice.
fn attr_in(elem: &str, name: &str) -> Option<f64> {
    let pat = format!("{name}=\"");
    let p = elem.find(&pat)? + pat.len();
    let v = &elem[p..];
    let q = v.find('"')?;
    let raw = v[..q].trim().trim_end_matches("px").trim();
    raw.parse::<f64>().ok()
}

/// The string value of attribute `name` within an element slice.
fn attr_str<'a>(elem: &'a str, name: &str) -> Option<&'a str> {
    let pat = format!("{name}=\"");
    let p = elem.find(&pat)? + pat.len();
    let v = &elem[p..];
    let q = v.find('"')?;
    Some(v[..q].trim())
}

fn page_size(s: &str) -> Option<(f64, f64)> {
    if let (Some(w), Some(h)) = (attr(s, "svg", "width"), attr(s, "svg", "height")) {
        if w > 0.0 && h > 0.0 {
            return Some((w, h));
        }
    }
    // viewBox="minX minY width height"
    let vb = attr_str(svg_open_tag(s)?, "viewBox")?;
    let nums: Vec<f64> = vb.split_whitespace().filter_map(|n| n.parse().ok()).collect();
    if nums.len() == 4 && nums[2] > 0.0 && nums[3] > 0.0 {
        return Some((nums[2], nums[3]));
    }
    None
}

fn svg_open_tag(s: &str) -> Option<&str> {
    let p = s.find("<svg")?;
    let e = s[p..].find('>')? + p;
    Some(&s[p..e])
}

/// Map a CSS colour (`#rrggbb`, `#rgb`, or a few names) to PDF RGB
/// `(r, g, b)` in 0..1. Returns `None` for `none` / unparseable.
fn parse_color(c: &str) -> Option<(f64, f64, f64)> {
    let c = c.trim();
    if c.eq_ignore_ascii_case("none") {
        return None;
    }
    if let Some(hex) = c.strip_prefix('#') {
        let h = if hex.len() == 3 {
            hex.chars().flat_map(|ch| [ch, ch]).collect::<String>()
        } else {
            hex.to_string()
        };
        if h.len() == 6 {
            let r = u8::from_str_radix(&h[0..2], 16).ok()?;
            let g = u8::from_str_radix(&h[2..4], 16).ok()?;
            let b = u8::from_str_radix(&h[4..6], 16).ok()?;
            return Some((r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0));
        }
        return None;
    }
    Some(match c.to_ascii_lowercase().as_str() {
        "black" => (0.0, 0.0, 0.0),
        "white" => (1.0, 1.0, 1.0),
        "red" => (1.0, 0.0, 0.0),
        "green" => (0.0, 0.5, 0.0),
        "blue" => (0.0, 0.0, 1.0),
        "yellow" => (1.0, 1.0, 0.0),
        "gray" | "grey" => (0.5, 0.5, 0.5),
        _ => return None,
    })
}

/// Iterate the `<rect …/>`, `<circle …/>`, `<line …/>` elements and
/// emit PDF content operators (y flipped against `h`).
fn render_shapes(s: &str, h: f64) -> Vec<u8> {
    let mut out = String::new();
    for elem in elements(s, "rect") {
        if let (Some(x), Some(y), Some(w), Some(rh)) = (
            attr_in(&elem, "x"),
            attr_in(&elem, "y"),
            attr_in(&elem, "width"),
            attr_in(&elem, "height"),
        ) {
            let fill = attr_str(&elem, "fill").and_then(parse_color);
            if let Some((r, g, b)) = fill {
                out.push_str(&format!(
                    "{r} {g} {b} rg\n{x} {} {w} {rh} re\nf\n",
                    h - y - rh
                ));
            }
        }
    }
    for elem in elements(s, "circle") {
        if let (Some(cx), Some(cy), Some(rad)) =
            (attr_in(&elem, "cx"), attr_in(&elem, "cy"), attr_in(&elem, "r"))
        {
            if let Some((r, g, b)) = attr_str(&elem, "fill").and_then(parse_color) {
                out.push_str(&format!("{r} {g} {b} rg\n"));
                out.push_str(&circle_path(cx, h - cy, rad));
                out.push_str("f\n");
            }
        }
    }
    for elem in elements(s, "line") {
        if let (Some(x1), Some(y1), Some(x2), Some(y2)) = (
            attr_in(&elem, "x1"),
            attr_in(&elem, "y1"),
            attr_in(&elem, "x2"),
            attr_in(&elem, "y2"),
        ) {
            let stroke = attr_str(&elem, "stroke").and_then(parse_color)
                .unwrap_or((0.0, 0.0, 0.0));
            let (r, g, b) = stroke;
            out.push_str(&format!(
                "{r} {g} {b} RG\n{x1} {} m\n{x2} {} l\nS\n",
                h - y1,
                h - y2
            ));
        }
    }
    out.into_bytes()
}

/// Every `<tag …>` / `<tag …/>` element slice (open tag only).
fn elements(s: &str, tag: &str) -> Vec<String> {
    let needle = format!("<{tag}");
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(rel) = s[from..].find(&needle) {
        let start = from + rel;
        let end = s[start..].find('>').map(|e| start + e + 1).unwrap_or(s.len());
        out.push(s[start..end].to_string());
        from = end;
    }
    out
}

/// A circle as four cubic Béziers (kappa = 0.5523).
fn circle_path(cx: f64, cy: f64, r: f64) -> String {
    let k = 0.552_284_75 * r;
    format!(
        "{} {} m\n\
         {} {} {} {} {} {} c\n\
         {} {} {} {} {} {} c\n\
         {} {} {} {} {} {} c\n\
         {} {} {} {} {} {} c\n",
        cx + r, cy,
        cx + r, cy + k, cx + k, cy + r, cx, cy + r,
        cx - k, cy + r, cx - r, cy + k, cx - r, cy,
        cx - r, cy - k, cx - k, cy - r, cx, cy - r,
        cx + k, cy - r, cx + r, cy - k, cx + r, cy,
    )
}

/// Assemble a one-page PDF (catalog + pages + page + content
/// stream) with a byte-offset xref table.
fn build_single_page_pdf(width: f64, height: f64, content: &[u8]) -> Vec<u8> {
    let header = b"%PDF-1.7\n";
    let obj1 = b"1 0 obj\n<</Type/Catalog/Pages 2 0 R>>\nendobj\n".to_vec();
    let obj2 = b"2 0 obj\n<</Type/Pages/Count 1/Kids[3 0 R]>>\nendobj\n".to_vec();
    let obj3 = format!(
        "3 0 obj\n<</Type/Page/Parent 2 0 R/MediaBox[0 0 {width} {height}]/Contents 4 0 R/Resources<<>>>>\nendobj\n"
    )
    .into_bytes();
    let mut obj4 = format!("4 0 obj\n<</Length {}>>\nstream\n", content.len())
        .into_bytes();
    obj4.extend_from_slice(content);
    obj4.extend_from_slice(b"\nendstream\nendobj\n");

    let mut out = Vec::new();
    out.extend_from_slice(header);
    let o1 = out.len();
    out.extend_from_slice(&obj1);
    let o2 = out.len();
    out.extend_from_slice(&obj2);
    let o3 = out.len();
    out.extend_from_slice(&obj3);
    let o4 = out.len();
    out.extend_from_slice(&obj4);
    let xref_off = out.len();
    let xref = format!(
        "xref\n0 5\n\
         0000000000 65535 f \n\
         {o1:010} 00000 n \n\
         {o2:010} 00000 n \n\
         {o3:010} 00000 n \n\
         {o4:010} 00000 n \n\
         trailer\n<</Size 5/Root 1 0 R>>\nstartxref\n{xref_off}\n%%EOF\n"
    );
    out.extend_from_slice(xref.as_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_size_and_shapes() {
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="150">
            <rect x="10" y="10" width="80" height="40" fill="#ff0000"/>
            <circle cx="150" cy="100" r="30" fill="blue"/>
            <line x1="0" y1="0" x2="200" y2="150" stroke="black"/>
            </svg>"##;
        let pdf = to_pdf(svg);
        let s = String::from_utf8_lossy(&pdf);
        assert!(s.contains("/MediaBox[0 0 200 150]"));
        assert!(s.contains(" re\nf")); // the filled rect
        assert!(s.contains(" l\nS\n")); // the stroked line
        assert!(s.starts_with("%PDF-1.7"));
    }

    #[test]
    fn falls_back_to_viewbox() {
        let svg = br##"<svg viewBox="0 0 300 400"></svg>"##;
        let (w, h) = page_size(&String::from_utf8_lossy(svg)).unwrap();
        assert_eq!((w, h), (300.0, 400.0));
    }
}
