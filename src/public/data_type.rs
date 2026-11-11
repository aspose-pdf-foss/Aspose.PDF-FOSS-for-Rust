//! `Aspose.Pdf.Facades.DataType` — form import / export data format, per
//! `capabilities/data_type.yaml`. Ordinals match canonical 26.4.0.

/// A form import / export data format.
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum DataType {
    /// Forms Data Format.
    #[default]
    FDF = 0,
    /// XML.
    XML = 1,
    /// XML Forms Data Format.
    XFDF = 2,
    /// The full PDF.
    PDF = 3,
    /// OLE DB data source.
    OLEDB = 4,
    /// ODBC data source.
    ODBC = 5,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ordinals() {
        assert_eq!(DataType::FDF as i32, 0);
        assert_eq!(DataType::ODBC as i32, 5);
    }
}
