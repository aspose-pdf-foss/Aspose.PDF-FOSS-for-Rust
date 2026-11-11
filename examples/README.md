# Examples

Small standalone programs showing how to call into the public
`aspose-pdf-foss` API. Each `.rs` is a Cargo example — Cargo's
auto-discovery picks them up; no entries in `Cargo.toml` needed.

A handful of tiny sample PDFs (< 3 KB each) lives alongside in
`pdfs/` so the examples run out of the box.

## Run

`cargo run --example <name>` builds the lib + that one example and
runs it:

```bash
SAMPLES=./examples/pdfs

# 1. Open a PDF, count its pages, iterate via the 1-based
#    pages().get(i) accessor and print each page.number().
cargo run --example 01_pages -- "$SAMPLES/two_pages.pdf"

# 2. Read the /Info dictionary (Title, Author, …).
cargo run --example 02_read_metadata -- "$SAMPLES/with_info.pdf"

# 3. Edit metadata (set_title + DocumentInfo setters + add)
#    and save. The lib appends an incremental update; original
#    bytes preserved.
cargo run --example 03_write_metadata -- \
    "$SAMPLES/with_info.pdf" /tmp/with_my_metadata.pdf

# 4. Read back what 03 wrote — our edits round-trip, original
#    fields the setters didn't touch are preserved verbatim.
cargo run --example 02_read_metadata -- /tmp/with_my_metadata.pdf

# 5. Extract every word of text via TextAbsorber::visit_document(&doc).
cargo run --example 04_text_extraction -- "$SAMPLES/hello_world.pdf"
cargo run --example 04_text_extraction -- "$SAMPLES/two_lines.pdf"

# 6. Save with no edits — output is byte-identical to the input.
cargo run --example 05_save_roundtrip -- \
    "$SAMPLES/two_pages.pdf" /tmp/copy.pdf

# 7. Construct rendering devices + round-trip their properties.
cargo run --example 06_devices_surface

# 8. Rasterise page 1 of two_pages.pdf to PNG (the end-to-end
#    rendering pipeline through PngDevice::process(page, sink)).
cargo run --example 07_render_png -- \
    "$SAMPLES/two_pages.pdf" 1 /tmp/page1.png

# 9. Rasterise every page of two_pages.pdf into a single multi-page
#    TIFF via TiffDevice::process_document_range(&doc, from, to, sink).
cargo run --example 08_render_tiff -- \
    "$SAMPLES/two_pages.pdf" /tmp/all_pages.tiff

# 10. Same fixture, 8-bit palettised TIFF (256-colour palette via
#     median-cut quantisation through TiffSettings::set_depth).
cargo run --example 09_indexed_tiff -- \
    "$SAMPLES/two_pages.pdf" 1 /tmp/page1_8bpp.tiff

# 11. Render a text-bearing PDF to PNG. text_hello.pdf carries an
#     embedded TrueType font, so its glyphs paint correctly.
cargo run --example 10_render_text_pdf -- \
    "$SAMPLES/text_hello.pdf" /tmp/text.png

# 12. Render a Standard14-only PDF — hello_world.pdf references
#     /BaseFont /Helvetica with NO embedded font program. The
#     internal/standard14_outlines fallback supplies bytes from
#     the bundled Liberation Sans substitute.
cargo run --example 11_render_standard14 -- \
    "$SAMPLES/hello_world.pdf" /tmp/std14.png

# 12. Build a ten-page feature showcase FROM SCRATCH — the only
#     example that takes no input PDF. Pages cover bookmarks, a
#     contents list, the Standard-14 font grid, a watermark, an
#     embedded image, six AcroForm field types, nine annotations,
#     a table with a spanned row, vector graphics, a bar chart and
#     a document-level attachment.
cargo run --example 12_create_features -- /tmp/showcase.pdf

#     Rasterise any of its pages to see what the engine paints:
cargo run --example 07_render_png -- /tmp/showcase.pdf 7 /tmp/showcase_p7.png
```

## What each example shows

| File | API surfaces | Sample input |
|---|---|---|
| `01_pages.rs` | `Document::open(path)`, `doc.pages().count()`, `doc.pages().get(i)` (1-based), `Page::number()` | any |
| `02_read_metadata.rs` | `doc.info()` accessor + 6 read-only `DocumentInfo` getters | `with_info.pdf` (or any) |
| `03_write_metadata.rs` | `doc.set_title`, `DocumentInfo` setters, `info.add`, `doc.save_to` (incremental update) | `with_info.pdf` |
| `04_text_extraction.rs` | `TextAbsorber`, `visit_document(&doc)`, `text()`, `has_errors()` | `hello_world.pdf`, `two_lines.pdf` |
| `05_save_roundtrip.rs` | `doc.save_to` byte-verbatim path | any |
| `06_devices_surface.rs` | `PngDevice` / `JpegDevice` / `BmpDevice` / `TextDevice` ctors + properties | none (no input) |
| `07_render_png.rs` | `doc.pages().get(N)` + `PngDevice::process(page, sink)` end-to-end | any |
| `08_render_tiff.rs` | `TiffDevice::process_document_range(&doc, from, to, sink)` — multi-page TIFF | any |
| `09_indexed_tiff.rs` | `TiffSettings::set_depth(ColorDepth::Format8bpp)` + `TiffDevice::process(page, sink)` — palettised TIFF (median-cut palette) | any |
| `10_render_text_pdf.rs` | Same `process`, on a text-bearing page (embedded TrueType font) | `text_hello.pdf` |
| `11_render_standard14.rs` | Same `process`, but on a PDF that references `/Helvetica` without an embedded font program — exercises the `internal/standard14_outlines` fallback (bundled Liberation Sans) | `hello_world.pdf` |
| `12_create_features.rs` | The creation surface end to end: `Document::new`, `pages().add()`, `Page::paragraphs()` (`TextFragment` / `Table` / `Graph`), `TextBuilder` + `TextParagraph`, `Page::artifacts()` (`WatermarkArtifact`), `Page::add_image`, `doc.form_mut()` with six field types, `Page::annotations()` with nine annotation types, `doc.outlines()` bookmarks with destinations, and `doc.embedded_files_mut()` | none — writes its own |

## Notes

* **Example 03 requires** the input PDF to already have an `/Info`
  dictionary in its trailer — the incremental-update writer patches
  an existing `/Info` object rather than synthesising a new one.
  `with_info.pdf` is bundled for this reason. Inputs without
  `/Info` cause `doc.save_to()` to return `Error::NoInfoForUpdate`.

* **Example 06 doesn't drive a device end-to-end** — it only
  constructs them and round-trips properties. Examples 07 and 08
  are the end-to-end rendering ones: `Document → pages().get(N) →
  PngDevice::process → PNG bytes` (single page) and
  `Document → TiffDevice::process_document_range(&doc, from, to,
  sink) → multi-page TIFF` (page range).

* **Standard14 fonts render via fallback** — PDFs that reference
  Helvetica / Times-Roman / Courier (× 4 styles each — 12 of the
  14 in v1 scope) without embedding font data paint correctly
  through `internal/standard14_outlines`. The 4 MB Liberation
  substitute (SIL OFL 1.1) is bundled into the lib. Symbol and
  ZapfDingbats are deferred.

* **Caller-pluggable palette quantisation** — `TiffDevice` has
  ctor overloads that take a trait object implementing
  `IIndexBitmapConverter` for callers who want their own
  palette-generation algorithm. Implement the trait and pass an
  `Arc<dyn IIndexBitmapConverter>` to one of those ctors.
  Default median-cut (used by example 09) is the no-converter path.

* The examples are intentionally small + comment-light — the `.rs`
  source itself is the documentation. Read them top to bottom in
  numeric order for a tour of the public API.

* **Example 12 is the exception on length** — it is the creation
  tour, so it is long by design, and it is the only one that needs
  no input. Its header comment lists what each page exercises and
  which parts of those pages are structure rather than paint.

## Sample PDFs

Tiny synthetic fixtures shared with the cpp + csharp + ts lib
repos. Each is hand-written PDF source — no glyphs from licensed
fonts, no compressed streams.

| File | Size | What it carries |
|---|---|---|
| `two_pages.pdf` | 687 B | A two-page document, no metadata, no text |
| `with_info.pdf` | 688 B | One blank page + a populated `/Info` dictionary (Title, Author, Producer, Keywords) |
| `hello_world.pdf` | 592 B | One page rendering "Hello World" via Helvetica-equivalent encoding |
| `two_lines.pdf` | 630 B | One page with two text lines on separate Tj operators |
| `text_hello.pdf` | 2.0 KB | One page with a 442-byte embedded TrueType font; glyphs render correctly via `PngDevice::process` |

If you want richer inputs (real-world PDFs with images, fonts,
encryption, …), drop them anywhere and pass the path as the first
argv. The examples don't depend on the bundled samples.
