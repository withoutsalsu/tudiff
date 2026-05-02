use std::fs;
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;
use tempfile::TempDir;
use tudiff::app::{App, FilterMode};
use tudiff::compare::DirectoryComparison;

/// Creates a file with specific byte content, creating parent dirs as needed.
pub fn write_file(path: &Path, content: &[u8]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

/// Creates a sparse file of `size` bytes. On Linux this uses no real disk space.
/// On other platforms it allocates zeros. Mark tests using 1GB files with #[ignore].
pub fn create_sparse_file(path: &Path, size: u64) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let mut f = fs::File::create(path).unwrap();
    if size > 0 {
        f.seek(SeekFrom::Start(size - 1)).unwrap();
        f.write_all(&[0u8]).unwrap();
    } else {
        // empty file
    }
}

/// Creates an empty directory.
pub fn create_empty_dir(path: &Path) {
    fs::create_dir_all(path).unwrap();
}

/// A pair of temporary left/right directories for tests.
pub struct FixturePair {
    pub _root: TempDir,
    pub left: std::path::PathBuf,
    pub right: std::path::PathBuf,
}

impl FixturePair {
    pub fn new() -> Self {
        let root = TempDir::new().unwrap();
        let left = root.path().join("left");
        let right = root.path().join("right");
        fs::create_dir_all(&left).unwrap();
        fs::create_dir_all(&right).unwrap();
        Self { _root: root, left, right }
    }

    pub fn compare(&self) -> DirectoryComparison {
        DirectoryComparison::new_silent(self.left.clone(), self.right.clone()).unwrap()
    }

    pub fn app(&self) -> App {
        App::new(self.compare())
    }
}

/// Blocks until `app.is_refreshing` becomes false by repeatedly calling
/// `check_refresh_progress`. Panics after 10 seconds.
pub fn wait_refresh(app: &mut App) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while app.is_refreshing {
        app.check_refresh_progress();
        if std::time::Instant::now() > deadline {
            panic!("wait_refresh timed out after 10s");
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

/// Returns the index of the item whose path matches `path_str` in the panel's item list.
/// `panel == 0` checks left_items, `panel == 1` checks right_items.
pub fn find_item_index(app: &App, path_str: &str, panel: usize) -> Option<usize> {
    let items = if panel == 0 { &app.left_items } else { &app.right_items };
    let target = std::path::PathBuf::from(path_str);
    items.iter().position(|i| i.path == target)
}

/// Selects the item by path in the given panel. Panics if not found.
pub fn select_item(app: &mut App, path_str: &str, panel: usize) {
    let idx = find_item_index(app, path_str, panel)
        .unwrap_or_else(|| panic!("item '{}' not found in panel {}", path_str, panel));
    app.active_panel = panel;
    if panel == 0 {
        app.left_list_state.select(Some(idx));
    } else {
        app.right_list_state.select(Some(idx));
    }
}

/// Sets filter mode and refreshes item lists.
pub fn set_filter(app: &mut App, mode: FilterMode) {
    app.filter_mode = mode;
    app.update_file_lists();
}
