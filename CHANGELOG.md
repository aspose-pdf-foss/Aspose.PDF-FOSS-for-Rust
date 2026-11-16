# Changelog

All notable changes to this project are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this
project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Versioning policy

Releases are numbered **`1.YY.MM`** — the major pinned at `1`, the minor the release
year, the patch the release month, with the month not zero-padded. The first public
release is **`1.26.11`** (November 2026); January 2027 is `1.27.1`, February `1.27.2`.
The pattern follows the Aspose house style (`YY.MM`) with the major fixed so that a new
calendar year is not announced as a breaking change.

The cadence is **one release a month**, which is what makes the scheme fit: the month
*is* the patch number, so there is no component left for a second release within the same
month.

The crate ships a **deliberate, permanent subset** of the canonical Aspose.PDF public
API. A release number says *when* the crate was cut, not how much of the canonical
surface it covers.

**What that means for a caller.** Because every `1.x` is semver-compatible, Cargo will
upgrade `aspose-pdf-foss = "1.26"` to a later year without asking. The scheme therefore
only works if the 1.x line never breaks a caller, and that is the promise: existing
names, signatures and semantics stay put, and the public surface only grows. Raising the
MSRV is called out in the entry for the release that raises it — it cannot be signalled
in the number, the minor and patch being the date.

## [Unreleased]

Work towards the next monthly release. Entries accumulate here, and at tag time this
heading takes that release's number and date while a fresh `[Unreleased]` opens above it.
`Cargo.toml` keeps the released number until the first change that alters behaviour:
a correction to documentation does not change what the crate does, so it ships under the
number it was released with.

## [1.26.11] — 2026-11-11

The first public release, so this entry describes the whole shipping surface rather
than a delta against a previous version; every later entry is a delta against this one.

### Added

- **Document model** — open, inspect and save PDFs. Saving preserves the original bytes
  and appends an incremental update (PDF 32000-1:2008 §7.5.6), so a load/save round-trip
  is byte-verbatim for the untouched part of the file.
- **Pages** — page collection, per-page geometry (`MediaBox` and friends), rotation,
  resources, page-level annotation and image access.
- **Creation** — a document built from nothing: `Document::new`, appended or inserted
  pages, and content authored onto them through `Page::paragraphs` (text fragments,
  tables, drawings), `TextBuilder`, `Page::artifacts`, annotations, form fields,
  bookmarks and attachments. `Page::add_image` places an image and draws it.
  `examples/12_create_features.rs` is the tour.
- **Annotations** — the markup, geometry, printer-mark and widget families, with the
  appearance, state and flag types they carry. Saving writes the entries §12.5.2 and
  §12.5.6 give an annotation: the colour and flags, the border with its style, dash and
  cloudy effect, the markup author, subject and opacity, and the entry that gives each
  type its shape — an ink annotation's `/InkList`, a line's `/L` and the rest of its
  Table 177, a polygon's `/Vertices`, a text-markup annotation's `/QuadPoints`, a free
  text annotation's required `/DA`, a file attachment's required `/FS`, the `/A` action a
  link or widget runs, and the `/MN` that tells five printer marks apart. §12.9's
  `/Measure` dictionary is written for the three annotations the format gives one — a
  line, a polygon and a polyline — including the per-axis `/NumberFormat` arrays, where
  Table 262's single `/D` carries a decimal format's precision and a fractional one's
  denominator. The two dates, `/M` (§12.5.2) and a markup annotation's `/CreationDate`
  (§12.5.6.2), are written only when the caller gave them, so no annotation is stamped
  with a time it was never told. An annotation
  loaded from a document keeps the entries this model does not name, so rewriting a page
  does not strip them. What is not written is an appearance stream: a viewer draws from
  the entries, which is why the ones that *declare* an appearance rather than contain it
  (a stamp's `/Name`, a note's icon, a check box's mark) are written and the streams are
  not.
- **Forms** — AcroForm field types (text, checkbox, radio, choice, list, combo, date,
  barcode, signature) plus the field-level appearance and characteristic types. A field
  is written as the merged field / widget object of §12.7.3.1, carrying the `/Ff` flags
  its type is defined by — DoNotSpellCheck among them, written for the `false` state
  because the flag is a negation, and only on an editable combo box as Table 230
  requires — a choice field's `/Opt` options and both entries §12.7.4.4 gives a
  selection, the `/V` a viewer displays and the `/I` indices it was chosen by, a text
  field's `/MaxLen` and comb cells, the `/MK` captions and check-box mark, `/DA`, `/BS`,
  the user-facing and export names and the action it runs. `FormEditor` configures a
  field by name — flags, limits, geometry, options, scripts, submit target, alignment —
  and `Form::flatten` drops the form structurally, removing the catalog's `/AcroForm`.
- **Facades** — the task-oriented layer: `PdfFileEditor`, `PdfFileSecurity`,
  `PdfFileSignature`, `FormEditor`, `PdfAnnotationEditor`, `PdfBookmarkEditor`,
  `PdfExtractor`, `PdfPageEditor`, `PdfFileInfo`, `PdfXmpMetadata`, `PdfFileStamp`,
  `PdfContentEditor`, `PdfConverter`.
- **Text** — `TextAbsorber` and `TextFragmentAbsorber` over a document or a single page,
  with the fragment, paragraph, segment-state and search-option types. A phrase search
  can be scoped to a rectangle or to the visible page; a search asked for as a regular
  expression refuses rather than answering a pattern with a literal match. Written text
  carries the font, size and fill colour, character and word spacing, horizontal
  scaling, invisibility, a stroked outline, the baseline shift of sub- and superscript,
  and underline / strike-out rules drawn at the position and thickness the Standard-14
  metrics state.
- **Rendering** — page-to-image through `PngDevice`, `JpegDevice`, `BmpDevice` and
  `TiffDevice` (single- and multi-page), plus `TextDevice`. TIFF output covers 1/4/8/24
  bpp with median-cut palette quantisation, and the palette generator is pluggable.
  The page engine covers fills and strokes with dash patterns, rectangular clipping,
  constant alpha, Form XObjects and the annotation appearance streams built on them,
  axial and radial shadings as both `sh` and pattern fills, tiling patterns, and images
  from JPEG, raw samples and inline `BI` / `ID` / `EI` data, honouring `/SMask` soft
  masks and `/Mask` stencils. What falls outside that is left unpainted rather than
  substituted: colour-key `/Mask` arrays, JBIG2 stencils, function-based and mesh
  shadings, uncoloured tiling patterns, soft-mask groups and non-Normal blend modes.
- **Text rendering** — embedded TrueType, the Standard-14 substitutes, composite Type0 /
  CID fonts with `/CIDToGIDMap` and `/W` metrics, and Type3 fonts whose glyph procedures
  execute like nested forms. CIDFontType0's CFF program is not rendered.
- **Font substitution** — *any* simple font with no embedded program paints. The
  `/BaseFont` name is mapped onto the nearest of the twelve resolvable Standard-14 faces
  by family, weight and slope, with the `/FontDescriptor` serif bit deciding a name that
  names no family, so a document's `/Arial` or `/Verdana` renders in a metric class
  rather than coming out blank. Platform fonts are probed first, then the bundled
  Liberation faces. Symbol and ZapfDingbats have no text-face substitute and are left
  unpainted rather than drawn with the wrong glyphs.
- **Encryption** — RC4-40, RC4-128, AES-128 and AES-256 (security-handler revisions 2, 3,
  4 and 6), read and write.
- **Signing** — PKCS#1, PKCS#7 and detached PKCS#7 signatures and DocMDP permissions.
  The signature dictionary carries the signer, the reason, the location, the contact and
  the time of signing taken from the signature object, with the `/Contents` placeholder
  sized from it; a PKCS#7 too long for that placeholder is refused rather than truncated
  into a signature that looks well-formed and fails to verify. A custom signature
  *appearance* is carried and not drawn — it is an appearance stream.
- **Metadata** — `DocumentInfo` typed accessors with custom-key add/remove/clear, and XMP
  read and written through `Metadata`.
- **Attachments** — a file embedded into the document's `/Names /EmbeddedFiles` with its
  description, MIME type, associated-file relationship and file system, and its bytes
  read back out through `PdfExtractor`, undoing a Flate, LZW, ASCII85 or ASCIIHex filter
  chain.
- **Drawing** — the graph and shape value types.
- **Image placement** — an absorber reporting where each image is drawn on a page, with
  its matrix, rectangle, rotation, effective resolution and compositing state.
- **Minimum supported Rust version: 1.86.** Proved by a CI job that builds and runs the
  suite against it, not asserted in metadata alone.
- **Robustness against untrusted input.** Every fixture PDF round-trips through
  open/save/reopen, including a second pass over the library's own output. A sweep of
  deliberately corrupted documents asserts the parser returns an error rather than
  aborting; the crate is built with `#![forbid(unsafe_code)]`, so a memory-safety defect
  cannot become undefined behaviour.
- **No dependencies.** `[dependencies]` and `[dev-dependencies]` are both empty. Every
  foundation primitive — inflate, LZW, CCITT, JBIG2, DCT, PNG/BMP/TIFF codecs, CFF and
  TrueType parsing, the rasteriser, the crypto stack — is authored from its specification
  rather than delegated to a crate.

### Notes on scope

The crate mirrors canonical API *names and shapes*; method casing follows Rust idiom
(`snake_case` methods, `PascalCase` types). Some operations ship their public surface
without being wired to the engine — those panic, naming the capability that would
unblock them, and each says so in its rustdoc. They never report a quiet success.

That set is not arbitrary. The README's scope note places every one of them, and says
which are work the crate intends to do rather than a boundary it has drawn — that note
is the single place those positions are kept, so this entry does not restate them.
Where a property is carried but not written, its rustdoc says so and says what
is missing — a second object the writer has no place for, a stage an encoder does not
have, an entry the format supersedes — and a test holds each of those notes down.
