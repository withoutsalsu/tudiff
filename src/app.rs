use anyhow::Result;
use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind};
use ratatui::{
    layout::Rect,
    widgets::{ListState, ScrollbarState},
};
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::SystemTime;

use crate::compare::{DirectoryComparison, FileNode, FileStatus};
use crate::utils::{log_error, log_info};

#[derive(PartialEq)]
pub enum AppMode {
    DirectoryView,
    #[allow(dead_code)]
    FileView,
    CopyConfirm,
}

#[derive(PartialEq, Clone, Copy)]
pub enum FilterMode {
    All,
    Different,
    DifferentNotOrphans,
}

enum RefreshMessage {
    Progress(String, f64),
    Complete(DirectoryComparison),
    Error(String),
}

#[derive(Clone)]
pub struct CopyInfo {
    pub source_path: PathBuf,
    pub target_path: PathBuf,
    pub file_count: usize,
    pub folder_count: usize,
    pub total_bytes: u64,
    pub from_left_to_right: bool,
}

pub struct App {
    pub comparison: DirectoryComparison,
    pub mode: AppMode,
    pub active_panel: usize,
    pub left_list_state: ListState,
    pub right_list_state: ListState,
    pub left_items: Vec<(
        String,
        FileStatus,
        PathBuf,
        bool,
        Option<u64>,
        Option<SystemTime>,
    )>,
    pub right_items: Vec<(
        String,
        FileStatus,
        PathBuf,
        bool,
        Option<u64>,
        Option<SystemTime>,
    )>,
    pub file_diff: String,
    pub filter_mode: FilterMode,
    pub is_refreshing: bool,
    pub refresh_progress: String,
    pub refresh_percentage: f64,
    refresh_rx: Option<mpsc::Receiver<RefreshMessage>>,
    pub left_scrollbar_state: ScrollbarState,
    pub right_scrollbar_state: ScrollbarState,
    pub viewport_height: u16,
    pub toolbar_area: Rect,
    pub copy_info: Option<CopyInfo>,
    saved_left_selection: Option<usize>,
    saved_right_selection: Option<usize>,
    saved_active_panel: usize,
    saved_expansion_state: Option<(FileNode, FileNode)>,
    saved_filter_mode: Option<FilterMode>,
}

impl App {
    pub fn new(comparison: DirectoryComparison) -> Self {
        let mut app = Self {
            comparison,
            mode: AppMode::DirectoryView,
            active_panel: 0,
            left_list_state: ListState::default(),
            right_list_state: ListState::default(),
            left_items: Vec::new(),
            right_items: Vec::new(),
            file_diff: String::new(),
            filter_mode: FilterMode::All,
            is_refreshing: false,
            refresh_progress: String::new(),
            refresh_percentage: 0.0,
            refresh_rx: None,
            left_scrollbar_state: ScrollbarState::default(),
            right_scrollbar_state: ScrollbarState::default(),
            viewport_height: 24,
            toolbar_area: Rect::default(),
            copy_info: None,
            saved_left_selection: None,
            saved_right_selection: None,
            saved_active_panel: 0,
            saved_expansion_state: None,
            saved_filter_mode: None,
        };

        app.update_file_lists();
        app.left_list_state.select(Some(0));
        app
    }

    pub fn update_file_lists(&mut self) {
        self.left_items =
            Self::flatten_tree_with_filter(&self.comparison.left_tree, 0, self.filter_mode);
        self.right_items =
            Self::flatten_tree_with_filter(&self.comparison.right_tree, 0, self.filter_mode);

        self.left_scrollbar_state = self
            .left_scrollbar_state
            .content_length(self.left_items.len());
        self.right_scrollbar_state = self
            .right_scrollbar_state
            .content_length(self.right_items.len());
    }

    fn flatten_tree_with_filter(
        node: &FileNode,
        depth: usize,
        filter: FilterMode,
    ) -> Vec<(
        String,
        FileStatus,
        PathBuf,
        bool,
        Option<u64>,
        Option<SystemTime>,
    )> {
        let mut items = Vec::new();

        if depth == 0 {
            if node.is_dir && node.expanded {
                for child in &node.children {
                    items.extend(Self::flatten_tree_with_filter(child, 1, filter));
                }
            }
            return items;
        }

        let indent = "  ".repeat(depth - 1);

        let icon = if node.name.is_empty() {
            ""
        } else if node.is_dir {
            if node.expanded {
                "📂"
            } else {
                "📁"
            }
        } else {
            "📄"
        };

        let display_name = if node.name.is_empty() {
            format!("{}", indent)
        } else if icon.is_empty() {
            format!("{}{}", indent, node.name)
        } else {
            format!("{}{} {}", indent, icon, node.name)
        };

        let should_include = match filter {
            FilterMode::All => true,
            FilterMode::Different => {
                matches!(
                    node.status,
                    FileStatus::Different | FileStatus::LeftOnly | FileStatus::RightOnly
                )
            }
            FilterMode::DifferentNotOrphans => {
                matches!(node.status, FileStatus::Different)
            }
        };

        if should_include {
            items.push((
                display_name,
                node.status,
                node.path.clone(),
                node.is_dir,
                node.size,
                node.modified,
            ));
        }

        if node.is_dir && node.expanded {
            for child in &node.children {
                items.extend(Self::flatten_tree_with_filter(child, depth + 1, filter));
            }
        }

        items
    }

    pub fn handle_mouse_click(&mut self, x: u16, y: u16) {
        if y >= self.toolbar_area.y
            && y <= self.toolbar_area.y + self.toolbar_area.height
            && x >= self.toolbar_area.x
            && x < self.toolbar_area.x + self.toolbar_area.width
        {
            let relative_x = x - self.toolbar_area.x;

            if relative_x <= 16 {
                self.filter_mode = FilterMode::All;
                self.update_file_lists();
            } else if relative_x <= 34 {
                self.filter_mode = FilterMode::Different;
                self.update_file_lists();
            } else if relative_x <= 52 {
                self.filter_mode = FilterMode::DifferentNotOrphans;
                self.update_file_lists();
            } else if relative_x <= 71 {
                self.expand_all();
            } else if relative_x <= 92 {
                self.collapse_all();
            } else if relative_x <= 109 {
                self.start_refresh();
            } else if relative_x <= 129 {
                self.swap_panels();
            } else if relative_x <= 143 {
                if self.can_copy() {
                    self.prepare_copy();
                }
            }
        }
    }

    pub fn calculate_half_page(&self) -> i32 {
        let available_height = self.viewport_height.saturating_sub(5);
        std::cmp::max(1, (available_height / 2) as i32)
    }

    pub fn get_selected_item(
        &self,
    ) -> Option<&(
        String,
        FileStatus,
        PathBuf,
        bool,
        Option<u64>,
        Option<SystemTime>,
    )> {
        let items = if self.active_panel == 0 {
            &self.left_items
        } else {
            &self.right_items
        };
        let state = if self.active_panel == 0 {
            &self.left_list_state
        } else {
            &self.right_list_state
        };

        if let Some(selected) = state.selected() {
            if selected < items.len() {
                return Some(&items[selected]);
            }
        }
        None
    }

    pub fn can_copy(&self) -> bool {
        if let Some((name, status, _path, _is_dir, _size, _modified)) = self.get_selected_item() {
            if name.is_empty() {
                return false;
            }

            match status {
                FileStatus::LeftOnly => self.active_panel == 0,
                FileStatus::RightOnly => self.active_panel == 1,
                FileStatus::Different | FileStatus::Same => true,
            }
        } else {
            false
        }
    }

    fn find_node_by_path(&mut self, path: &PathBuf, is_left: bool) -> Option<&mut FileNode> {
        let tree = if is_left {
            &mut self.comparison.left_tree
        } else {
            &mut self.comparison.right_tree
        };
        Self::find_node_in_tree(tree, path)
    }

    fn find_node_in_tree<'a>(
        node: &'a mut FileNode,
        target_path: &PathBuf,
    ) -> Option<&'a mut FileNode> {
        let exact_path_match = &node.path == target_path;

        if exact_path_match {
            return Some(node);
        }

        for child in &mut node.children {
            if let Some(found) = Self::find_node_in_tree(child, target_path) {
                return Some(found);
            }
        }

        None
    }

    pub fn toggle_folder(&mut self) {
        if let Some((_, _, path, is_dir, _, _)) = self.get_selected_item() {
            if *is_dir {
                let path = path.clone();
                let current_selected = if self.active_panel == 0 {
                    self.left_list_state.selected()
                } else {
                    self.right_list_state.selected()
                };

                if let Some(_selected_index) = current_selected {
                    let new_expanded_state =
                        if let Some(node) = self.find_node_by_path(&path, self.active_panel == 0) {
                            node.toggle_expanded();
                            node.expanded
                        } else {
                            return;
                        };

                    if let Some(opposite_node) =
                        self.find_node_by_path(&path, self.active_panel != 0)
                    {
                        opposite_node.expanded = new_expanded_state;
                    }

                    self.update_file_lists();
                }
            }
        }
    }

    pub fn move_selection(&mut self, delta: i32) {
        let current_state = if self.active_panel == 0 {
            &mut self.left_list_state
        } else {
            &mut self.right_list_state
        };
        let current_items = if self.active_panel == 0 {
            &self.left_items
        } else {
            &self.right_items
        };

        if current_items.is_empty() {
            return;
        }

        let current_selected = current_state.selected().unwrap_or(0);
        let new_selected = if delta > 0 {
            std::cmp::min(current_selected + delta as usize, current_items.len() - 1)
        } else {
            current_selected.saturating_sub((-delta) as usize)
        };

        current_state.select(Some(new_selected));

        if self.active_panel == 0 {
            self.left_scrollbar_state = self.left_scrollbar_state.position(new_selected);
        } else {
            self.right_scrollbar_state = self.right_scrollbar_state.position(new_selected);
        }

        let opposite_state = if self.active_panel == 0 {
            &mut self.right_list_state
        } else {
            &mut self.left_list_state
        };
        let opposite_items = if self.active_panel == 0 {
            &self.right_items
        } else {
            &self.left_items
        };

        if !opposite_items.is_empty() {
            let sync_selected = std::cmp::min(new_selected, opposite_items.len() - 1);
            opposite_state.select(Some(sync_selected));

            if self.active_panel == 0 {
                self.right_scrollbar_state = self.right_scrollbar_state.position(sync_selected);
            } else {
                self.left_scrollbar_state = self.left_scrollbar_state.position(sync_selected);
            }
        }
    }

    pub fn scroll_to_top(&mut self) {
        let current_state = if self.active_panel == 0 {
            &mut self.left_list_state
        } else {
            &mut self.right_list_state
        };
        let current_items = if self.active_panel == 0 {
            &self.left_items
        } else {
            &self.right_items
        };

        if !current_items.is_empty() {
            current_state.select(Some(0));
            if self.active_panel == 0 {
                self.left_scrollbar_state = self.left_scrollbar_state.position(0);
            } else {
                self.right_scrollbar_state = self.right_scrollbar_state.position(0);
            }

            // Sync opposite panel to top as well
            let opposite_state = if self.active_panel == 0 {
                &mut self.right_list_state
            } else {
                &mut self.left_list_state
            };
            let opposite_items = if self.active_panel == 0 {
                &self.right_items
            } else {
                &self.left_items
            };

            if !opposite_items.is_empty() {
                opposite_state.select(Some(0));
                if self.active_panel == 0 {
                    self.right_scrollbar_state = self.right_scrollbar_state.position(0);
                } else {
                    self.left_scrollbar_state = self.left_scrollbar_state.position(0);
                }
            }
        }
    }

    pub fn scroll_to_bottom(&mut self) {
        let current_state = if self.active_panel == 0 {
            &mut self.left_list_state
        } else {
            &mut self.right_list_state
        };
        let current_items = if self.active_panel == 0 {
            &self.left_items
        } else {
            &self.right_items
        };

        if !current_items.is_empty() {
            let last_index = current_items.len() - 1;
            current_state.select(Some(last_index));
            if self.active_panel == 0 {
                self.left_scrollbar_state = self.left_scrollbar_state.position(last_index);
            } else {
                self.right_scrollbar_state = self.right_scrollbar_state.position(last_index);
            }

            // Sync opposite panel to bottom as well
            let opposite_state = if self.active_panel == 0 {
                &mut self.right_list_state
            } else {
                &mut self.left_list_state
            };
            let opposite_items = if self.active_panel == 0 {
                &self.right_items
            } else {
                &self.left_items
            };

            if !opposite_items.is_empty() {
                let opposite_last_index = opposite_items.len() - 1;
                opposite_state.select(Some(opposite_last_index));
                if self.active_panel == 0 {
                    self.right_scrollbar_state =
                        self.right_scrollbar_state.position(opposite_last_index);
                } else {
                    self.left_scrollbar_state =
                        self.left_scrollbar_state.position(opposite_last_index);
                }
            }
        }
    }

    fn expand_all_folders(node: &mut FileNode) {
        if node.is_dir {
            node.expanded = true;
            for child in &mut node.children {
                Self::expand_all_folders(child);
            }
        }
    }

    fn collapse_all_folders(node: &mut FileNode) {
        if node.is_dir {
            node.expanded = false;
            for child in &mut node.children {
                Self::collapse_all_folders(child);
            }
        }
    }

    pub fn expand_all(&mut self) {
        Self::expand_all_folders(&mut self.comparison.left_tree);
        Self::expand_all_folders(&mut self.comparison.right_tree);
        self.update_file_lists();
    }

    pub fn collapse_all(&mut self) {
        Self::collapse_all_folders(&mut self.comparison.left_tree);
        Self::collapse_all_folders(&mut self.comparison.right_tree);
        self.comparison.left_tree.expanded = true;
        self.comparison.right_tree.expanded = true;
        self.update_file_lists();
    }

    pub fn start_refresh(&mut self) {
        if self.is_refreshing {
            return;
        }

        self.is_refreshing = true;
        self.refresh_progress = "Starting refresh...".to_string();

        let (tx, rx) = mpsc::channel();
        self.refresh_rx = Some(rx);

        let left_dir = self.comparison.left_dir.clone();
        let right_dir = self.comparison.right_dir.clone();

        thread::spawn(move || {
            let result = DirectoryComparison::new_with_progress(left_dir, right_dir, |msg| {
                let percentage = if msg.contains("Starting") {
                    0.0
                } else if msg.contains("Scanning left") {
                    0.05
                } else if msg.contains("Scanning right") {
                    0.15
                } else if msg.contains("Scanning...") {
                    if let Some(files_pos) = msg.find(" files") {
                        let search_str = &msg[..files_pos];
                        if let Some(space_pos) = search_str.rfind(' ') {
                            if let Ok(count) = search_str[space_pos + 1..].parse::<f64>() {
                                (count / 1000.0).min(0.2) + 0.05
                            } else {
                                0.1
                            }
                        } else {
                            0.1
                        }
                    } else {
                        0.1
                    }
                } else if msg.contains("Files to compare:") {
                    0.25
                } else if msg.contains("Progress:") {
                    if let Some(slash_pos) = msg.find('/') {
                        let current_str = msg[9..slash_pos].trim();
                        let total_str = msg[slash_pos + 1..].trim();

                        if let (Ok(current), Ok(total)) =
                            (current_str.parse::<f64>(), total_str.parse::<f64>())
                        {
                            0.25 + (current / total) * 0.75
                        } else {
                            0.5
                        }
                    } else {
                        0.5
                    }
                } else if msg.contains("Comparing...") {
                    if let Some(slash_pos) = msg.find('/') {
                        let search_str = &msg[..slash_pos];
                        if let Some(space_pos) = search_str.rfind(' ') {
                            if let (Ok(current), Ok(total)) = (
                                search_str[space_pos + 1..].parse::<f64>(),
                                msg[slash_pos + 1..].parse::<f64>(),
                            ) {
                                0.25 + (current / total) * 0.7
                            } else {
                                0.5
                            }
                        } else {
                            0.5
                        }
                    } else {
                        0.5
                    }
                } else if msg.contains("Complete") {
                    1.0
                } else {
                    0.5
                };
                let _ = tx.send(RefreshMessage::Progress(msg.to_string(), percentage));
            });

            match result {
                Ok(comparison) => {
                    let _ = tx.send(RefreshMessage::Complete(comparison));
                }
                Err(e) => {
                    let _ = tx.send(RefreshMessage::Error(format!("Error: {}", e)));
                }
            }
        });
    }

    pub fn check_refresh_progress(&mut self) {
        if self.refresh_rx.is_none() {
            return;
        }

        let mut messages = Vec::new();
        if let Some(rx) = &self.refresh_rx {
            while let Ok(msg) = rx.try_recv() {
                messages.push(msg);
            }
        }

        for msg in messages {
            match msg {
                RefreshMessage::Progress(message, percentage) => {
                    self.refresh_progress = message;
                    self.refresh_percentage = percentage;
                }
                RefreshMessage::Complete(comparison) => {
                    self.comparison = comparison;

                    self.comparison.left_tree.expanded = true;
                    self.comparison.right_tree.expanded = true;

                    self.update_file_lists();
                    self.is_refreshing = false;
                    self.refresh_progress.clear();
                    self.refresh_rx = None;

                    if self.saved_expansion_state.is_some() {
                        self.restore_saved_state_safe();
                    }

                    break;
                }
                RefreshMessage::Error(error) => {
                    self.refresh_progress =
                        format!("Refresh failed: {} (Press F5 to retry)", error);
                    self.is_refreshing = false;
                    self.refresh_rx = None;
                    // log_error(&format!("Directory refresh failed: {}", error));
                    break;
                }
            }
        }
    }

    pub fn swap_panels(&mut self) {
        std::mem::swap(
            &mut self.comparison.left_dir,
            &mut self.comparison.right_dir,
        );
        std::mem::swap(
            &mut self.comparison.left_tree,
            &mut self.comparison.right_tree,
        );
        self.update_file_lists();
    }

    pub fn prepare_copy(&mut self) {
        if let Some((_, _, path, is_dir, size, _)) = self.get_selected_item() {
            let from_left_to_right = self.active_panel == 0;

            let source_path = if from_left_to_right {
                self.comparison.left_dir.join(path)
            } else {
                self.comparison.right_dir.join(path)
            };

            let target_path = if from_left_to_right {
                self.comparison.right_dir.join(path)
            } else {
                self.comparison.left_dir.join(path)
            };

            let (file_count, folder_count, total_bytes) = if *is_dir {
                self.calculate_dir_stats(&source_path)
            } else {
                (1, 0, size.unwrap_or(0))
            };

            self.copy_info = Some(CopyInfo {
                source_path,
                target_path,
                file_count,
                folder_count,
                total_bytes,
                from_left_to_right,
            });

            self.mode = AppMode::CopyConfirm;
        }
    }

    fn calculate_dir_stats(&self, dir_path: &std::path::Path) -> (usize, usize, u64) {
        use std::fs;

        let mut file_count = 0;
        let mut folder_count = 1;
        let mut total_bytes = 0;

        if let Ok(entries) = fs::read_dir(dir_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let (sub_files, sub_folders, sub_bytes) = self.calculate_dir_stats(&path);
                    file_count += sub_files;
                    folder_count += sub_folders;
                    total_bytes += sub_bytes;
                } else {
                    file_count += 1;
                    if let Ok(metadata) = entry.metadata() {
                        total_bytes += metadata.len();
                    }
                }
            }
        }

        (file_count, folder_count, total_bytes)
    }

    pub fn execute_copy(&mut self) -> Result<()> {
        if let Some(copy_info) = self.copy_info.clone() {
            use std::fs;

            self.save_current_state();

            if copy_info.source_path.is_dir() {
                self.copy_dir_all(&copy_info.source_path, &copy_info.target_path)?;
            } else {
                if let Some(parent) = copy_info.target_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(&copy_info.source_path, &copy_info.target_path)?;
                self.preserve_file_attributes(&copy_info.source_path, &copy_info.target_path)?;
            }

            let source_parent = copy_info.source_path.parent();
            let is_potentially_large = if let Some(parent) = source_parent {
                std::fs::read_dir(parent)
                    .map(|entries| entries.count() > 1000)
                    .unwrap_or(false)
            } else {
                false
            };

            if is_potentially_large {
                std::thread::sleep(std::time::Duration::from_millis(500));
                // log_info("Large directory detected, using extended sync delay");
            } else {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }

            self.start_refresh();
        }

        self.copy_info = None;
        self.mode = AppMode::DirectoryView;
        Ok(())
    }

    fn copy_dir_all(&self, src: &std::path::Path, dst: &std::path::Path) -> Result<()> {
        use std::fs;

        fs::create_dir_all(dst)?;

        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());

            if src_path.is_dir() {
                self.copy_dir_all(&src_path, &dst_path)?;
                self.preserve_file_attributes(&src_path, &dst_path)?;
            } else {
                fs::copy(&src_path, &dst_path)?;
                self.preserve_file_attributes(&src_path, &dst_path)?;
            }
        }

        self.preserve_file_attributes(src, dst)?;

        Ok(())
    }

    fn preserve_file_attributes(&self, src: &std::path::Path, dst: &std::path::Path) -> Result<()> {
        use std::fs;

        let src_metadata = fs::metadata(src)?;

        if src_metadata.is_file() {
            let modified_time = src_metadata.modified()?;
            let dst_file = fs::File::open(dst)?;
            dst_file.set_modified(modified_time)?;
        }

        Ok(())
    }

    pub fn cancel_copy(&mut self) {
        self.copy_info = None;
        self.mode = AppMode::DirectoryView;
    }

    fn save_current_state(&mut self) {
        self.saved_left_selection = self.left_list_state.selected();
        self.saved_right_selection = self.right_list_state.selected();
        self.saved_active_panel = self.active_panel;

        self.saved_filter_mode = Some(self.filter_mode);

        self.saved_expansion_state = Some((
            self.comparison.left_tree.clone(),
            self.comparison.right_tree.clone(),
        ));
    }

    fn restore_saved_state_safe(&mut self) {
        if let Some(original_filter) = self.saved_filter_mode.take() {
            self.filter_mode = original_filter;
        }

        self.active_panel = self.saved_active_panel;

        use std::io::Write;
        let mut debug = format!(
            "=== BEFORE restoration ===\nLeft tree expanded: {}\nRight tree expanded: {}\n",
            self.comparison.left_tree.expanded, self.comparison.right_tree.expanded
        );

        if let Some((saved_left_tree, saved_right_tree)) = self.saved_expansion_state.take() {
            debug.push_str(&format!(
                "Restoring from saved state - Left: {}, Right: {}\n",
                saved_left_tree.expanded, saved_right_tree.expanded
            ));

            debug.push_str("=== Current Right Tree (before restore) ===\n");
            Self::debug_tree_structure(&self.comparison.right_tree, 0, &mut debug);

            debug.push_str("=== Saved Right Tree ===\n");
            Self::debug_tree_structure(&saved_right_tree, 0, &mut debug);

            Self::restore_expansion_state_safe(&mut self.comparison.left_tree, &saved_left_tree);
            Self::restore_expansion_state_safe(&mut self.comparison.right_tree, &saved_right_tree);

            debug.push_str("=== Current Right Tree (after restore) ===\n");
            Self::debug_tree_structure(&self.comparison.right_tree, 0, &mut debug);
        }

        self.comparison.left_tree.expanded = true;
        self.comparison.right_tree.expanded = true;

        debug.push_str(&format!(
            "=== AFTER restoration ===\nLeft tree expanded: {}\nRight tree expanded: {}\n",
            self.comparison.left_tree.expanded, self.comparison.right_tree.expanded
        ));

        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/tudiff_restore_debug.log")
        {
            let _ = file.write_all(debug.as_bytes());
        }

        self.update_file_lists();

        if let Some(left_selection) = self.saved_left_selection {
            if left_selection < self.left_items.len() {
                self.left_list_state.select(Some(left_selection));
            } else if !self.left_items.is_empty() {
                self.left_list_state.select(Some(0));
            }
        }

        if let Some(right_selection) = self.saved_right_selection {
            if right_selection < self.right_items.len() {
                self.right_list_state.select(Some(right_selection));
            } else if !self.right_items.is_empty() {
                self.right_list_state.select(Some(0));
            }
        }

        self.saved_left_selection = None;
        self.saved_right_selection = None;
        self.saved_expansion_state = None;
        self.saved_filter_mode = None;
    }

    fn restore_expansion_state_safe(current_tree: &mut FileNode, saved_tree: &FileNode) {
        if current_tree.is_dir && saved_tree.is_dir && current_tree.path == saved_tree.path {
            current_tree.expanded = saved_tree.expanded;
        }

        for current_child in &mut current_tree.children {
            if let Some(saved_child) = saved_tree.children.iter().find(|child| {
                child.path == current_child.path && child.is_dir == current_child.is_dir
            }) {
                Self::restore_expansion_state_safe(current_child, saved_child);
            }
        }
    }

    fn debug_tree_structure(node: &FileNode, depth: usize, output: &mut String) {
        let indent = "  ".repeat(depth);
        output.push_str(&format!(
            "{}[{}] expanded:{} status:{:?}\n",
            indent, node.name, node.expanded, node.status
        ));

        for child in &node.children {
            Self::debug_tree_structure(child, depth + 1, output);
        }
    }

    pub fn handle_key_event(&mut self, key: crossterm::event::KeyEvent) -> anyhow::Result<bool> {
        if key.kind == KeyEventKind::Press {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => {
                    if self.mode == AppMode::CopyConfirm {
                        self.cancel_copy();
                    } else {
                        return Ok(true); // Signal to exit
                    }
                }
                KeyCode::Left => {
                    if self.mode == AppMode::DirectoryView {
                        if self.active_panel == 1 {
                            if let Some(right_selected) = self.right_list_state.selected() {
                                if right_selected < self.left_items.len() {
                                    self.left_list_state.select(Some(right_selected));
                                } else if !self.left_items.is_empty() {
                                    self.left_list_state.select(Some(self.left_items.len() - 1));
                                }
                            }
                        }
                        self.active_panel = 0;
                    }
                }
                KeyCode::Right => {
                    if self.mode == AppMode::DirectoryView {
                        if self.active_panel == 0 {
                            if let Some(left_selected) = self.left_list_state.selected() {
                                if left_selected < self.right_items.len() {
                                    self.right_list_state.select(Some(left_selected));
                                } else if !self.right_items.is_empty() {
                                    self.right_list_state
                                        .select(Some(self.right_items.len() - 1));
                                }
                            }
                        }
                        self.active_panel = 1;
                    }
                }
                KeyCode::Up => {
                    if self.mode == AppMode::DirectoryView {
                        self.move_selection(-1);
                    }
                }
                KeyCode::Down => {
                    if self.mode == AppMode::DirectoryView {
                        self.move_selection(1);
                    }
                }
                KeyCode::PageUp => {
                    if self.mode == AppMode::DirectoryView {
                        let half_page = self.calculate_half_page();
                        self.move_selection(-half_page);
                    }
                }
                KeyCode::PageDown => {
                    if self.mode == AppMode::DirectoryView {
                        let half_page = self.calculate_half_page();
                        self.move_selection(half_page);
                    }
                }
                KeyCode::Home => {
                    if key.modifiers.contains(KeyModifiers::CONTROL)
                        && self.mode == AppMode::DirectoryView
                    {
                        // Ctrl+Home: Go to top
                        self.scroll_to_top();
                    }
                }
                KeyCode::End => {
                    if key.modifiers.contains(KeyModifiers::CONTROL)
                        && self.mode == AppMode::DirectoryView
                    {
                        // Ctrl+End: Go to bottom
                        self.scroll_to_bottom();
                    }
                }
                KeyCode::Char('1') => {
                    self.filter_mode = FilterMode::All;
                    self.update_file_lists();
                }
                KeyCode::Char('2') => {
                    self.filter_mode = FilterMode::Different;
                    self.update_file_lists();
                }
                KeyCode::Char('3') => {
                    self.filter_mode = FilterMode::DifferentNotOrphans;
                    self.update_file_lists();
                }
                KeyCode::Char('+') => {
                    self.expand_all();
                }
                KeyCode::Char('-') => {
                    self.collapse_all();
                }
                KeyCode::Char('s') => {
                    self.swap_panels();
                }
                KeyCode::F(5) => {
                    if self.mode == AppMode::DirectoryView {
                        self.start_refresh();
                    }
                }
                KeyCode::Char('r') => {
                    if key
                        .modifiers
                        .contains(crossterm::event::KeyModifiers::CONTROL)
                        && self.mode == AppMode::DirectoryView
                        && self.active_panel == 0
                        && self.can_copy()
                    {
                        self.prepare_copy();
                    }
                }
                KeyCode::Char('l') => {
                    if key
                        .modifiers
                        .contains(crossterm::event::KeyModifiers::CONTROL)
                        && self.mode == AppMode::DirectoryView
                        && self.active_panel == 1
                        && self.can_copy()
                    {
                        self.prepare_copy();
                    }
                }
                KeyCode::Enter => {
                    if self.mode == AppMode::DirectoryView {
                        if let Some((_, status, path, is_dir, _, _)) = self.get_selected_item() {
                            if *is_dir {
                                self.toggle_folder();
                            } else if path.to_string_lossy() != "" {
                                let status = *status;
                                let path = path.clone();
                                self.handle_file_comparison(status, path)?;
                            }
                        }
                    } else if self.mode == AppMode::CopyConfirm {
                        if let Err(e) = self.execute_copy() {
                            eprintln!("Copy failed: {}", e);
                        }
                    } else {
                        self.mode = AppMode::DirectoryView;
                    }
                }
                _ => {}
            }
        }
        Ok(false)
    }

    pub fn handle_mouse_event(&mut self, mouse: crossterm::event::MouseEvent) {
        if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
            self.handle_mouse_click(mouse.column, mouse.row);
        }
    }

    fn handle_file_comparison(&mut self, status: FileStatus, path: PathBuf) -> anyhow::Result<()> {
        let left_path = self.comparison.left_dir.join(&path);
        let right_path = self.comparison.right_dir.join(&path);

        crate::terminal::launch_external_editor(&status, &left_path, &right_path)?;
        Ok(())
    }
}
