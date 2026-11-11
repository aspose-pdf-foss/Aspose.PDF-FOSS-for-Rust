//! `Aspose.Pdf.Text.FontRepository` — font lookup and loading, per
//! `capabilities/font_repository.yaml`.
//!
//! A strict subset: the name-resolving `find_font(name)` and the
//! TrueType file load `open_font(path)`. The `Stream` + `FontTypes`
//! overload, the `path` + AFM-metrics overload, the canonical
//! constructor, and the substitution / sources surface are dropped
//! (see `targets/rust/drops.yaml`).

use crate::internal::truetype;
use crate::public::error::Error;
use crate::public::font::Font;
use std::path::Path;

/// Font lookup and loading. The members are static — there is no
/// `FontRepository` instance to construct.
pub struct FontRepository;

impl FontRepository {
    /// Resolve a font by name. The returned [`Font`] carries the
    /// name as its base font and is not embedded.
    pub fn find_font(font_name: &str) -> Font {
        Font::resolved(font_name)
    }

    /// Load an embeddable TrueType font from a file.
    ///
    /// Returns [`Error::Io`] if the file cannot be read and
    /// [`Error::InvalidFont`] if the bytes are not a valid TrueType
    /// program. The returned [`Font`] is embedded by default and
    /// carries the font program for the drawing / save pipeline.
    pub fn open_font(font_file_path: impl AsRef<Path>) -> Result<Font, Error> {
        let path = font_file_path.as_ref();
        let bytes = std::fs::read(path)?;
        let parsed = truetype::parse(&bytes)
            .map_err(|e| Error::InvalidFont(e.to_string()))?;
        let name = if parsed.name.is_empty() {
            // Fall back to the file stem when the font carries no
            // usable `name` record, so the base font is never empty.
            path.file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default()
        } else {
            parsed.name
        };
        Ok(Font::from_program(name, bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn liberation_ttf() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/internal/standard14_outlines.fonts/LiberationSans-Regular.ttf")
    }

    #[test]
    fn find_font_resolves_by_name() {
        let f = FontRepository::find_font("Helvetica");
        assert_eq!(f.font_name(), "Helvetica");
        assert_eq!(f.base_font(), "Helvetica");
        assert!(!f.is_embedded());
        assert!(!f.is_subset());
    }

    #[test]
    fn open_font_loads_embeddable_font() {
        let f = FontRepository::open_font(liberation_ttf()).unwrap();
        assert!(f.is_embedded());
        assert!(!f.font_name().is_empty());
        assert!(f.program.is_some());
    }

    #[test]
    fn open_font_errors_on_bad_input() {
        // A missing file is an I/O error.
        assert!(matches!(
            FontRepository::open_font("/no/such/font.ttf"),
            Err(Error::Io(_))
        ));
        // A non-font byte stream is an invalid-font error.
        let tmp = std::env::temp_dir().join("asp_rust_notafont.bin");
        std::fs::write(&tmp, b"%PDF-1.7 not a font").unwrap();
        assert!(matches!(
            FontRepository::open_font(&tmp),
            Err(Error::InvalidFont(_))
        ));
        let _ = std::fs::remove_file(&tmp);
    }
}
