mod common;
use common::{set_filter, write_file, FixturePair};
use tudiff::app::FilterMode;
use tudiff::compare::FileStatus;

/// Returns (left item count, right item count) after applying the given filter.
fn item_counts(f: &FixturePair, mode: FilterMode) -> (usize, usize) {
    let mut app = f.app();
    set_filter(&mut app, mode);
    (app.left_items.len(), app.right_items.len())
}

fn make_mixed_fixture() -> FixturePair {
    let f = FixturePair::new();
    write_file(&f.left.join("same.txt"), b"same");
    write_file(&f.right.join("same.txt"), b"same");
    write_file(&f.left.join("diff.txt"), b"version A");
    write_file(&f.right.join("diff.txt"), b"version B");
    write_file(&f.left.join("lo.txt"), b"left only");
    write_file(&f.right.join("ro.txt"), b"right only");
    f
}

// ── FilterMode::All ───────────────────────────────────────────────────────────

#[test]
fn filter_all_shows_same_items() {
    let f = make_mixed_fixture();
    let mut app = f.app();
    set_filter(&mut app, FilterMode::All);
    let has_same = app.left_items.iter().any(|i| i.status == FileStatus::Same && i.path.ends_with("same.txt"));
    assert!(has_same, "FilterMode::All must include Same items");
}

#[test]
fn filter_all_shows_different_items() {
    let f = make_mixed_fixture();
    let mut app = f.app();
    set_filter(&mut app, FilterMode::All);
    let has_diff = app.left_items.iter().any(|i| i.status == FileStatus::Different);
    assert!(has_diff, "FilterMode::All must include Different items");
}

#[test]
fn filter_all_shows_leftonly_items() {
    let f = make_mixed_fixture();
    let mut app = f.app();
    set_filter(&mut app, FilterMode::All);
    let has_lo = app.left_items.iter().any(|i| i.status == FileStatus::LeftOnly && i.path.ends_with("lo.txt"));
    assert!(has_lo, "FilterMode::All must include LeftOnly items");
}

#[test]
fn filter_all_shows_rightonly_items() {
    let f = make_mixed_fixture();
    let mut app = f.app();
    set_filter(&mut app, FilterMode::All);
    let has_ro = app.right_items.iter().any(|i| i.status == FileStatus::RightOnly && i.path.ends_with("ro.txt"));
    assert!(has_ro, "FilterMode::All must include RightOnly items");
}

// ── FilterMode::Different ─────────────────────────────────────────────────────

#[test]
fn filter_different_hides_same_items() {
    let f = make_mixed_fixture();
    let mut app = f.app();
    set_filter(&mut app, FilterMode::Different);
    let has_same = app.left_items.iter().any(|i| i.status == FileStatus::Same && i.path.ends_with("same.txt"));
    assert!(!has_same, "FilterMode::Different must exclude Same items");
}

#[test]
fn filter_different_shows_different_items() {
    let f = make_mixed_fixture();
    let mut app = f.app();
    set_filter(&mut app, FilterMode::Different);
    let has_diff = app.left_items.iter().any(|i| i.status == FileStatus::Different);
    assert!(has_diff, "FilterMode::Different must show Different items");
}

#[test]
fn filter_different_shows_leftonly_items() {
    let f = make_mixed_fixture();
    let mut app = f.app();
    set_filter(&mut app, FilterMode::Different);
    let has_lo = app.left_items.iter().any(|i| i.status == FileStatus::LeftOnly && i.path.ends_with("lo.txt"));
    assert!(has_lo, "FilterMode::Different must show LeftOnly items");
}

#[test]
fn filter_different_shows_rightonly_items() {
    let f = make_mixed_fixture();
    let mut app = f.app();
    set_filter(&mut app, FilterMode::Different);
    let has_ro = app.right_items.iter().any(|i| i.status == FileStatus::RightOnly && i.path.ends_with("ro.txt"));
    assert!(has_ro, "FilterMode::Different must show RightOnly items");
}

// ── FilterMode::DifferentNotOrphans ──────────────────────────────────────────

#[test]
fn filter_diffonly_shows_only_different_items() {
    let f = make_mixed_fixture();
    let mut app = f.app();
    set_filter(&mut app, FilterMode::DifferentNotOrphans);
    let has_diff = app.left_items.iter().any(|i| i.status == FileStatus::Different);
    assert!(has_diff, "FilterMode::DifferentNotOrphans must show Different items");
}

#[test]
fn filter_diffonly_hides_same_items() {
    let f = make_mixed_fixture();
    let mut app = f.app();
    set_filter(&mut app, FilterMode::DifferentNotOrphans);
    let has_same = app.left_items.iter().any(|i| i.status == FileStatus::Same && i.path.ends_with("same.txt"));
    assert!(!has_same, "FilterMode::DifferentNotOrphans must exclude Same items");
}

#[test]
fn filter_diffonly_hides_leftonly_items() {
    let f = make_mixed_fixture();
    let mut app = f.app();
    set_filter(&mut app, FilterMode::DifferentNotOrphans);
    let has_lo = app.left_items.iter().any(|i| i.status == FileStatus::LeftOnly && i.path.ends_with("lo.txt"));
    assert!(!has_lo, "FilterMode::DifferentNotOrphans must exclude LeftOnly items");
}

#[test]
fn filter_diffonly_hides_rightonly_items() {
    let f = make_mixed_fixture();
    let mut app = f.app();
    set_filter(&mut app, FilterMode::DifferentNotOrphans);
    let has_ro = app.right_items.iter().any(|i| i.status == FileStatus::RightOnly && i.path.ends_with("ro.txt"));
    assert!(!has_ro, "FilterMode::DifferentNotOrphans must exclude RightOnly items");
}

// ── Filter totals ─────────────────────────────────────────────────────────────

#[test]
fn filter_all_has_more_items_than_different() {
    let f = make_mixed_fixture();
    let (all_l, _) = item_counts(&f, FilterMode::All);
    let (diff_l, _) = item_counts(&f, FilterMode::Different);
    assert!(all_l > diff_l, "FilterMode::All should show more items than FilterMode::Different");
}

#[test]
fn filter_different_has_more_items_than_diffonly() {
    let f = make_mixed_fixture();
    let (diff_l, _) = item_counts(&f, FilterMode::Different);
    let (diffonly_l, _) = item_counts(&f, FilterMode::DifferentNotOrphans);
    assert!(diff_l > diffonly_l, "FilterMode::Different should show more than DifferentNotOrphans");
}

// ── Nested items ──────────────────────────────────────────────────────────────

#[test]
fn filter_nested_same_hidden_in_different_mode() {
    let f = FixturePair::new();
    // A nested file that is Same, inside a dir that has a different file too
    write_file(&f.left.join("subdir").join("same.txt"), b"same");
    write_file(&f.right.join("subdir").join("same.txt"), b"same");
    write_file(&f.left.join("subdir").join("diff.txt"), b"A");
    write_file(&f.right.join("subdir").join("diff.txt"), b"B");

    let mut app = f.app();
    // Expand the subdir so nested items are visible
    app.expand_all();
    set_filter(&mut app, FilterMode::Different);

    let has_nested_same = app.left_items.iter()
        .any(|i| i.status == FileStatus::Same && i.path.ends_with("same.txt"));
    assert!(!has_nested_same, "nested Same items must be hidden in FilterMode::Different");

    let has_nested_diff = app.left_items.iter()
        .any(|i| i.status == FileStatus::Different && i.path.ends_with("diff.txt"));
    assert!(has_nested_diff, "nested Different items must be visible in FilterMode::Different");
}
