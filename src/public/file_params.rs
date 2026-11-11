//! `Aspose.Pdf.FileParams` — the embedded-file `/Params` metadata of a
//! [`FileSpecification`](crate::FileSpecification), per
//! `capabilities/file_params.yaml`.
//!
//! Carries the embedded file's uncompressed [`size`](FileParams::size),
//! [`creation_date`](FileParams::creation_date) /
//! [`mod_date`](FileParams::mod_date), and [`check_sum`](FileParams::check_sum).
//! Reached via [`FileSpecification::params`](crate::FileSpecification::params);
//! the `FileSpecification`-taking constructor has no thin-model
//! equivalent and is dropped.

use crate::public::date_time::DateTime;

/// The `/Params` metadata of an embedded file.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FileParams {
    size: i32,
    creation_date: DateTime,
    mod_date: DateTime,
    check_sum: String,
}

impl FileParams {
    /// Build the params from the parsed `/Params` values. Crate-internal
    /// — reached via [`FileSpecification::params`](crate::FileSpecification::params).
    pub(crate) fn from_parts(
        size: i32,
        creation_date: DateTime,
        mod_date: DateTime,
        check_sum: String,
    ) -> Self {
        Self { size, creation_date, mod_date, check_sum }
    }

    /// The uncompressed file size, in bytes (`0` when unknown).
    pub fn size(&self) -> i32 {
        self.size
    }

    /// The file's creation date.
    pub fn creation_date(&self) -> DateTime {
        self.creation_date
    }
    /// Set the file's creation date.
    pub fn set_creation_date(&mut self, value: DateTime) {
        self.creation_date = value;
    }

    /// The file's modification date.
    pub fn mod_date(&self) -> DateTime {
        self.mod_date
    }
    /// Set the file's modification date.
    pub fn set_mod_date(&mut self, value: DateTime) {
        self.mod_date = value;
    }

    /// The file's checksum (`/CheckSum`), or empty when absent.
    pub fn check_sum(&self) -> &str {
        &self.check_sum
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_empty() {
        let p = FileParams::default();
        assert_eq!(p.size(), 0);
        assert_eq!(p.check_sum(), "");
        assert_eq!(p.creation_date(), DateTime::default());
    }

    #[test]
    fn from_parts_and_setters() {
        let mut p = FileParams::from_parts(
            1234,
            DateTime::new(2026, 8, 7, 9, 30, 0),
            DateTime::default(),
            "ABCD".to_string(),
        );
        assert_eq!(p.size(), 1234);
        assert_eq!(p.check_sum(), "ABCD");
        assert_eq!(p.creation_date().year(), 2026);
        p.set_mod_date(DateTime::new(2026, 8, 7, 10, 0, 0));
        assert_eq!(p.mod_date().hour(), 10);
    }
}
