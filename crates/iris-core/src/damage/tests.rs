
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
