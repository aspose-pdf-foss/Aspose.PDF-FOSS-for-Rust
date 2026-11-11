//! A calendar date-time value — the Rust carrier for the BCL
//! `System.DateTime` used by Aspose date members (`Modified`,
//! `CreationDate`, `Value`, `Date`, `XmpValue`).
//!
//! Std-only and from-scratch (no `chrono` / `time` crate), honouring
//! the Rust track's no-external-crate lock. It mirrors the calendar
//! component surface a value carrier needs; it is not a full
//! `System.DateTime` port (no arithmetic, no time zone).

/// A calendar date-time: year, month, day, hour, minute, second.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DateTime {
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
}

impl DateTime {
    /// Construct from calendar components (mirrors the canonical
    /// `DateTime(year, month, day, hour, minute, second)` constructor).
    pub fn new(year: i32, month: u32, day: u32, hour: u32, minute: u32, second: u32) -> Self {
        DateTime {
            year,
            month,
            day,
            hour,
            minute,
            second,
        }
    }

    /// The year (canonical `Year`).
    pub fn year(&self) -> i32 {
        self.year
    }
    /// The month, 1–12 (canonical `Month`).
    pub fn month(&self) -> u32 {
        self.month
    }
    /// The day of the month (canonical `Day`).
    pub fn day(&self) -> u32 {
        self.day
    }
    /// The hour, 0–23 (canonical `Hour`).
    pub fn hour(&self) -> u32 {
        self.hour
    }
    /// The minute, 0–59 (canonical `Minute`).
    pub fn minute(&self) -> u32 {
        self.minute
    }
    /// The second, 0–59 (canonical `Second`).
    pub fn second(&self) -> u32 {
        self.second
    }

    /// This date as a PDF date string (`D:YYYYMMDDHHmmSSZ`, §7.9.4),
    /// or `None` when there is no date to write. Month zero is the
    /// marker: the default value has no month, and no calendar month is
    /// zero, so a `Default`-constructed date is distinguishable from
    /// any real one. Crate-internal — the counterpart of
    /// [`from_pdf_date`](Self::from_pdf_date).
    ///
    /// The time zone is written as `Z`: this carrier has no zone (see
    /// the module note), so the components are taken as UTC rather than
    /// a local time this crate cannot know.
    pub(crate) fn to_pdf_date(self) -> Option<String> {
        if self.month == 0 || self.day == 0 {
            return None;
        }
        Some(format!(
            "D:{:04}{:02}{:02}{:02}{:02}{:02}Z",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        ))
    }
    /// Parse a PDF date string (`D:YYYYMMDDHHmmSS…`, with the `D:`
    /// prefix and trailing time-zone offset both optional) into a
    /// [`DateTime`]. Missing trailing components default to their zero
    /// value; an unparseable string yields the default (all-zero) date.
    /// Crate-internal — used when loading `/Params` / `/Info` dates.
    pub(crate) fn from_pdf_date(s: &str) -> DateTime {
        let d = s.strip_prefix("D:").unwrap_or(s);
        let digits: Vec<u8> = d.bytes().take_while(|b| b.is_ascii_digit()).collect();
        let field = |start: usize, len: usize, default: i64| -> i64 {
            if digits.len() >= start + len {
                std::str::from_utf8(&digits[start..start + len])
                    .ok()
                    .and_then(|t| t.parse::<i64>().ok())
                    .unwrap_or(default)
            } else {
                default
            }
        };
        if digits.len() < 4 {
            return DateTime::default();
        }
        DateTime::new(
            field(0, 4, 0) as i32,
            field(4, 2, 1) as u32,
            field(6, 2, 1) as u32,
            field(8, 2, 0) as u32,
            field(10, 2, 0) as u32,
            field(12, 2, 0) as u32,
        )
    }
}
