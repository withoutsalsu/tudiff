mod common;
use common::{select_item, write_file, FixturePair};
use std::path::PathBuf;
use tudiff::compare::FileStatus;

// ── File copy: left → right ───────────────────────────────────────────────────

#[test]
fn copy_leftonly_file_l2r_status_becomes_same() {
    let f = FixturePair::new();
    write_file(&f.left.join("lo.txt"), b"hello");

    let mut app = f.app();
    select_item(&mut app, "lo.txt", 0);
    app.prepare_copy();
    app.execute_copy().unwrap();

    let left_node = app.left_items.iter().find(|i| i.path == PathBuf::from("lo.txt")).unwrap();
    assert_eq!(left_node.status, FileStatus::Same, "source node must be Same after copy");

    let right_node = app.right_items.iter().find(|i| i.path == PathBuf::from("lo.txt")).unwrap();
    assert_eq!(right_node.status, FileStatus::Same, "target node must be Same after copy");
}

#[test]
fn copy_leftonly_file_l2r_target_name_updated() {
    let f = FixturePair::new();
    write_file(&f.left.join("named.txt"), b"data");

    let mut app = f.app();
    select_item(&mut app, "named.txt", 0);
    app.prepare_copy();
    app.execute_copy().unwrap();

    let right_node = app.right_items.iter().find(|i| i.path == PathBuf::from("named.txt")).unwrap();
    assert!(!right_node.display_name.trim().is_empty(), "right node display_name must not be empty after copy");
}

#[test]
fn copy_leftonly_file_l2r_file_exists_on_disk() {
    let f = FixturePair::new();
    write_file(&f.left.join("disk.txt"), b"disk content");

    let mut app = f.app();
    select_item(&mut app, "disk.txt", 0);
    app.prepare_copy();
    app.execute_copy().unwrap();

    assert!(f.right.join("disk.txt").exists(), "file must exist in right dir after copy");
    let content = std::fs::read(f.right.join("disk.txt")).unwrap();
    assert_eq!(content, b"disk content", "copied file content must match source");
}

// ── File copy: right → left ───────────────────────────────────────────────────

#[test]
fn copy_rightonly_file_r2l_status_becomes_same() {
    let f = FixturePair::new();
    write_file(&f.right.join("ro.txt"), b"from right");

    let mut app = f.app();
    select_item(&mut app, "ro.txt", 1);
    app.prepare_copy();
    app.execute_copy().unwrap();

    let right_node = app.right_items.iter().find(|i| i.path == PathBuf::from("ro.txt")).unwrap();
    assert_eq!(right_node.status, FileStatus::Same);

    let left_node = app.left_items.iter().find(|i| i.path == PathBuf::from("ro.txt")).unwrap();
    assert_eq!(left_node.status, FileStatus::Same);
}

#[test]
fn copy_rightonly_file_r2l_file_exists_on_disk() {
    let f = FixturePair::new();
    write_file(&f.right.join("r2l.txt"), b"r2l data");

    let mut app = f.app();
    select_item(&mut app, "r2l.txt", 1);
    app.prepare_copy();
    app.execute_copy().unwrap();

    assert!(f.left.join("r2l.txt").exists(), "file must exist in left dir after r2l copy");
}

// ── Different file overwrite ──────────────────────────────────────────────────

#[test]
fn copy_different_file_l2r_becomes_same() {
    let f = FixturePair::new();
    write_file(&f.left.join("ver.txt"), b"version A");
    write_file(&f.right.join("ver.txt"), b"version B");

    let mut app = f.app();
    select_item(&mut app, "ver.txt", 0);
    app.prepare_copy();
    app.execute_copy().unwrap();

    let left_node = app.left_items.iter().find(|i| i.path == PathBuf::from("ver.txt")).unwrap();
    assert_eq!(left_node.status, FileStatus::Same);

    let content = std::fs::read(f.right.join("ver.txt")).unwrap();
    assert_eq!(content, b"version A", "overwritten file must contain source content");
}

// ── Directory copy: left → right ─────────────────────────────────────────────

#[test]
fn copy_leftonly_dir_l2r_dir_status_becomes_same() {
    let f = FixturePair::new();
    write_file(&f.left.join("mydir").join("a.txt"), b"a");
    write_file(&f.left.join("mydir").join("b.txt"), b"b");

    let mut app = f.app();
    app.expand_all();
    select_item(&mut app, "mydir", 0);
    app.prepare_copy();
    app.execute_copy().unwrap();

    let dir_node = app.left_items.iter().find(|i| i.path == PathBuf::from("mydir")).unwrap();
    assert_eq!(dir_node.status, FileStatus::Same, "source dir must be Same after copy");

    let right_dir = app.right_items.iter().find(|i| i.path == PathBuf::from("mydir")).unwrap();
    assert_eq!(right_dir.status, FileStatus::Same, "target dir must be Same after copy");
}

#[test]
fn copy_leftonly_dir_l2r_children_become_same() {
    let f = FixturePair::new();
    write_file(&f.left.join("mydir").join("child1.txt"), b"c1");
    write_file(&f.left.join("mydir").join("child2.txt"), b"c2");

    let mut app = f.app();
    app.expand_all();
    select_item(&mut app, "mydir", 0);
    app.prepare_copy();
    app.execute_copy().unwrap();

    let child1 = app.right_items.iter().find(|i| i.path == PathBuf::from("mydir/child1.txt"));
    let child2 = app.right_items.iter().find(|i| i.path == PathBuf::from("mydir/child2.txt"));

    assert!(child1.is_some(), "child1.txt must appear in right panel after dir copy");
    assert!(child2.is_some(), "child2.txt must appear in right panel after dir copy");

    if let Some(c) = child1 {
        assert_eq!(c.status, FileStatus::Same, "child1 must be Same");
    }
    if let Some(c) = child2 {
        assert_eq!(c.status, FileStatus::Same, "child2 must be Same");
    }
}

#[test]
fn copy_leftonly_dir_l2r_target_folder_is_expanded() {
    let f = FixturePair::new();
    write_file(&f.left.join("expdir").join("x.txt"), b"x");

    let mut app = f.app();
    app.expand_all();
    select_item(&mut app, "expdir", 0);
    app.prepare_copy();
    app.execute_copy().unwrap();

    // If the target folder is expanded, its children appear in right_items
    let has_child = app.right_items.iter().any(|i| i.path == PathBuf::from("expdir/x.txt"));
    assert!(has_child, "target folder must be expanded after copy so children are visible");
}

#[test]
fn copy_leftonly_dir_creates_all_files_on_disk() {
    let f = FixturePair::new();
    write_file(&f.left.join("dir_copy").join("f1.txt"), b"file1");
    write_file(&f.left.join("dir_copy").join("f2.txt"), b"file2");
    write_file(&f.left.join("dir_copy").join("sub").join("f3.txt"), b"file3");

    let mut app = f.app();
    app.expand_all();
    select_item(&mut app, "dir_copy", 0);
    app.prepare_copy();
    app.execute_copy().unwrap();

    assert!(f.right.join("dir_copy").join("f1.txt").exists(), "f1.txt must exist");
    assert!(f.right.join("dir_copy").join("f2.txt").exists(), "f2.txt must exist");
    assert!(f.right.join("dir_copy").join("sub").join("f3.txt").exists(), "f3.txt in subdir must exist");
}

// ── Parent status update ──────────────────────────────────────────────────────

#[test]
fn copy_updates_parent_dir_status() {
    let f = FixturePair::new();
    // parent dir has one LeftOnly child and one Same child
    write_file(&f.left.join("parent").join("same.txt"), b"same");
    write_file(&f.right.join("parent").join("same.txt"), b"same");
    write_file(&f.left.join("parent").join("lo.txt"), b"left");

    let mut app = f.app();
    app.expand_all();
    select_item(&mut app, "parent/lo.txt", 0);
    app.prepare_copy();
    app.execute_copy().unwrap();

    // After copying lo.txt, parent should now be Same
    let parent = app.left_items.iter().find(|i| i.path == PathBuf::from("parent")).unwrap();
    assert_eq!(parent.status, FileStatus::Same, "parent status must update to Same after all children become Same");
}

// ── Nested directory copy ─────────────────────────────────────────────────────

#[test]
fn copy_nested_dir_3_levels_all_become_same() {
    let f = FixturePair::new();
    write_file(&f.left.join("l1").join("l2").join("l3").join("deep.txt"), b"deep");
    write_file(&f.left.join("l1").join("top.txt"), b"top");

    let mut app = f.app();
    app.expand_all();
    select_item(&mut app, "l1", 0);
    app.prepare_copy();
    app.execute_copy().unwrap();

    // Root dir must be Same
    let l1 = app.left_items.iter().find(|i| i.path == PathBuf::from("l1")).unwrap();
    assert_eq!(l1.status, FileStatus::Same);

    // Files must exist on disk
    assert!(f.right.join("l1").join("top.txt").exists());
    assert!(f.right.join("l1").join("l2").join("l3").join("deep.txt").exists());
}

// ── Consecutive copies (restore_saved_state_safe re-entry) ────────────────────

#[test]
fn two_consecutive_copies_in_same_session_are_both_correct() {
    let f = FixturePair::new();
    write_file(&f.left.join("first.txt"), b"first");
    write_file(&f.left.join("second.txt"), b"second");

    let mut app = f.app();

    // First copy
    select_item(&mut app, "first.txt", 0);
    app.prepare_copy();
    app.execute_copy().unwrap();

    let first = app.right_items.iter().find(|i| i.path == PathBuf::from("first.txt")).unwrap();
    assert_eq!(first.status, FileStatus::Same, "first.txt must be Same after first copy");

    // Second copy in same session — exercises saved_expansion_state re-entry
    select_item(&mut app, "second.txt", 0);
    app.prepare_copy();
    app.execute_copy().unwrap();

    let second_r = app.right_items.iter().find(|i| i.path == PathBuf::from("second.txt")).unwrap();
    assert_eq!(second_r.status, FileStatus::Same, "second.txt must be Same after second copy");
    // First copy result must still be intact
    let first_r = app.right_items.iter().find(|i| i.path == PathBuf::from("first.txt")).unwrap();
    assert_eq!(first_r.status, FileStatus::Same, "first.txt must still be Same after second copy");
    assert!(f.right.join("second.txt").exists());
    assert!(f.right.join("first.txt").exists());
}

// ── Parent-status update at depth ≥ 3 ────────────────────────────────────────

#[test]
fn copy_file_in_deep_dir_updates_all_ancestors() {
    let f = FixturePair::new();
    // Structure: root/a/b/c/file.txt  ← LeftOnly
    //            root/a/b/c/other.txt  ← Different (keeps ancestor Different after copy)
    write_file(&f.left.join("a").join("b").join("c").join("file.txt"), b"file");
    write_file(&f.left.join("a").join("b").join("c").join("other.txt"), b"L");
    write_file(&f.right.join("a").join("b").join("c").join("other.txt"), b"R");

    let mut app = f.app();
    app.expand_all();

    // Copy the LeftOnly file
    select_item(&mut app, "a/b/c/file.txt", 0);
    app.prepare_copy();
    app.execute_copy().unwrap();

    // file.txt itself must be Same
    let file_node = app.left_items.iter().find(|i| i.path == PathBuf::from("a/b/c/file.txt")).unwrap();
    assert_eq!(file_node.status, FileStatus::Same);

    // ancestor 'c' still has other.txt which is Different → stays Different
    let c_node = app.left_items.iter().find(|i| i.path == PathBuf::from("a/b/c")).unwrap();
    assert_eq!(c_node.status, FileStatus::Different, "'c' must stay Different because other.txt is still Different");

    // ancestors b and a must also be Different
    let b_node = app.left_items.iter().find(|i| i.path == PathBuf::from("a/b")).unwrap();
    assert_eq!(b_node.status, FileStatus::Different, "'b' must be Different");
    let a_node = app.left_items.iter().find(|i| i.path == PathBuf::from("a")).unwrap();
    assert_eq!(a_node.status, FileStatus::Different, "'a' must be Different");
}

#[test]
fn copy_last_differing_file_makes_all_ancestors_same() {
    let f = FixturePair::new();
    write_file(&f.left.join("x").join("y").join("only.txt"), b"only");

    let mut app = f.app();
    app.expand_all();

    select_item(&mut app, "x/y/only.txt", 0);
    app.prepare_copy();
    app.execute_copy().unwrap();

    let only = app.left_items.iter().find(|i| i.path == PathBuf::from("x/y/only.txt")).unwrap();
    assert_eq!(only.status, FileStatus::Same);

    let y = app.left_items.iter().find(|i| i.path == PathBuf::from("x/y")).unwrap();
    assert_eq!(y.status, FileStatus::Same, "'y' must be Same after sole differing file is copied");
    let x = app.left_items.iter().find(|i| i.path == PathBuf::from("x")).unwrap();
    assert_eq!(x.status, FileStatus::Same, "'x' must be Same");
}

// ── Copy error path ───────────────────────────────────────────────────────────

#[test]
fn copy_with_missing_source_returns_error_and_leaves_tree_unchanged() {
    let f = FixturePair::new();
    write_file(&f.left.join("vanish.txt"), b"will vanish");

    let mut app = f.app();
    select_item(&mut app, "vanish.txt", 0);
    app.prepare_copy();

    // Delete the source file from disk before the copy executes
    std::fs::remove_file(f.left.join("vanish.txt")).unwrap();

    // execute_copy must return an error
    let result = app.execute_copy();
    assert!(result.is_err(), "copy with missing source must return Err");

    // The right side must not have acquired a half-written file
    assert!(!f.right.join("vanish.txt").exists(),
        "no partial file must appear on right when copy fails");
}
