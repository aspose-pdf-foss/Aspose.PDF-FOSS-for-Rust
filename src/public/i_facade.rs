//! `Aspose.Pdf.Facades.IFacade` — the base facade interface, per
//! `capabilities/i_facade.yaml`.
//!
//! A facade binds to a PDF (by path, bytes, or an in-memory
//! [`Document`]) and operates on it. Rust models the interface as a
//! trait with default methods over a shared `FacadeBase`; the
//! concrete facades embed a `FacadeBase` and gain the surface via
//! the `impl_facade!` macro. `IDisposable.Dispose` maps to
//! `close`.

use crate::public::document::Document;
use crate::public::error::Error;
use std::path::Path;

/// Shared state for a facade: the bound document (crate-internal).
#[derive(Debug, Default)]
pub(crate) struct FacadeBase {
    pub document: Option<Document>,
}

pub(crate) mod sealed {
    use super::FacadeBase;
    pub(crate) trait HasFacadeBase {
        fn facade_base(&self) -> &FacadeBase;
        fn facade_base_mut(&mut self) -> &mut FacadeBase;
    }
}

/// The base facade surface: bind a PDF and release it.
#[allow(private_bounds)]
pub trait IFacade: sealed::HasFacadeBase {
    /// Bind the facade to a PDF file on disk.
    fn bind_pdf(&mut self, path: impl AsRef<Path>) -> Result<(), Error> {
        let doc = Document::open(path)?;
        self.facade_base_mut().document = Some(doc);
        Ok(())
    }

    /// Bind the facade to an in-memory PDF byte buffer.
    fn bind_pdf_bytes(&mut self, bytes: &[u8]) -> Result<(), Error> {
        let doc = Document::from_bytes(bytes.to_vec())?;
        self.facade_base_mut().document = Some(doc);
        Ok(())
    }

    /// Bind the facade to an already-parsed [`Document`].
    fn bind_pdf_document(&mut self, document: Document) {
        self.facade_base_mut().document = Some(document);
    }

    /// Release the bound document.
    fn close(&mut self) {
        self.facade_base_mut().document = None;
    }
}

/// Implement [`sealed::HasFacadeBase`] for a facade with a `base:
/// FacadeBase` field.
macro_rules! impl_facade {
    ($ty:ty) => {
        impl $crate::public::i_facade::sealed::HasFacadeBase for $ty {
            fn facade_base(&self) -> &$crate::public::i_facade::FacadeBase {
                &self.base
            }
            fn facade_base_mut(
                &mut self,
            ) -> &mut $crate::public::i_facade::FacadeBase {
                &mut self.base
            }
        }
        impl $crate::public::i_facade::IFacade for $ty {}
        impl $ty {
            /// The bound document, if any (the canonical
            /// `Facade.Document` property).
            pub fn document(&self) -> Option<&$crate::public::document::Document> {
                use $crate::public::i_facade::sealed::HasFacadeBase as _;
                self.facade_base().document.as_ref()
            }
            /// Release the bound document (the canonical
            /// `Facade.Dispose`).
            pub fn dispose(&mut self) {
                use $crate::public::i_facade::sealed::HasFacadeBase as _;
                self.facade_base_mut().document = None;
            }
        }
    };
}
pub(crate) use impl_facade;
