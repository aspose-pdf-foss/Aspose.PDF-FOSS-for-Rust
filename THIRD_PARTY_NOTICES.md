# Third-Party Notices

Aspose.PDF FOSS for Rust has **no third-party dependencies** — neither at runtime nor for
the test suite. `[dependencies]` and `[dev-dependencies]` in `Cargo.toml` are both empty,
and every codec (inflate, LZW, CCITT, JBIG2, DCT, PNG, BMP, TIFF), font parser (CFF,
TrueType) and cryptographic primitive is implemented from scratch against its
specification.

The following third-party material is bundled for the purpose noted below.

## Bundled

### Liberation Fonts (SIL Open Font License 1.1)

The library bundles the Liberation font family to provide metrically-compatible
substitutes for the Standard-14 PDF fonts (`Helvetica`, `Times-Roman`, `Courier`, and
their styles), so that a document referencing one without an embedded `/FontFile` still
renders. Any other non-embedded font is mapped onto the nearest of those names first, so
these faces stand in for it too.

- Upstream: https://github.com/liberationfonts/liberation-fonts
- License: SIL Open Font License, Version 1.1 — https://openfontlicense.org
- Copyright: © Red Hat, Inc., with Reserved Font Name "Liberation".

### Noto Sans (SIL Open Font License 1.1)

Three Noto Sans faces are bundled as **test fixtures**. No rendering path reads them and
nothing in the public API exposes them; they exist because the from-scratch SFNT/TrueType
parser is gated against real fonts rather than synthetic ones. Each covers a small script,
which is what keeps a fixture that exercises the whole table directory down to a few
kilobytes — simple and composite `glyf` outlines, a format-4 `cmap`, short and long
`loca`, and an `hmtx` that repeats its last advance.

- Files: `tests/fixtures/truetype/buginese.ttf`, `buhid.ttf`, `tagbanwa.ttf` — Noto Sans
  Buginese, Noto Sans Buhid and Noto Sans Tagbanwa, as published upstream at version
  2.000.
- Upstream: https://github.com/notofonts/noto-fonts
- License: SIL Open Font License, Version 1.1 — https://openfontlicense.org
- Copyright: © 2017 Google Inc. All Rights Reserved. Noto is a trademark of Google Inc.

## Licence text

Both bundled families carry the same licence, and its full text ships with the crate as
`src/internal/standard14_outlines.fonts/OFL.txt`. The Liberation upstream `AUTHORS` file
sits beside it.
