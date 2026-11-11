//! `Aspose.Pdf.Facades.SaveableFacade` — the concrete base for
//! facades that persist their document, per
//! `capabilities/saveable_facade.yaml`.
//!
//! Extends [`Facade`](crate::Facade) with the
//! [`ISaveableFacade`](crate::ISaveableFacade) save surface.

use crate::public::i_facade::{impl_facade, FacadeBase};
use crate::public::i_saveable_facade::impl_saveable_facade;

/// The concrete saveable-facade base.
#[derive(Debug, Default)]
pub struct SaveableFacade {
    base: FacadeBase,
}

impl SaveableFacade {
    /// Construct an unbound saveable facade.
    pub fn new() -> Self {
        Self::default()
    }
}

impl_facade!(SaveableFacade);
impl_saveable_facade!(SaveableFacade);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::public::i_facade::IFacade;
    use crate::public::i_saveable_facade::ISaveableFacade;

    #[test]
    fn bind_and_save() {
        let mut f = SaveableFacade::new();
        f.bind_pdf_document(crate::public::document::Document::new());
        let mut out = Vec::new();
        f.save_to_writer(&mut out).unwrap();
        assert!(out.starts_with(b"%PDF"));
    }
}
