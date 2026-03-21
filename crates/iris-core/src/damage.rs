/// A contiguous region of visible grid damage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DamageRegion {
    /// First damaged row, inclusive.
    pub start_row: usize,
    /// Last damaged row, inclusive.
    pub end_row: usize,
    /// First damaged column, inclusive.
    pub start_col: usize,
    /// Last damaged column, inclusive.
    pub end_col: usize,
}

impl DamageRegion {
    /// Creates a region from explicit coordinates.
    #[must_use]
    pub const fn new(start_row: usize, end_row: usize, start_col: usize, end_col: usize) -> Self {
        Self {
            start_row,
            end_row,
            start_col,
            end_col,
        }
    }
}

/// A visible grid scroll operation that can be consumed by renderers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScrollDelta {
    /// First affected row, inclusive.
    pub top: usize,
    /// Last affected row, inclusive.
    pub bottom: usize,
    /// Signed row delta applied to the region.
    ///
    /// Positive values mean content moved upward and new rows were exposed at
    /// the bottom edge. Negative values mean content moved downward and new
    /// rows were exposed at the top edge.
    pub lines: i32,
}

impl ScrollDelta {
    /// Creates a scroll delta from an affected row range and signed line count.
    #[must_use]
    pub const fn new(top: usize, bottom: usize, lines: i32) -> Self {
        Self { top, bottom, lines }
    }
}

/// Tracks which visible rows and columns changed since the last render pass.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DamageTracker {
    rows: Vec<Option<(usize, usize)>>,
    all_damaged: bool,
}

impl DamageTracker {
    /// Creates a tracker for the provided row count.
    #[must_use]
    pub fn new(rows: usize) -> Self {
        Self {
            rows: vec![None; rows],
            all_damaged: false,
        }
    }

    /// Resizes the internal row tracking to match the grid height.
    pub fn resize(&mut self, rows: usize) {
        self.rows.resize(rows, None);
        self.rows.fill(None);
        self.all_damaged = true;
    }

    /// Marks a single cell as damaged.
    pub fn mark(&mut self, row: usize, col: usize) {
        if let Some(range) = self.rows.get_mut(row) {
            match range {
                Some((start_col, end_col)) => {
                    *start_col = (*start_col).min(col);
                    *end_col = (*end_col).max(col);
                }
                slot @ None => *slot = Some((col, col)),
            }
        }
    }

    /// Marks an inclusive column range as damaged.
    pub fn mark_range(&mut self, row: usize, start_col: usize, end_col: usize) {
        if start_col > end_col {
            return;
        }

        if let Some(range) = self.rows.get_mut(row) {
            match range {
                Some((current_start, current_end)) => {
                    *current_start = (*current_start).min(start_col);
                    *current_end = (*current_end).max(end_col);
                }
                slot @ None => *slot = Some((start_col, end_col)),
            }
        }
    }

    /// Marks a complete row as damaged.
    pub fn mark_row(&mut self, row: usize, cols: usize) {
        if let Some(slot) = self.rows.get_mut(row) {
            if cols == 0 {
                *slot = None;
            } else {
                *slot = Some((0, cols.saturating_sub(1)));
            }
        }
    }

    /// Marks the entire grid as damaged.
    pub fn mark_all(&mut self) {
        self.all_damaged = true;
        self.rows.fill(None);
    }

    /// Returns `true` when any visible damage exists.
    #[must_use]
    pub fn is_damaged(&self) -> bool {
        self.all_damaged || self.rows.iter().any(Option::is_some)
    }

    /// Drains all pending damage into regions.
    pub fn take(&mut self, cols: usize) -> Vec<DamageRegion> {
        if self.all_damaged {
            self.all_damaged = false;
            if self.rows.is_empty() || cols == 0 {
                return Vec::new();
            }

            return vec![DamageRegion::new(
                0,
                self.rows.len().saturating_sub(1),
                0,
                cols.saturating_sub(1),
            )];
        }

        let mut regions = Vec::new();
        for (row, range) in self.rows.iter_mut().enumerate() {
            if let Some((start_col, end_col)) = range.take() {
                regions.push(DamageRegion::new(row, row, start_col, end_col));
            }
        }
        regions
    }
}

impl Default for DamageTracker {
    fn default() -> Self {
        Self::new(0)
    }
}

#[cfg(test)]
mod tests {
    use super::{DamageRegion, DamageTracker};

    #[test]
    fn damage_mark_tracks_min_and_max_columns() {
        let mut tracker = DamageTracker::new(4);
        tracker.mark(1, 7);
        tracker.mark(1, 2);
        assert_eq!(tracker.take(10), vec![DamageRegion::new(1, 1, 2, 7)]);
    }

    #[test]
    fn damage_mark_row_returns_full_row_region() {
        let mut tracker = DamageTracker::new(2);
        tracker.mark_row(0, 8);
        assert_eq!(tracker.take(8), vec![DamageRegion::new(0, 0, 0, 7)]);
    }

    #[test]
    fn damage_mark_row_ignores_zero_width_rows() {
        let mut tracker = DamageTracker::new(2);
        tracker.mark_row(0, 0);
        assert!(tracker.take(0).is_empty());
    }

    #[test]
    fn damage_mark_range_merges_with_existing_damage() {
        let mut tracker = DamageTracker::new(2);
        tracker.mark(0, 6);
        tracker.mark_range(0, 2, 4);
        assert_eq!(tracker.take(8), vec![DamageRegion::new(0, 0, 2, 6)]);
    }

    #[test]
    fn damage_mark_all_returns_single_full_region() {
        let mut tracker = DamageTracker::new(3);
        tracker.mark_all();
        assert_eq!(tracker.take(6), vec![DamageRegion::new(0, 2, 0, 5)]);
    }

    #[test]
    fn damage_mark_all_ignores_zero_width_grids() {
        let mut tracker = DamageTracker::new(3);
        tracker.mark_all();
        assert!(tracker.take(0).is_empty());
    }
}
