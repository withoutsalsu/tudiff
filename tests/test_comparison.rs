mod common;
use common::{create_empty_dir, create_sparse_file, write_file, FixturePair};
use tudiff::compare::FileStatus;

// ── Both empty ────────────────────────────────────────────────────────────────

#[test]
fn both_empty_dirs_have_no_children() {
    let f = FixturePair::new();
    let cmp = f.compare();
    assert!(cmp.left_tree.children.is_empty());
    assert!(cmp.right_tree.children.is_empty());
}

// ── LeftOnly ──────────────────────────────────────────────────────────────────

#[test]
fn left_only_file_has_leftonly_status() {
    let f = FixturePair::new();
    write_file(&f.left.join("only_left.txt"), b"hello");

    let cmp = f.compare();
    let node = cmp.left_tree.children.iter().find(|n| n.name == "only_left.txt").unwrap();
    assert_eq!(node.status, FileStatus::LeftOnly);
}

#[test]
fn left_only_file_has_placeholder_in_right_tree() {
    let f = FixturePair::new();
    write_file(&f.left.join("only_left.txt"), b"hello");

    let cmp = f.compare();
    // Right tree must have an empty-name placeholder so both panels stay aligned
    let placeholder = cmp.right_tree.children.iter().find(|n| n.path.ends_with("only_left.txt"));
    assert!(placeholder.is_some(), "right tree must contain a placeholder for the left-only file");
}

// ── RightOnly ─────────────────────────────────────────────────────────────────

#[test]
fn right_only_file_has_rightonly_status() {
    let f = FixturePair::new();
    write_file(&f.right.join("only_right.txt"), b"world");

    let cmp = f.compare();
    let node = cmp.right_tree.children.iter().find(|n| n.name == "only_right.txt").unwrap();
    assert_eq!(node.status, FileStatus::RightOnly);
}

// ── Same ──────────────────────────────────────────────────────────────────────

#[test]
fn identical_files_are_same() {
    let f = FixturePair::new();
    write_file(&f.left.join("shared.txt"), b"identical content");
    write_file(&f.right.join("shared.txt"), b"identical content");

    let cmp = f.compare();
    let node = cmp.left_tree.children.iter().find(|n| n.name == "shared.txt").unwrap();
    assert_eq!(node.status, FileStatus::Same);
}

// ── Different ─────────────────────────────────────────────────────────────────

#[test]
fn different_content_files_are_different() {
    let f = FixturePair::new();
    write_file(&f.left.join("file.txt"), b"version A");
    write_file(&f.right.join("file.txt"), b"version B");

    let cmp = f.compare();
    let node = cmp.left_tree.children.iter().find(|n| n.name == "file.txt").unwrap();
    assert_eq!(node.status, FileStatus::Different);
}

// ── Directory status propagation ──────────────────────────────────────────────

#[test]
fn leftonly_dir_propagates_status() {
    let f = FixturePair::new();
    write_file(&f.left.join("mydir").join("a.txt"), b"a");
    write_file(&f.left.join("mydir").join("b.txt"), b"b");

    let cmp = f.compare();
    let dir_node = cmp.left_tree.children.iter().find(|n| n.name == "mydir").unwrap();
    assert_eq!(dir_node.status, FileStatus::LeftOnly);
    for child in &dir_node.children {
        assert_eq!(child.status, FileStatus::LeftOnly, "child {} should be LeftOnly", child.name);
    }
}

#[test]
fn dir_with_leftonly_and_rightonly_children_becomes_different() {
    let f = FixturePair::new();
    write_file(&f.left.join("mixed").join("left.txt"), b"L");
    write_file(&f.right.join("mixed").join("right.txt"), b"R");

    let cmp = f.compare();
    let left_dir = cmp.left_tree.children.iter().find(|n| n.name == "mixed").unwrap();
    assert_eq!(left_dir.status, FileStatus::Different,
        "dir with both LeftOnly and RightOnly children must be Different");
}

#[test]
fn dir_with_only_same_children_is_same() {
    let f = FixturePair::new();
    write_file(&f.left.join("same_dir").join("a.txt"), b"same");
    write_file(&f.right.join("same_dir").join("a.txt"), b"same");

    let cmp = f.compare();
    let dir_node = cmp.left_tree.children.iter().find(|n| n.name == "same_dir").unwrap();
    assert_eq!(dir_node.status, FileStatus::Same);
}

#[test]
fn dir_with_only_leftonly_children_is_leftonly() {
    let f = FixturePair::new();
    write_file(&f.left.join("lo_dir").join("x.txt"), b"x");
    // right has the dir as placeholder but no file children
    create_empty_dir(&f.right.join("lo_dir"));

    let cmp = f.compare();
    let dir_node = cmp.left_tree.children.iter().find(|n| n.name == "lo_dir").unwrap();
    assert_eq!(dir_node.status, FileStatus::LeftOnly);
}

// ── Deep nesting ──────────────────────────────────────────────────────────────

#[test]
fn deep_nesting_10_levels_correct_status() {
    let f = FixturePair::new();
    let deep_path: std::path::PathBuf = (0..10).fold(f.left.clone(), |p, i| p.join(format!("d{}", i)));
    write_file(&deep_path.join("leaf.txt"), b"deep");

    let cmp = f.compare();
    // Root-level dir should be LeftOnly
    let root_dir = cmp.left_tree.children.iter().find(|n| n.name == "d0").unwrap();
    assert_eq!(root_dir.status, FileStatus::LeftOnly);
}

// ── Unicode filenames ─────────────────────────────────────────────────────────

#[test]
fn unicode_korean_filename_left_only() {
    let f = FixturePair::new();
    write_file(&f.left.join("한글파일.txt"), b"korean");

    let cmp = f.compare();
    let node = cmp.left_tree.children.iter().find(|n| n.name == "한글파일.txt").unwrap();
    assert_eq!(node.status, FileStatus::LeftOnly);
}

#[test]
fn unicode_korean_filename_same() {
    let f = FixturePair::new();
    write_file(&f.left.join("동일파일.txt"), b"same korean content");
    write_file(&f.right.join("동일파일.txt"), b"same korean content");

    let cmp = f.compare();
    let node = cmp.left_tree.children.iter().find(|n| n.name == "동일파일.txt").unwrap();
    assert_eq!(node.status, FileStatus::Same);
}

// ── Empty subdirectory ────────────────────────────────────────────────────────

#[test]
fn empty_dir_leftonly_is_leftonly() {
    let f = FixturePair::new();
    create_empty_dir(&f.left.join("empty_dir"));

    let cmp = f.compare();
    let dir_node = cmp.left_tree.children.iter().find(|n| n.name == "empty_dir").unwrap();
    assert_eq!(dir_node.status, FileStatus::LeftOnly);
}

// ── Many files ────────────────────────────────────────────────────────────────

#[test]
fn many_files_100_correct_counts() {
    let f = FixturePair::new();
    // 50 same, 25 left-only, 25 right-only
    for i in 0..50 {
        write_file(&f.left.join(format!("same_{}.txt", i)), b"same");
        write_file(&f.right.join(format!("same_{}.txt", i)), b"same");
    }
    for i in 0..25 {
        write_file(&f.left.join(format!("lo_{}.txt", i)), b"left");
    }
    for i in 0..25 {
        write_file(&f.right.join(format!("ro_{}.txt", i)), b"right");
    }

    let cmp = f.compare();
    let same_count = cmp.left_tree.children.iter().filter(|n| n.status == FileStatus::Same).count();
    let lo_count = cmp.left_tree.children.iter().filter(|n| n.status == FileStatus::LeftOnly).count();
    let ro_count = cmp.right_tree.children.iter().filter(|n| n.status == FileStatus::RightOnly).count();

    assert_eq!(same_count, 50, "expected 50 same files");
    assert_eq!(lo_count, 25, "expected 25 left-only files");
    assert_eq!(ro_count, 25, "expected 25 right-only files");
}

// ── Large sparse files ────────────────────────────────────────────────────────

#[test]
fn medium_sparse_file_same_content() {
    let f = FixturePair::new();
    // 50 MB sparse (all zeros) — exercises the CRC32 path without 1GB overhead
    create_sparse_file(&f.left.join("big.bin"), 50 * 1024 * 1024);
    create_sparse_file(&f.right.join("big.bin"), 50 * 1024 * 1024);

    let cmp = f.compare();
    let node = cmp.left_tree.children.iter().find(|n| n.name == "big.bin").unwrap();
    assert_eq!(node.status, FileStatus::Same, "two 50MB zero-filled sparse files must be Same");
}

#[test]
fn medium_sparse_file_different_content() {
    let f = FixturePair::new();
    // Left: sparse 50MB (all zeros). Right: 50MB with a different byte at the end.
    create_sparse_file(&f.left.join("big.bin"), 50 * 1024 * 1024);
    {
        use std::io::{Seek, SeekFrom, Write};
        let mut file = std::fs::File::create(f.right.join("big.bin")).unwrap();
        file.seek(SeekFrom::Start(50 * 1024 * 1024 - 2)).unwrap();
        file.write_all(&[0xFF]).unwrap(); // differs from zero
        file.seek(SeekFrom::Start(50 * 1024 * 1024 - 1)).unwrap();
        file.write_all(&[0x00]).unwrap();
    }

    let cmp = f.compare();
    let node = cmp.left_tree.children.iter().find(|n| n.name == "big.bin").unwrap();
    assert_eq!(node.status, FileStatus::Different, "50MB files with differing byte must be Different");
}

/// Requires ~2GB of disk reads. Run with: cargo test -- --include-ignored
#[test]
#[ignore]
fn large_1gb_sparse_file_same() {
    let f = FixturePair::new();
    create_sparse_file(&f.left.join("huge.bin"), 1024 * 1024 * 1024);
    create_sparse_file(&f.right.join("huge.bin"), 1024 * 1024 * 1024);

    let cmp = f.compare();
    let node = cmp.left_tree.children.iter().find(|n| n.name == "huge.bin").unwrap();
    assert_eq!(node.status, FileStatus::Same);
}

/// Requires ~2GB of disk reads. Run with: cargo test -- --include-ignored
#[test]
#[ignore]
fn large_1gb_sparse_files_different() {
    let f = FixturePair::new();
    create_sparse_file(&f.left.join("huge.bin"), 1024 * 1024 * 1024);
    {
        use std::io::{Seek, SeekFrom, Write};
        let mut file = std::fs::File::create(f.right.join("huge.bin")).unwrap();
        // differ at the 512MB mark
        file.seek(SeekFrom::Start(512 * 1024 * 1024)).unwrap();
        file.write_all(&[0xAB]).unwrap();
        file.seek(SeekFrom::Start(1024 * 1024 * 1024 - 1)).unwrap();
        file.write_all(&[0x00]).unwrap();
    }

    let cmp = f.compare();
    let node = cmp.left_tree.children.iter().find(|n| n.name == "huge.bin").unwrap();
    assert_eq!(node.status, FileStatus::Different);
}
