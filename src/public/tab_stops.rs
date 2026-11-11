//! `Aspose.Pdf.Text.TabStops` — a paragraph's ordered ruler of
//! [`TabStop`]s, per `capabilities/tab_stops.yaml`.
//!
//! Built up with the [`add`](TabStops::add) family and read back by
//! [`count`](TabStops::count) / [`get`](TabStops::get) (the canonical
//! indexer). The read-only flag and `ICloneable.Clone` are out of the
//! shipped subset.

use crate::public::tab_leader_type::TabLeaderType;
use crate::public::tab_stop::TabStop;

/// An ordered collection of [`TabStop`]s.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TabStops {
    items: Vec<TabStop>,
}

impl TabStops {
    /// An empty tab ruler.
    pub fn new() -> Self {
        Self::default()
    }

    /// The number of tab stops.
    pub fn count(&self) -> usize {
        self.items.len()
    }

    /// The tab stop at `index` (the canonical indexer getter), or
    /// `None` when out of range.
    pub fn get(&self, index: usize) -> Option<&TabStop> {
        self.items.get(index)
    }

    /// Replace the tab stop at `index` (the canonical indexer setter).
    /// Out-of-range indices are ignored.
    pub fn set(&mut self, index: usize, value: TabStop) {
        if let Some(slot) = self.items.get_mut(index) {
            *slot = value;
        }
    }

    /// Append a default tab stop and return it for further
    /// configuration.
    pub fn add(&mut self) -> &mut TabStop {
        self.items.push(TabStop::new());
        self.items.last_mut().unwrap()
    }

    /// Append a tab stop at `position` points and return it.
    pub fn add_at_position(&mut self, position: f32) -> &mut TabStop {
        self.items.push(TabStop::with_position(position));
        self.items.last_mut().unwrap()
    }

    /// Append an existing tab stop.
    pub fn add_tab_stop(&mut self, tab_stop: TabStop) {
        self.items.push(tab_stop);
    }

    /// Append a tab stop at `position` points with `leader_type` and
    /// return it.
    pub fn add_with_leader(
        &mut self,
        position: f32,
        leader_type: TabLeaderType,
    ) -> &mut TabStop {
        let mut ts = TabStop::with_position(position);
        ts.set_leader_type(leader_type);
        self.items.push(ts);
        self.items.last_mut().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::public::tab_alignment_type::TabAlignmentType;

    #[test]
    fn build_and_read_back() {
        let mut stops = TabStops::new();
        assert_eq!(stops.count(), 0);
        assert!(stops.get(0).is_none());

        stops.add().set_alignment_type(TabAlignmentType::Right);
        stops.add_at_position(72.0);
        stops.add_with_leader(144.0, TabLeaderType::Dot);
        stops.add_tab_stop(TabStop::with_position(216.0));

        assert_eq!(stops.count(), 4);
        assert_eq!(stops.get(0).unwrap().alignment_type(), TabAlignmentType::Right);
        assert_eq!(stops.get(1).unwrap().position(), 72.0);
        assert_eq!(stops.get(2).unwrap().leader_type(), TabLeaderType::Dot);
        assert_eq!(stops.get(3).unwrap().position(), 216.0);
        assert!(stops.get(4).is_none());
    }

    #[test]
    fn indexer_set_replaces() {
        let mut stops = TabStops::new();
        stops.add_at_position(10.0);
        stops.set(0, TabStop::with_position(20.0));
        assert_eq!(stops.get(0).unwrap().position(), 20.0);
        // Out-of-range set is a no-op.
        stops.set(5, TabStop::with_position(99.0));
        assert_eq!(stops.count(), 1);
    }
}
