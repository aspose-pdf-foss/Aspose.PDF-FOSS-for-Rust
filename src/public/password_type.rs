//! `Aspose.Pdf.PasswordType` — which password opened a document, per
//! `capabilities/password_type.yaml`.

/// The kind of password a document was opened with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum PasswordType {
    /// No password (not encrypted, or opened without one).
    #[default]
    None = 0,
    /// Opened with the user (open) password.
    User = 1,
    /// Opened with the owner (permissions) password.
    Owner = 2,
    /// Encrypted but no usable password was supplied.
    Inaccessible = 3,
}
