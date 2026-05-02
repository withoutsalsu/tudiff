/// Advanced edge-case tests:
/// - file comparison size boundaries (0B, 4KB, 1MB)
/// - same-size files with differing interior byte (real CRC32 detection)
/// - zero-byte files
/// - swap_panels
/// - filter persistence after refresh
/// - selection clamp after delete

mod common;
use common::{create_sparse_file, select_item, set_filter, wait_refresh, write_file, FixturePair};
use std::io::{Seek, SeekFrom, Write};
use std::path::PathBuf;
use tudiff::app::FilterMode;
use tudiff::compare::FileStatus;

// ── Zero-byte files ───────────────────────────────────────────────────────────

#[test]
fn zero_byte_files_both_sides_are_same() {
    let f = FixturePair::new();
    write_file(&f.left.join("empty.bin"), b"");
    write_file(&f.right.join("empty.bin"), b"");

    let cmp = f.compare();
    let node = cmp.left_tree.children.iter().find(|n| n.name == "empty.bin").unwrap();
    assert_eq!(node.status, FileStatus::Same, "two zero-byte files must be Same");
}

#[test]
fn zero_byte_vs_nonempty_are_different() {
    let f = FixturePair::new();
    write_file(&f.left.join("empty.bin"), b"");
    write_file(&f.right.join("empty.bin"), b"X");

    let cmp = f.compare();
    let node = cmp.left_tree.children.iter().find(|n| n.name == "empty.bin").unwrap();
    assert_eq!(node.status, FileStatus::Different, "zero-byte vs non-empty must be Different");
}

// ── File comparison size boundaries ──────────────────────────────────────────

fn make_exact_file(path: &std::path::Path, size: usize, fill: u8) {
    let content = vec![fill; size];
    write_file(path, &content);
}

/// Small file boundary: exactly 4095 bytes (just below small→CRC32 boundary)
#[test]
fn file_4095_bytes_same_content() {
    let f = FixturePair::new();
    make_exact_file(&f.left.join("b4095.bin"), 4095, 0xAA);
    make_exact_file(&f.right.join("b4095.bin"), 4095, 0xAA);

    let cmp = f.compare();
    let node = cmp.left_tree.children.iter().find(|n| n.name == "b4095.bin").unwrap();
    assert_eq!(node.status, FileStatus::Same);
}

#[test]
fn file_4095_bytes_different_content() {
    let f = FixturePair::new();
    make_exact_file(&f.left.join("b4095d.bin"), 4095, 0xAA);
    make_exact_file(&f.right.join("b4095d.bin"), 4095, 0xBB);

    let cmp = f.compare();
    let node = cmp.left_tree.children.iter().find(|n| n.name == "b4095d.bin").unwrap();
    assert_eq!(node.status, FileStatus::Different);
}

/// Small→CRC32 boundary: exactly 4096 bytes
#[test]
fn file_4096_bytes_same_content() {
    let f = FixturePair::new();
    make_exact_file(&f.left.join("b4096.bin"), 4096, 0x55);
    make_exact_file(&f.right.join("b4096.bin"), 4096, 0x55);

    let cmp = f.compare();
    let node = cmp.left_tree.children.iter().find(|n| n.name == "b4096.bin").unwrap();
    assert_eq!(node.status, FileStatus::Same);
}

#[test]
fn file_4096_bytes_different_content() {
    let f = FixturePair::new();
    make_exact_file(&f.left.join("b4096d.bin"), 4096, 0x55);
    make_exact_file(&f.right.join("b4096d.bin"), 4096, 0x66);

    let cmp = f.compare();
    let node = cmp.left_tree.children.iter().find(|n| n.name == "b4096d.bin").unwrap();
    assert_eq!(node.status, FileStatus::Different);
}

/// CRC32→large boundary: exactly 1MB - 1
#[test]
fn file_1mb_minus_1_same_content() {
    let f = FixturePair::new();
    make_exact_file(&f.left.join("b1m1.bin"), 1024 * 1024 - 1, 0x33);
    make_exact_file(&f.right.join("b1m1.bin"), 1024 * 1024 - 1, 0x33);

    let cmp = f.compare();
    let node = cmp.left_tree.children.iter().find(|n| n.name == "b1m1.bin").unwrap();
    assert_eq!(node.status, FileStatus::Same);
}

#[test]
fn file_1mb_minus_1_different_content() {
    let f = FixturePair::new();
    make_exact_file(&f.left.join("b1m1d.bin"), 1024 * 1024 - 1, 0x33);
    make_exact_file(&f.right.join("b1m1d.bin"), 1024 * 1024 - 1, 0x44);

    let cmp = f.compare();
    let node = cmp.left_tree.children.iter().find(|n| n.name == "b1m1d.bin").unwrap();
    assert_eq!(node.status, FileStatus::Different);
}

/// Exactly 1MB (hits the large-file CRC32 branch)
#[test]
fn file_exactly_1mb_same_content() {
    let f = FixturePair::new();
    make_exact_file(&f.left.join("b1m.bin"), 1024 * 1024, 0x77);
    make_exact_file(&f.right.join("b1m.bin"), 1024 * 1024, 0x77);

    let cmp = f.compare();
    let node = cmp.left_tree.children.iter().find(|n| n.name == "b1m.bin").unwrap();
    assert_eq!(node.status, FileStatus::Same);
}

#[test]
fn file_exactly_1mb_different_content() {
    let f = FixturePair::new();
    make_exact_file(&f.left.join("b1md.bin"), 1024 * 1024, 0x77);
    make_exact_file(&f.right.join("b1md.bin"), 1024 * 1024, 0x88);

    let cmp = f.compare();
    let node = cmp.left_tree.children.iter().find(|n| n.name == "b1md.bin").unwrap();
    assert_eq!(node.status, FileStatus::Different);
}

// ── CRC32 interior-byte mismatch (same length, one differing byte at midpoint) ──

#[test]
fn crc32_medium_file_interior_byte_mismatch_detected() {
    // Two 512KB files with all-zero content except one byte at offset 256KB
    let f = FixturePair::new();
    let size: u64 = 512 * 1024;

    create_sparse_file(&f.left.join("interior.bin"), size);
    {
        let mut right = std::fs::File::create(f.right.join("interior.bin")).unwrap();
        right.seek(SeekFrom::Start(size / 2)).unwrap();
        right.write_all(&[0xFF]).unwrap();
        right.seek(SeekFrom::Start(size - 1)).unwrap();
        right.write_all(&[0x00]).unwrap();
    }

    let cmp = f.compare();
    let node = cmp.left_tree.children.iter().find(|n| n.name == "interior.bin").unwrap();
    assert_eq!(node.status, FileStatus::Different,
        "CRC32 must detect a single differing byte in the middle of a 512KB file");
}

#[test]
fn crc32_large_file_interior_byte_mismatch_detected() {
    // Two 4MB files with all-zero content except one byte at offset 2MB
    let f = FixturePair::new();
    let size: u64 = 4 * 1024 * 1024;

    create_sparse_file(&f.left.join("big_interior.bin"), size);
    {
        let mut right = std::fs::File::create(f.right.join("big_interior.bin")).unwrap();
        right.seek(SeekFrom::Start(size / 2)).unwrap();
        right.write_all(&[0xAB]).unwrap();
        right.seek(SeekFrom::Start(size - 1)).unwrap();
        right.write_all(&[0x00]).unwrap();
    }

    let cmp = f.compare();
    let node = cmp.left_tree.children.iter().find(|n| n.name == "big_interior.bin").unwrap();
    assert_eq!(node.status, FileStatus::Different,
        "CRC32 must detect a single differing byte in the middle of a 4MB file");
}

// ── swap_panels ───────────────────────────────────────────────────────────────

#[test]
fn swap_panels_exchanges_left_and_right_items() {
    let f = FixturePair::new();
    write_file(&f.left.join("lo.txt"), b"left only");
    write_file(&f.right.join("ro.txt"), b"right only");

    let mut app = f.app();

    // Before swap: lo.txt is LeftOnly in left panel, ro.txt is RightOnly in right panel
    let lo_left = app.left_items.iter().any(|i| i.path == PathBuf::from("lo.txt") && i.status == FileStatus::LeftOnly);
    let ro_right = app.right_items.iter().any(|i| i.path == PathBuf::from("ro.txt") && i.status == FileStatus::RightOnly);
    assert!(lo_left, "lo.txt must be LeftOnly in left panel before swap");
    assert!(ro_right, "ro.txt must be RightOnly in right panel before swap");

    app.swap_panels();

    // After swap: the directories are exchanged, so lo.txt is now RightOnly in right panel
    let lo_right = app.right_items.iter().any(|i| i.path == PathBuf::from("lo.txt"));
    let ro_left = app.left_items.iter().any(|i| i.path == PathBuf::from("ro.txt"));
    assert!(lo_right, "lo.txt must appear in right panel after swap");
    assert!(ro_left, "ro.txt must appear in left panel after swap");
}

#[test]
fn swap_panels_is_reversible() {
    let f = FixturePair::new();
    write_file(&f.left.join("file.txt"), b"content");

    let mut app = f.app();
    let original_left_count = app.left_items.len();

    app.swap_panels();
    app.swap_panels();

    assert_eq!(app.left_items.len(), original_left_count,
        "double swap must restore original left item count");
}

// ── Filter persistence across refresh ────────────────────────────────────────

#[test]
fn filter_mode_persists_across_delete_and_refresh() {
    let f = FixturePair::new();
    write_file(&f.left.join("del.txt"), b"delete");
    write_file(&f.left.join("same.txt"), b"same");
    write_file(&f.right.join("same.txt"), b"same");

    let mut app = f.app();
    set_filter(&mut app, FilterMode::Different);

    select_item(&mut app, "del.txt", 0);
    app.prepare_delete();
    app.execute_delete().unwrap();
    wait_refresh(&mut app);

    assert_eq!(app.filter_mode, FilterMode::Different,
        "filter mode must be restored to Different after delete+refresh");
}

// ── Selection clamping after delete ──────────────────────────────────────────

#[test]
fn delete_last_item_selection_stays_valid() {
    let f = FixturePair::new();
    write_file(&f.left.join("only.txt"), b"only one");

    let mut app = f.app();
    select_item(&mut app, "only.txt", 0);
    app.prepare_delete();
    app.execute_delete().unwrap();
    wait_refresh(&mut app);

    // After deleting the only item, list is empty — selection must not panic
    // and must be either None or 0 (clamped)
    let sel = app.left_list_state.selected();
    assert!(sel.is_none() || sel == Some(0),
        "selection after deleting last item must be None or 0, got {:?}", sel);
}

// ── Symlinks ──────────────────────────────────────────────────────────────────
// tudiff uses WalkDir without follow_links, so symlinks appear as entries
// but their content is not dereferenced. This test documents current behavior.

#[cfg(unix)]
#[test]
fn symlink_to_file_is_detected_as_leftonly() {
    let f = FixturePair::new();
    write_file(&f.left.join("target.txt"), b"target");
    std::os::unix::fs::symlink(f.left.join("target.txt"), f.left.join("link.txt")).unwrap();

    let cmp = f.compare();
    // Both "target.txt" and "link.txt" should appear in the left tree (walkdir visits both)
    let target = cmp.left_tree.children.iter().find(|n| n.name == "target.txt");
    let link = cmp.left_tree.children.iter().find(|n| n.name == "link.txt");
    assert!(target.is_some(), "target.txt must appear in left tree");
    assert!(link.is_some(), "link.txt (symlink) must appear in left tree");
    // Both should be LeftOnly since right has neither
    if let Some(l) = link {
        assert_eq!(l.status, FileStatus::LeftOnly, "symlink must be LeftOnly");
    }
}
