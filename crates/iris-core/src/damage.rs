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
    /// Constructs a DamageRegion from explicit inclusive row and column coordinates.
    ///
    /// # Examples
    ///
    /// ```
    /// let r = DamageRegion::new(2, 4, 0, 7);
    /// assert_eq!(r.start_row, 2);
    /// assert_eq!(r.end_row, 4);
    /// assert_eq!(r.start_col, 0);
    /// assert_eq!(r.end_col, 7);
    /// ```
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

/// Tracks which visible rows and columns changed since the last render pass.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DamageTracker {
    rows: Vec<Option<(usize, usize)>>,
    all_damaged: bool,
}

impl DamageTracker {
    /// Creates a DamageTracker for the given number of visible rows.
    ///
    /// Each row's damage range is initially unset and the tracker is not marked as fully damaged.
    ///
    /// # Examples
    ///
    /// ```
    /// let t = DamageTracker::new(3);
    /// assert!(!t.is_damaged());
    /// ```
    #[must_use]
    pub fn new(rows: usize) -> Self {
        Self {
            rows: vec![None; rows],
            all_damaged: false,
        }
    }

    /// Resize the tracker to the given number of rows and mark the entire grid as damaged.
    ///
    /// This adjusts internal per-row storage to `rows` entries and sets the tracker into
    /// "all damaged" state so subsequent `take` calls will return a full-region covering
    /// the resized grid.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut dt = DamageTracker::new(2);
    /// dt.resize(4);
    /// assert!(dt.is_damaged());
    /// ```
    pub fn resize(&mut self, rows: usize) {
        self.rows.resize(rows, None);
        self.all_damaged = true;
    }

    /// Adds a single cell to the tracked damage for its row, expanding the row's column range as needed.
    ///
    /// If the row index is outside the tracker's current height, the call is ignored. If the row already
    /// has a damaged column range, that range is expanded to include `col`; otherwise a new range of
    /// `(col, col)` is created for that row.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut t = DamageTracker::new(3);
    /// t.mark(1, 2);
    /// t.mark(1, 4);
    /// let regions = t.take(10);
    /// assert_eq!(regions.len(), 1);
    /// let r = &regions[0];
    /// assert_eq!(r.start_row, 1);
    /// assert_eq!(r.end_row, 1);
    /// assert_eq!(r.start_col, 2);
    /// assert_eq!(r.end_col, 4);
    /// ```
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

    /// Marks the specified row as damaged covering the full column range for that row.
    ///
    /// If `cols` is 0 the damaged column range is set to `0..=0`; otherwise it is set to
    /// `0..=(cols - 1)`. No action is taken if `row` is out of bounds for this tracker.
    ///
    /// # Arguments
    ///
    /// * `row` - The row index to mark as damaged.
    /// * `cols` - The number of columns in the row; determines the end column of the damaged range.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut dt = DamageTracker::new(3);
    /// dt.mark_row(1, 5);
    /// let regions = dt.take(5);
    /// assert_eq!(regions.len(), 1);
    /// let r = &regions[0];
    /// assert_eq!(r.start_row, 1);
    /// assert_eq!(r.end_row, 1);
    /// assert_eq!(r.start_col, 0);
    /// assert_eq!(r.end_col, 4);
    /// ```
    pub fn mark_row(&mut self, row: usize, cols: usize) {
        if let Some(slot) = self.rows.get_mut(row) {
            if cols == 0 {
                *slot = Some((0, 0));
            } else {
                *slot = Some((0, cols.saturating_sub(1)));
            }
        }
    }

    /// Marks the entire grid as damaged.
    ///
    /// After calling, the tracker reports damage covering every row and column; per-row ranges are cleared.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut t = DamageTracker::new(3);
    /// t.mark_all();
    /// assert!(t.is_damaged());
    /// let regions = t.take(5);
    /// assert_eq!(regions.len(), 1);
    /// let r = &regions[0];
    /// assert_eq!(r.start_row, 0);
    /// assert_eq!(r.end_row, 2);
    /// assert_eq!(r.start_col, 0);
    /// assert_eq!(r.end_col, 4);
    /// ```
    pub fn mark_all(&mut self) {
        self.all_damaged = true;
        self.rows.fill(None);
    }

    /// Reports whether any damage is currently tracked.
    ///
    /// # Returns
    /// `true` if any damage is present (either the entire grid was marked or at least one row has a damaged column range), `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut t = DamageTracker::new(3);
    /// assert!(!t.is_damaged());
    /// t.mark(1, 2);
    /// assert!(t.is_damaged());
    /// ```
    pub fn is_damaged(&self) -> bool {
        self.all_damaged || self.rows.iter().any(Option::is_some)
    }

    /// Drains pending damage into a vector of damage regions.
    ///
    /// The method consumes (clears) the tracker's recorded damage and produces a `Vec<DamageRegion>`
    /// describing the damaged areas. If the tracker is in "all damaged" state and has at least one row,
    /// a single region covering rows `0..=rows-1` and columns `0..=cols-1` is returned; if there are
    /// no rows, an empty vector is returned. Otherwise, one region per row with recorded column ranges
    /// is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut t = DamageTracker::new(3);
    /// t.mark(1, 2);
    /// t.mark(1, 4);
    /// let regions = t.take(10);
    /// assert_eq!(regions.len(), 1);
    /// assert_eq!(regions[0].start_row, 1);
    /// assert_eq!(regions[0].end_row, 1);
    /// assert_eq!(regions[0].start_col, 2);
    /// assert_eq!(regions[0].end_col, 4);
    /// ```
    pub fn take(&mut self, cols: usize) -> Vec<DamageRegion> {
        if self.all_damaged {
            self.all_damaged = false;
            if self.rows.is_empty() {
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
    /// Constructs a default `DamageTracker` initialized for zero rows.
    ///
    /// # Examples
    ///
    /// ```
    /// let tracker = DamageTracker::default();
    /// assert!(!tracker.is_damaged());
    /// ```
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
    fn damage_mark_all_returns_single_full_region() {
        let mut tracker = DamageTracker::new(3);
        tracker.mark_all();
        assert_eq!(tracker.take(6), vec![DamageRegion::new(0, 2, 0, 5)]);
    }
}
