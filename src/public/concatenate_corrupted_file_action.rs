//! `Aspose.Pdf.Facades.PdfFileEditor.ConcatenateCorruptedFileAction` —
//! how `PdfFileEditor` concatenation treats a corrupted input file, per
//! `capabilities/concatenate_corrupted_file_action.yaml`.
//!
//! A public nested enum of `Aspose.Pdf.Facades.PdfFileEditor`. The
//! `CorruptedFileAction` property that selects it stays outside the
//! shipped `PdfFileEditor` subset (file-path core ops only), so this
//! enum ships as reference surface (the `ColorSpace` / `DigestHashAlgorithm`
//! precedent). Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// How concatenation handles a corrupted source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum ConcatenateCorruptedFileAction {
    /// Stop and raise an error on the first corrupted file.
    #[default]
    StopWithError = 0,
    /// Skip the corrupted file and continue concatenating.
    ConcatenateIgnoringCorrupted = 1,
    /// Concatenate the file while ignoring its corrupted objects.
    ConcatenateIgnoringCorruptedObjects = 2,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinals_and_default() {
        assert_eq!(ConcatenateCorruptedFileAction::default(), ConcatenateCorruptedFileAction::StopWithError);
        assert_eq!(ConcatenateCorruptedFileAction::StopWithError as i32, 0);
        assert_eq!(ConcatenateCorruptedFileAction::ConcatenateIgnoringCorrupted as i32, 1);
        assert_eq!(ConcatenateCorruptedFileAction::ConcatenateIgnoringCorruptedObjects as i32, 2);
    }
}
