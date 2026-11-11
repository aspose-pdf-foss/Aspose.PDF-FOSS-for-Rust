//! `Aspose.Pdf.Permissions` — the document access-permission flags, per
//! `capabilities/permissions.yaml`.
//!
//! A `[Flags]` enum whose members are combinable permission bits, so it
//! is modelled as a bit-flag newtype with associated constants and the
//! bitwise operators (matching the [`BorderSide`](crate::BorderSide)
//! precedent). The raw bits map to the standard PDF `/Encrypt /P` flags
//! and to [`Document::permissions`](crate::Document::permissions).

use std::ops::{BitAnd, BitOr};

/// A set of document access permissions (bit flags).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Permissions(i32);

impl Permissions {
    /// No permissions.
    pub const NONE: Permissions = Permissions(0);
    /// Print the document.
    pub const PRINT_DOCUMENT: Permissions = Permissions(4);
    /// Modify the document content.
    pub const MODIFY_CONTENT: Permissions = Permissions(8);
    /// Extract (copy) content.
    pub const EXTRACT_CONTENT: Permissions = Permissions(16);
    /// Add or modify text annotations / fill form fields.
    pub const MODIFY_TEXT_ANNOTATIONS: Permissions = Permissions(32);
    /// Fill in form fields.
    pub const FILL_FORM: Permissions = Permissions(256);
    /// Extract content for accessibility.
    pub const EXTRACT_CONTENT_WITH_DISABILITIES: Permissions = Permissions(512);
    /// Assemble the document (insert / rotate / delete pages).
    pub const ASSEMBLE_DOCUMENT: Permissions = Permissions(1024);
    /// High-quality printing.
    pub const PRINTING_QUALITY: Permissions = Permissions(2048);

    /// Construct from raw flag bits.
    pub fn from_bits(bits: i32) -> Self {
        Permissions(bits)
    }

    /// The raw flag bits.
    pub fn bits(self) -> i32 {
        self.0
    }

    /// Whether every flag in `other` is set in `self`.
    pub fn contains(self, other: Permissions) -> bool {
        (self.0 & other.0) == other.0
    }
}

impl BitOr for Permissions {
    type Output = Permissions;
    fn bitor(self, rhs: Permissions) -> Permissions {
        Permissions(self.0 | rhs.0)
    }
}

impl BitAnd for Permissions {
    type Output = Permissions;
    fn bitand(self, rhs: Permissions) -> Permissions {
        Permissions(self.0 & rhs.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_combine() {
        assert_eq!(Permissions::PRINT_DOCUMENT.bits(), 4);
        assert_eq!(Permissions::PRINTING_QUALITY.bits(), 2048);
        let pm = Permissions::PRINT_DOCUMENT | Permissions::MODIFY_CONTENT;
        assert_eq!(pm.bits(), 12);
        assert!(pm.contains(Permissions::PRINT_DOCUMENT));
        assert!(!pm.contains(Permissions::FILL_FORM));
        assert_eq!(Permissions::default(), Permissions::NONE);
        assert_eq!(Permissions::from_bits(12), pm);
    }
}
