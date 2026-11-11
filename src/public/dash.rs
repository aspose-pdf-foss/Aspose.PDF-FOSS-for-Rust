//! `Aspose.Pdf.Annotations.Dash` — a dash pattern for annotation
//! borders, per `capabilities/dash.yaml`.
//!
//! A value type describing a dash: the length of the "on" (drawn) and
//! "off" (gap) segments. Built from an on/off pair or from an explicit
//! pattern array; [`on`](Dash::on) / [`off`](Dash::off) address the
//! first two pattern entries, and [`pattern`](Dash::pattern) exposes
//! the whole array.

/// A dash pattern (on / off segment lengths).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Dash {
    pattern: Vec<i32>,
}

impl Dash {
    /// Construct a dash from an on (drawn) and off (gap) length.
    pub fn new(on: i32, off: i32) -> Self {
        Self { pattern: vec![on, off] }
    }

    /// Construct a dash from an explicit pattern array.
    pub fn with_pattern(pattern: Vec<i32>) -> Self {
        Self { pattern }
    }

    /// The on (drawn) segment length — the first pattern entry.
    pub fn on(&self) -> i32 {
        self.pattern.first().copied().unwrap_or(0)
    }
    /// Set the on (drawn) segment length.
    pub fn set_on(&mut self, value: i32) {
        self.ensure_len(1);
        self.pattern[0] = value;
    }

    /// The off (gap) segment length — the second pattern entry.
    pub fn off(&self) -> i32 {
        self.pattern.get(1).copied().unwrap_or(0)
    }
    /// Set the off (gap) segment length.
    pub fn set_off(&mut self, value: i32) {
        self.ensure_len(2);
        self.pattern[1] = value;
    }

    /// The full dash pattern.
    pub fn pattern(&self) -> &[i32] {
        &self.pattern
    }

    /// Grow the pattern with zeros so index `len - 1` is addressable.
    fn ensure_len(&mut self, len: usize) {
        while self.pattern.len() < len {
            self.pattern.push(0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn on_off_ctor() {
        let d = Dash::new(3, 2);
        assert_eq!(d.on(), 3);
        assert_eq!(d.off(), 2);
        assert_eq!(d.pattern(), &[3, 2]);
    }

    #[test]
    fn pattern_ctor() {
        let d = Dash::with_pattern(vec![4, 1, 2, 1]);
        assert_eq!(d.on(), 4);
        assert_eq!(d.off(), 1);
        assert_eq!(d.pattern(), &[4, 1, 2, 1]);
    }

    #[test]
    fn setters_round_trip() {
        let mut d = Dash::new(1, 1);
        d.set_on(5);
        d.set_off(6);
        assert_eq!(d.on(), 5);
        assert_eq!(d.off(), 6);
    }

    #[test]
    fn setters_grow_empty_pattern() {
        let mut d = Dash::default();
        d.set_off(7);
        assert_eq!(d.on(), 0);
        assert_eq!(d.off(), 7);
    }
}
