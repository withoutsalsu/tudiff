mod common;
use common::{select_item, wait_refresh, write_file, FixturePair, create_empty_dir};
use std::path::PathBuf;
use tudiff::app::AppMode;

// ── File deletion ─────────────────────────────────────────────────────────────

#[test]
fn delete_leftonly_file_removes_from_disk() {
    let f = FixturePair::new();
    write_file(&f.left.join("del.txt"), b"delete me");
    assert!(f.left.join("del.txt").exists());

    let mut app = f.app();
    select_item(&mut app, "del.txt", 0);
    app.prepare_delete();
    app.execute_delete().unwrap();

    assert!(!f.left.join("del.txt").exists(), "file must be gone from disk after delete");
}

#[test]
fn delete_rightonly_file_removes_from_disk() {
    let f = FixturePair::new();
    write_file(&f.right.join("rdel.txt"), b"delete right");

    let mut app = f.app();
    select_item(&mut app, "rdel.txt", 1);
    app.prepare_delete();
    app.execute_delete().unwrap();

    assert!(!f.right.join("rdel.txt").exists(), "right file must be gone after delete");
}

#[test]
fn delete_same_file_from_left_removes_from_disk() {
    let f = FixturePair::new();
    write_file(&f.left.join("both.txt"), b"both sides");
    write_file(&f.right.join("both.txt"), b"both sides");

    let mut app = f.app();
    select_item(&mut app, "both.txt", 0);
    app.prepare_delete();
    app.execute_delete().unwrap();

    assert!(!f.left.join("both.txt").exists(), "left copy must be deleted");
    assert!(f.right.join("both.txt").exists(), "right copy must remain untouched");
}

// ── Directory deletion ────────────────────────────────────────────────────────

#[test]
fn delete_dir_removes_recursively_from_disk() {
    let f = FixturePair::new();
    write_file(&f.left.join("rmdir").join("a.txt"), b"a");
    write_file(&f.left.join("rmdir").join("b.txt"), b"b");
    write_file(&f.left.join("rmdir").join("sub").join("c.txt"), b"c");

    let mut app = f.app();
    app.expand_all();
    select_item(&mut app, "rmdir", 0);
    app.prepare_delete();
    app.execute_delete().unwrap();

    assert!(!f.left.join("rmdir").exists(), "directory must be recursively deleted");
}

#[test]
fn delete_empty_dir_removes_from_disk() {
    let f = FixturePair::new();
    create_empty_dir(&f.left.join("emptydir"));

    let mut app = f.app();
    select_item(&mut app, "emptydir", 0);
    app.prepare_delete();
    app.execute_delete().unwrap();

    assert!(!f.left.join("emptydir").exists(), "empty dir must be deleted");
}

// ── App state after delete ────────────────────────────────────────────────────

#[test]
fn delete_triggers_background_refresh() {
    let f = FixturePair::new();
    write_file(&f.left.join("trigger.txt"), b"trigger");

    let mut app = f.app();
    select_item(&mut app, "trigger.txt", 0);
    app.prepare_delete();
    app.execute_delete().unwrap();

    assert!(app.is_refreshing, "app.is_refreshing must be true immediately after delete");
}

#[test]
fn delete_mode_returns_to_directory_view() {
    let f = FixturePair::new();
    write_file(&f.left.join("mode.txt"), b"mode test");

    let mut app = f.app();
    select_item(&mut app, "mode.txt", 0);
    app.prepare_delete();
    app.execute_delete().unwrap();

    assert_eq!(app.mode, AppMode::DirectoryView, "mode must return to DirectoryView after delete");
}

#[test]
fn delete_completes_refresh_and_file_absent_in_tree() {
    let f = FixturePair::new();
    write_file(&f.left.join("gone.txt"), b"gone");
    // also add another file so the tree isn't empty after delete
    write_file(&f.left.join("stays.txt"), b"stays");
    write_file(&f.right.join("stays.txt"), b"stays");

    let mut app = f.app();
    select_item(&mut app, "gone.txt", 0);
    app.prepare_delete();
    app.execute_delete().unwrap();

    // Wait for background refresh to complete
    wait_refresh(&mut app);

    let found = app.left_items.iter().any(|i| i.path == PathBuf::from("gone.txt"));
    assert!(!found, "deleted file must not appear in left_items after refresh completes");

    let stays = app.left_items.iter().any(|i| i.path == PathBuf::from("stays.txt"));
    assert!(stays, "non-deleted file must still appear after refresh");
}

// ── Repeat-delete edge case ───────────────────────────────────────────────────

#[test]
fn delete_right_only_file_and_refresh_shows_correct_tree() {
    let f = FixturePair::new();
    write_file(&f.right.join("ronly.txt"), b"right only");
    write_file(&f.left.join("keep.txt"), b"keep");
    write_file(&f.right.join("keep.txt"), b"keep");

    let mut app = f.app();
    select_item(&mut app, "ronly.txt", 1);
    app.prepare_delete();
    app.execute_delete().unwrap();
    wait_refresh(&mut app);

    let ronly_in_right = app.right_items.iter().any(|i| i.path == PathBuf::from("ronly.txt"));
    assert!(!ronly_in_right, "ronly.txt must not appear in right_items after delete+refresh");
}
