//! `Aspose.Pdf.Id` — the document's `/ID` file identifier, per
//! `capabilities/id.yaml`.
//!
//! The PDF `/ID` trailer entry is an array of two byte strings: a
//! permanent identifier ([`original`](Id::original)) fixed at creation,
//! and a changing identifier ([`modified`](Id::modified)) updated on
//! each save. Both are exposed as their uppercase-hex representation
//! (file identifiers are binary); an absent `/ID` yields empty strings.

/// The document's `/ID` file identifier pair.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Id {
    original: String,
    modified: String,
}

impl Id {
    /// Build an [`Id`] from the raw `/ID` array byte strings. Absent
    /// elements become empty strings. Crate-internal — reached via
    /// [`Document::id`](crate::Document::id).
    pub(crate) fn from_bytes(
        original: Option<&[u8]>,
        modified: Option<&[u8]>,
    ) -> Self {
        Self {
            original: original.map(to_hex).unwrap_or_default(),
            modified: modified.map(to_hex).unwrap_or_default(),
        }
    }

    /// The permanent file identifier, as uppercase hex.
    pub fn original(&self) -> &str {
        &self.original
    }

    /// The changing file identifier, as uppercase hex.
    pub fn modified(&self) -> &str {
        &self.modified
    }
}

/// Uppercase-hex encode a byte slice.
fn to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(char::from_digit((b >> 4) as u32, 16).unwrap().to_ascii_uppercase());
        s.push(char::from_digit((b & 0x0f) as u32, 16).unwrap().to_ascii_uppercase());
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_encodes_both_elements() {
        let id = Id::from_bytes(Some(&[0x00, 0x11, 0xAB]), Some(&[0xFF, 0xCD]));
        assert_eq!(id.original(), "0011AB");
        assert_eq!(id.modified(), "FFCD");
    }

    #[test]
    fn absent_elements_are_empty() {
        let id = Id::from_bytes(None, None);
        assert_eq!(id.original(), "");
        assert_eq!(id.modified(), "");
    }
}
