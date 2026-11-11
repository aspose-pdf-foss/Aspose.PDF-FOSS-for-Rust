//! `Aspose.Pdf.Forms.Form.FlattenSettings` — options for flattening a
//! form (a public nested type of `Aspose.Pdf.Forms.Form`), per
//! `capabilities/flatten_settings.yaml`.
//!
//! **Carried, not consulted.** Flattening itself now works, but it is
//! *structural*: [`Form::flatten`](crate::Form::flatten) drops the
//! fields and the save removes the catalog's `/AcroForm`, and the
//! annotation editors remove the annotations. These four options steer
//! the part that is missing —
//! [`update_appearances`](FlattenSettings::update_appearances) asks for
//! appearances to be regenerated first,
//! [`call_events`](FlattenSettings::call_events) for the fields'
//! scripts to run, [`hide_buttons`](FlattenSettings::hide_buttons) for
//! buttons to be dropped rather than drawn, and
//! [`apply_redactions`](FlattenSettings::apply_redactions) for a
//! redaction to black out what it covers. Each needs something drawn
//! into the page content, which is the appearance generator this crate
//! does not have; with nothing drawn, there is nothing for them to
//! change.

/// Options controlling how a form is flattened into page content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FlattenSettings {
    update_appearances: bool,
    call_events: bool,
    hide_buttons: bool,
    apply_redactions: bool,
}

impl FlattenSettings {
    /// Construct default flatten settings.
    pub fn new() -> Self { Self::default() }
    /// Whether appearances are regenerated before flattening.
    pub fn update_appearances(&self) -> bool { self.update_appearances }
    /// Set whether appearances are regenerated.
    pub fn set_update_appearances(&mut self, v: bool) { self.update_appearances = v; }
    /// Whether field events fire during flattening.
    pub fn call_events(&self) -> bool { self.call_events }
    /// Set whether field events fire.
    pub fn set_call_events(&mut self, v: bool) { self.call_events = v; }
    /// Whether buttons are hidden during flattening.
    pub fn hide_buttons(&self) -> bool { self.hide_buttons }
    /// Set whether buttons are hidden.
    pub fn set_hide_buttons(&mut self, v: bool) { self.hide_buttons = v; }
    /// Whether redactions are applied during flattening.
    pub fn apply_redactions(&self) -> bool { self.apply_redactions }
    /// Set whether redactions are applied.
    pub fn set_apply_redactions(&mut self, v: bool) { self.apply_redactions = v; }
}
