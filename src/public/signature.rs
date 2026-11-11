//! `Aspose.Pdf.Forms.Signature` — the base of the digital-signature
//! family, per `capabilities/signature.yaml`.
//!
//! Ships the signature metadata + appearance surface. The
//! cryptographic configuration (X509 / digest / validation /
//! timestamp / OCSP / custom sign-hash) and the crypto `Verify`
//! overloads are deferred to the encryption beat. The concrete
//! PKCS1 / PKCS7 / PKCS7Detached / ExternalSignature subtypes are
//! deferred with them.
//!
//! **What reaches a signed file, and what does not.** The signing call
//! writes `/Name`, `/Reason`, `/Location` and `/ContactInfo` from its own
//! arguments, as the canonical call shape has it, and reads two things
//! from here: [`date`](Signature::date) becomes `/M` (§12.8.1 Table 252)
//! and [`default_signature_length`](Signature::default_signature_length)
//! sizes the `/Contents` placeholder. A date left unset writes no `/M`
//! at all — the alternative is stamping every signed document with a
//! time the caller never stated.
//!
//! **Carried, not written.** [`authority`](Signature::authority) names a
//! timestamp authority and [`use_ltv`](Signature::use_ltv) asks for
//! long-term validation; both mean fetching data from a service at
//! signing time, which this crate does not do.
//! [`avoid_estimating_signature_length`](Signature::avoid_estimating_signature_length)
//! turns off an estimate that is never made — the placeholder is always
//! the length asked for — so the flag describes what already happens.
//! [`show_properties`](Signature::show_properties) and
//! [`custom_appearance`](Signature::custom_appearance) describe a
//! signature appearance stream, a second object this signer does not
//! emit. [`byte_range`](Signature::byte_range) is read off a signed
//! file rather than given to one.

use crate::public::date_time::DateTime;
use crate::public::signature_custom_appearance::SignatureCustomAppearance;

/// A digital signature's metadata and appearance.
#[derive(Debug, Clone, PartialEq)]
pub struct Signature {
    custom_appearance: SignatureCustomAppearance,
    authority: String,
    location: String,
    reason: String,
    contact_info: String,
    byte_range: Vec<i32>,
    use_ltv: bool,
    avoid_estimating_signature_length: bool,
    show_properties: bool,
    default_signature_length: i32,
    date: DateTime,
}

impl Default for Signature {
    fn default() -> Self {
        Self {
            custom_appearance: SignatureCustomAppearance::new(),
            authority: String::new(),
            location: String::new(),
            reason: String::new(),
            contact_info: String::new(),
            byte_range: Vec::new(),
            use_ltv: false,
            avoid_estimating_signature_length: false,
            show_properties: true,
            default_signature_length: 3000,
            date: DateTime::default(),
        }
    }
}

impl Signature {
    /// Construct an empty signature.
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct from a PFX certificate file path and its password.
    pub fn with_certificate(_pfx_path: String, _password: String) -> Self {
        Self::default()
    }

    /// Verify the signature. Always returns `false` in the in-memory
    /// model (cryptographic verification is deferred).
    pub fn verify(&self) -> bool {
        false
    }

    /// The signing date (canonical `Date`).
    pub fn date(&self) -> DateTime {
        self.date
    }
    /// Set the signing date.
    pub fn set_date(&mut self, value: DateTime) {
        self.date = value;
    }

    /// The custom visual appearance.
    pub fn custom_appearance(&self) -> &SignatureCustomAppearance { &self.custom_appearance }
    /// Set the custom visual appearance.
    pub fn set_custom_appearance(&mut self, value: SignatureCustomAppearance) { self.custom_appearance = value; }

    /// The certifying authority name.
    pub fn authority(&self) -> &str { &self.authority }
    /// Set the certifying authority name.
    pub fn set_authority(&mut self, value: String) { self.authority = value; }

    /// The signing location.
    pub fn location(&self) -> &str { &self.location }
    /// Set the signing location.
    pub fn set_location(&mut self, value: String) { self.location = value; }

    /// The signing reason.
    pub fn reason(&self) -> &str { &self.reason }
    /// Set the signing reason.
    pub fn set_reason(&mut self, value: String) { self.reason = value; }

    /// The signer contact info.
    pub fn contact_info(&self) -> &str { &self.contact_info }
    /// Set the signer contact info.
    pub fn set_contact_info(&mut self, value: String) { self.contact_info = value; }

    /// The signed byte range.
    pub fn byte_range(&self) -> &[i32] { &self.byte_range }

    /// Whether long-term validation (LTV) is enabled.
    pub fn use_ltv(&self) -> bool { self.use_ltv }
    /// Set whether long-term validation is enabled.
    pub fn set_use_ltv(&mut self, value: bool) { self.use_ltv = value; }

    /// Whether to skip estimating the signature length.
    pub fn avoid_estimating_signature_length(&self) -> bool { self.avoid_estimating_signature_length }
    /// Set whether to skip estimating the signature length.
    pub fn set_avoid_estimating_signature_length(&mut self, value: bool) { self.avoid_estimating_signature_length = value; }

    /// Whether the signature properties dialog is shown.
    pub fn show_properties(&self) -> bool { self.show_properties }
    /// Set whether the signature properties dialog is shown.
    pub fn set_show_properties(&mut self, value: bool) { self.show_properties = value; }

    /// The default reserved signature length, in bytes.
    pub fn default_signature_length(&self) -> i32 { self.default_signature_length }
    /// Set the default reserved signature length.
    pub fn set_default_signature_length(&mut self, value: i32) { self.default_signature_length = value; }
}
