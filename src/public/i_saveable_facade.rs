//! `Aspose.Pdf.Facades.ISaveableFacade` — a facade that can persist
//! its bound document, per `capabilities/i_saveable_facade.yaml`.
//!
//! Extends [`IFacade`] with save entry points,
//! provided as default methods over the bound document.

use crate::public::error::Error;
use crate::public::i_facade::IFacade;
use std::io::Write;
use std::path::Path;

/// A facade whose bound document can be saved.
#[allow(private_bounds)]
pub trait ISaveableFacade: IFacade {
    /// Save the bound document to a file path.
    fn save(&self, path: impl AsRef<Path>) -> Result<(), Error> {
        match self.facade_base().document.as_ref() {
            Some(doc) => doc.save_to(path),
            None => Err(Error::InvalidPdf("facade has no bound document".into())),
        }
    }

    /// Save the bound document to a byte sink.
    fn save_to_writer(&self, writer: impl Write) -> Result<(), Error> {
        match self.facade_base().document.as_ref() {
            Some(doc) => doc.save_to_writer(writer),
            None => Err(Error::InvalidPdf("facade has no bound document".into())),
        }
    }
}

/// Implement [`ISaveableFacade`] for a facade (requires
/// `impl_facade!` first).
macro_rules! impl_saveable_facade {
    ($ty:ty) => {
        impl $crate::public::i_saveable_facade::ISaveableFacade for $ty {}
    };
}
pub(crate) use impl_saveable_facade;
