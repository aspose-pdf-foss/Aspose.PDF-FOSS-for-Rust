//! `Aspose.Pdf.Facades.SignatureName` — the name of a signature field,
//! per `capabilities/signature_name.yaml`.
//!
//! A value type carrying a signature field's partial [`name`](SignatureName::name)
//! and fully-qualified [`full_name`](SignatureName::full_name), plus a
//! [`has_signature`](SignatureName::has_signature) flag set by the
//! `PdfFileSignature` enumeration. The `Equals(System.Object)` comparator
//! is dropped (non-idiomatic in Rust; equality is via the fields).

/// The name of a signature field.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SignatureName {
    /// The field's partial name.
    pub name: String,
    /// The field's fully-qualified name.
    pub full_name: String,
    pub(crate) has_signature: bool,
}

impl SignatureName {
    /// Construct an empty signature name.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether this name refers to a signed field (vs a blank signature
    /// field).
    pub fn has_signature(&self) -> bool {
        self.has_signature
    }

    /// A stable hash of the fully-qualified name.
    pub fn get_hash_code(&self) -> i32 {
        // FNV-1a over the full name, truncated to i32.
        let mut h: u32 = 0x811c_9dc5;
        for b in self.full_name.bytes() {
            h ^= b as u32;
            h = h.wrapping_mul(0x0100_0193);
        }
        h as i32
    }
}

impl std::fmt::Display for SignatureName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.full_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields_and_display() {
        let mut s = SignatureName::new();
        assert!(!s.has_signature());
        s.name = "Signature1".into();
        s.full_name = "form.Signature1".into();
        assert_eq!(s.to_string(), "form.Signature1");
        assert_eq!(s.get_hash_code(), SignatureName { full_name: "form.Signature1".into(), ..Default::default() }.get_hash_code());
    }
}
