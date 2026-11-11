//! `Aspose.Pdf.Facades.KeySize` — the encryption key length used by
//! `PdfFileSecurity`, per `capabilities/key_size.yaml`. Ordinals match
//! canonical 26.4.0.

/// An encryption key length.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum KeySize {
    /// 40-bit key (RC4 only).
    #[default]
    x40 = 0,
    /// 128-bit key.
    x128 = 1,
    /// 256-bit key (AES only).
    x256 = 2,
}
