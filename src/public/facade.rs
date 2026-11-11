//! `Aspose.Pdf.Facades.Facade` — the concrete base for the facade
//! family, per `capabilities/facade.yaml`.
//!
//! Holds the bound document and provides the
//! [`IFacade`](crate::IFacade) surface (`bind_pdf*` / `close`) plus
//! the `document` accessor and `dispose`. The concrete facades
//! embed a `FacadeBase` and
//! reuse the same surface via the `impl_facade!` macro.

use crate::public::i_facade::{impl_facade, FacadeBase};

/// The concrete facade base.
#[derive(Debug, Default)]
pub struct Facade {
    base: FacadeBase,
}

impl Facade {
    /// Construct an unbound facade.
    pub fn new() -> Self {
        Self::default()
    }
}

impl_facade!(Facade);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::public::i_facade::IFacade;

    #[test]
    fn bind_and_close() {
        let mut f = Facade::new();
        assert!(f.document().is_none());
        f.bind_pdf_document(crate::public::document::Document::new());
        assert!(f.document().is_some());
        f.close();
        assert!(f.document().is_none());
        f.bind_pdf_document(crate::public::document::Document::new());
        f.dispose();
        assert!(f.document().is_none());
    }
}
