use anyhow::Result;
use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind};
use ratatui::{
    layout::Rect,
    widgets::{ListState, ScrollbarState},
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::SystemTime;

use crate::compare::{DirectoryComparison, FileNode, FileStatus};
// use crate::utils::{log_error, log_info};

pub struct FileItem {
    pub display_name: String,
    pub status: FileStatus,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: Option<u64>,
    pub modified: Option<SystemTime>,
}

#[derive(PartialEq, Debug)]
pub enum AppMode {
    DirectoryView,
    #[allow(dead_code)]
    FileView,
    CopyConfirm,
    DeleteConfirm,
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum FilterMode {
    All,
    Different,
    DifferentNotOrphans,
}

enum RefreshMessage {
    Progress(String, f64),
    Complete(Box<DirectoryComparison>),
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

#[derive(Clone)]
pub struct DeleteInfo {
    pub path: PathBuf,
    pub file_count: usize,
    pub folder_count: usize,
    pub total_bytes: u64,
    pub is_left: bool,
}

pub struct App {
    pub comparison: DirectoryComparison,
    pub mode: AppMode,
    pub active_panel: usize,
    pub left_list_state: ListState,
    pub right_list_state: ListState,
    pub left_items: Vec<FileItem>,
    pub right_items: Vec<FileItem>,
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
    pub delete_info: Option<DeleteInfo>,
    saved_left_selection: Option<usize>,
    saved_right_selection: Option<usize>,
    saved_active_panel: usize,
    saved_expansion_state: Option<(HashMap<PathBuf, bool>, HashMap<PathBuf, bool>)>,
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
            delete_info: None,
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
    ) -> Vec<FileItem> {
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
            indent.to_string()
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
            items.push(FileItem {
                display_name,
                status: node.status,
                path: node.path.clone(),
                is_dir: node.is_dir,
                size: node.size,
                modified: node.modified,
            });
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
            } else if relative_x <= 166
                && self.can_delete() {
                    self.prepare_delete();
                }
        }
    }

    pub fn calculate_half_page(&self) -> i32 {
        let available_height = self.viewport_height.saturating_sub(5);
        std::cmp::max(1, (available_height / 2) as i32)
    }

    pub fn get_selected_item(&self) -> Option<&FileItem> {
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
        if let Some(item) = self.get_selected_item() {
            if item.display_name.is_empty() {
                return false;
            }

            match item.status {
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
        if let Some(item) = self.get_selected_item() {
            if item.is_dir {
                let path = item.path.clone();
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
                    let _ = tx.send(RefreshMessage::Complete(Box::new(comparison)));
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
                    self.comparison = *comparison;

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
        if let Some(item) = self.get_selected_item() {
            let from_left_to_right = self.active_panel == 0;

            let source_path = if from_left_to_right {
                self.comparison.left_dir.join(&item.path)
            } else {
                self.comparison.right_dir.join(&item.path)
            };

            let target_path = if from_left_to_right {
                self.comparison.right_dir.join(&item.path)
            } else {
                self.comparison.left_dir.join(&item.path)
            };

            let (file_count, folder_count, total_bytes) = if item.is_dir {
                let source_tree = if from_left_to_right {
                    &self.comparison.left_tree
                } else {
                    &self.comparison.right_tree
                };
                Self::find_node_ref(source_tree, &item.path)
                    .map(Self::calculate_stats_from_node)
                    .unwrap_or((0, 1, 0))
            } else {
                (1, 0, item.size.unwrap_or(0))
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

    fn find_node_ref<'a>(node: &'a FileNode, target: &Path) -> Option<&'a FileNode> {
        if node.path == target {
            return Some(node);
        }
        for child in &node.children {
            if let Some(found) = Self::find_node_ref(child, target) {
                return Some(found);
            }
        }
        None
    }

    fn calculate_stats_from_node(node: &FileNode) -> (usize, usize, u64) {
        let mut file_count = 0usize;
        let mut folder_count = 1usize; // count the directory itself
        let mut total_bytes = 0u64;
        for child in &node.children {
            if child.is_dir {
                // recursive call already counts the child dir itself (folder_count starts at 1)
                let (f, d, b) = Self::calculate_stats_from_node(child);
                file_count += f;
                folder_count += d;
                total_bytes += b;
            } else {
                file_count += 1;
                total_bytes += child.size.unwrap_or(0);
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

            // Partial update instead of full refresh
            self.partial_update_after_copy(&copy_info)?;
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

    fn partial_update_after_copy(&mut self, copy_info: &CopyInfo) -> Result<()> {
        use crate::compare::FileStatus;
        use std::fs;

        // Calculate paths first
        let from_left_to_right = copy_info.from_left_to_right;
        let (source_dir, target_dir) = if from_left_to_right {
            (self.comparison.left_dir.clone(), self.comparison.right_dir.clone())
        } else {
            (self.comparison.right_dir.clone(), self.comparison.left_dir.clone())
        };

        let source_relative = copy_info.source_path.strip_prefix(&source_dir)
            .unwrap_or(&copy_info.source_path).to_path_buf();
        let target_relative = copy_info.target_path.strip_prefix(&target_dir)
            .unwrap_or(&copy_info.target_path).to_path_buf();

        // For directories, copying makes both sides identical by definition
        let are_same = if copy_info.source_path.is_dir() {
            true
        } else {
            Self::check_if_files_same_static(
                &copy_info.source_path,
                &copy_info.target_path
            )?
        };

        let new_status = if are_same {
            FileStatus::Same
        } else {
            FileStatus::Different
        };

        // Get target file metadata
        let target_metadata = fs::metadata(&copy_info.target_path).ok();
        let target_size = target_metadata.as_ref().and_then(|m| {
            if m.is_file() { Some(m.len()) } else { None }
        });
        let target_modified = target_metadata.as_ref().and_then(|m| m.modified().ok());

        // Update nodes
        if from_left_to_right {
            // Update parent folder names (empty nodes)
            Self::update_parent_folder_names(&mut self.comparison.right_tree, &target_relative, &target_dir);

            // Update right tree
            if let Some(target_node) = Self::find_node_in_tree_by_path(
                &mut self.comparison.right_tree,
                &target_relative
            ) {
                // If it was an empty node, update name and metadata
                if target_node.name.is_empty() {
                    if let Some(file_name) = copy_info.target_path.file_name() {
                        target_node.name = file_name.to_string_lossy().to_string();
                    }
                }
                target_node.status = new_status;
                target_node.size = target_size;
                target_node.modified = target_modified;

                if let Some(source_node) = Self::find_node_in_tree_by_path(
                    &mut self.comparison.left_tree,
                    &source_relative
                ) {
                    source_node.status = new_status;
                }
            }
            // Update parent statuses
            Self::update_parent_statuses_static(&mut self.comparison.left_tree, &source_relative);
            Self::update_parent_statuses_static(&mut self.comparison.right_tree, &target_relative);
        } else {
            // Update parent folder names (empty nodes)
            Self::update_parent_folder_names(&mut self.comparison.left_tree, &target_relative, &target_dir);

            // Update left tree
            if let Some(target_node) = Self::find_node_in_tree_by_path(
                &mut self.comparison.left_tree,
                &target_relative
            ) {
                // If it was an empty node, update name and metadata
                if target_node.name.is_empty() {
                    if let Some(file_name) = copy_info.target_path.file_name() {
                        target_node.name = file_name.to_string_lossy().to_string();
                    }
                }
                target_node.status = new_status;
                target_node.size = target_size;
                target_node.modified = target_modified;

                if let Some(source_node) = Self::find_node_in_tree_by_path(
                    &mut self.comparison.right_tree,
                    &source_relative
                ) {
                    source_node.status = new_status;
                }
            }
            // Update parent statuses
            Self::update_parent_statuses_static(&mut self.comparison.right_tree, &source_relative);
            Self::update_parent_statuses_static(&mut self.comparison.left_tree, &target_relative);
        }

        // When copying a directory, recursively update all child nodes to Same
        if copy_info.source_path.is_dir() {
            let descendants = if from_left_to_right {
                Self::collect_descendants_info(&self.comparison.left_tree, &source_relative)
            } else {
                Self::collect_descendants_info(&self.comparison.right_tree, &source_relative)
            };

            {
                let target_tree = if from_left_to_right {
                    &mut self.comparison.right_tree
                } else {
                    &mut self.comparison.left_tree
                };
                for (child_path, child_name) in &descendants {
                    if let Some(node) = Self::find_node_in_tree_by_path(target_tree, child_path) {
                        if node.name.is_empty() {
                            node.name = child_name.clone();
                        }
                        node.status = FileStatus::Same;
                    }
                }
            }

            {
                let source_tree = if from_left_to_right {
                    &mut self.comparison.left_tree
                } else {
                    &mut self.comparison.right_tree
                };
                for (child_path, _) in &descendants {
                    if let Some(node) = Self::find_node_in_tree_by_path(source_tree, child_path) {
                        node.status = FileStatus::Same;
                    }
                }
            }
        }

        // Update UI
        self.update_file_lists();

        // Restore saved state
        if self.saved_expansion_state.is_some() {
            self.restore_saved_state_safe();
        }

        // After copying a directory, expand it on both sides so children are visible.
        // restore_saved_state_safe restores the target folder's expanded=false (it was a
        // placeholder before the copy and was never expanded), hiding all copied children.
        if copy_info.source_path.is_dir() {
            {
                let target_tree = if from_left_to_right {
                    &mut self.comparison.right_tree
                } else {
                    &mut self.comparison.left_tree
                };
                if let Some(node) = Self::find_node_in_tree_by_path(target_tree, &target_relative) {
                    node.expanded = true;
                }
            }
            {
                let source_tree = if from_left_to_right {
                    &mut self.comparison.left_tree
                } else {
                    &mut self.comparison.right_tree
                };
                if let Some(node) = Self::find_node_in_tree_by_path(source_tree, &source_relative) {
                    node.expanded = true;
                }
            }
            self.update_file_lists();
        }

        Ok(())
    }

    fn update_parent_folder_names(tree: &mut FileNode, child_path: &std::path::Path, base_dir: &std::path::Path) {
        // Collect all parent paths
        let mut parent_paths = Vec::new();
        let mut current_path = child_path;

        while let Some(parent) = current_path.parent() {
            if parent.as_os_str().is_empty() {
                break;
            }
            parent_paths.push(parent.to_path_buf());
            current_path = parent;
        }

        // Set names for empty parent folder nodes
        for parent_path in parent_paths {
            if let Some(parent_node) = Self::find_node_in_tree_by_path(tree, &parent_path) {
                if parent_node.name.is_empty() && parent_node.is_dir {
                    // Get actual folder name from filesystem
                    let full_path = base_dir.join(&parent_path);
                    if let Some(folder_name) = full_path.file_name() {
                        parent_node.name = folder_name.to_string_lossy().to_string();
                    }
                }
            }
        }
    }

    fn collect_descendants_info(tree: &FileNode, target_path: &std::path::Path) -> Vec<(PathBuf, String)> {
        fn find_node<'a>(node: &'a FileNode, target: &std::path::Path) -> Option<&'a FileNode> {
            if node.path == target {
                return Some(node);
            }
            for child in &node.children {
                if let Some(found) = find_node(child, target) {
                    return Some(found);
                }
            }
            None
        }

        fn collect_all(node: &FileNode, result: &mut Vec<(PathBuf, String)>) {
            for child in &node.children {
                result.push((child.path.clone(), child.name.clone()));
                if child.is_dir {
                    collect_all(child, result);
                }
            }
        }

        let mut result = Vec::new();
        if let Some(node) = find_node(tree, target_path) {
            collect_all(node, &mut result);
        }
        result
    }

    fn find_node_in_tree_by_path<'a>(
        node: &'a mut FileNode,
        target_path: &std::path::Path,
    ) -> Option<&'a mut FileNode> {
        // Compare current node path with target path
        if node.path == target_path {
            return Some(node);
        }

        // Search child nodes recursively
        for child in &mut node.children {
            if let Some(found) = Self::find_node_in_tree_by_path(child, target_path) {
                return Some(found);
            }
        }

        None
    }

    fn check_if_files_same_static(left_path: &PathBuf, right_path: &PathBuf) -> Result<bool> {
        use std::fs;
        use crate::compare::DirectoryComparison;

        if !left_path.exists() || !right_path.exists() {
            return Ok(false);
        }

        let left_meta = fs::metadata(left_path)?;
        let right_meta = fs::metadata(right_path)?;

        // For directories, return false (need to compare children)
        if left_meta.is_dir() || right_meta.is_dir() {
            return Ok(false);
        }

        DirectoryComparison::files_are_same_public(left_path, right_path, &left_meta, &right_meta)
    }

    fn update_parent_statuses_static(tree: &mut FileNode, child_path: &std::path::Path) {
        use crate::compare::FileStatus;

        // Collect all parent paths
        let mut parent_paths = Vec::new();
        let mut current_path = child_path;

        while let Some(parent) = current_path.parent() {
            if parent.as_os_str().is_empty() {
                break;
            }
            parent_paths.push(parent.to_path_buf());
            current_path = parent;
        }

        // Update parent statuses from leaf upward (innermost parent first)
        // parent_paths is already in [leaf_parent, ..., root] order — do NOT reverse
        for parent_path in parent_paths {
            if let Some(parent_node) = Self::find_node_in_tree_by_path(tree, &parent_path) {
                // Check children statuses to determine parent status
                let child_statuses: Vec<FileStatus> = parent_node.children.iter()
                    .map(|c| c.status)
                    .collect();

                if child_statuses.is_empty() {
                    continue;
                }

                let has_different = child_statuses.contains(&FileStatus::Different);
                let has_left_only = child_statuses.contains(&FileStatus::LeftOnly);
                let has_right_only = child_statuses.contains(&FileStatus::RightOnly);
                let has_same = child_statuses.contains(&FileStatus::Same);

                let new_status = if has_different
                    || (has_left_only && (has_right_only || has_same))
                    || (has_right_only && has_same)
                {
                    FileStatus::Different
                } else if has_left_only {
                    FileStatus::LeftOnly
                } else if has_right_only {
                    FileStatus::RightOnly
                } else {
                    FileStatus::Same
                };

                parent_node.status = new_status;
            }
        }
    }

    pub fn cancel_copy(&mut self) {
        self.copy_info = None;
        self.mode = AppMode::DirectoryView;
    }

    pub fn can_delete(&self) -> bool {
        if let Some(item) = self.get_selected_item() {
            !item.display_name.is_empty()
        } else {
            false
        }
    }

    pub fn prepare_delete(&mut self) {
        if let Some(item) = self.get_selected_item() {
            let is_left = self.active_panel == 0;

            let full_path = if is_left {
                self.comparison.left_dir.join(&item.path)
            } else {
                self.comparison.right_dir.join(&item.path)
            };

            let (file_count, folder_count, total_bytes) = if item.is_dir {
                let tree = if is_left {
                    &self.comparison.left_tree
                } else {
                    &self.comparison.right_tree
                };
                Self::find_node_ref(tree, &item.path)
                    .map(Self::calculate_stats_from_node)
                    .unwrap_or((0, 1, 0))
            } else {
                (1, 0, item.size.unwrap_or(0))
            };

            self.delete_info = Some(DeleteInfo {
                path: full_path,
                file_count,
                folder_count,
                total_bytes,
                is_left,
            });

            self.mode = AppMode::DeleteConfirm;
        }
    }

    pub fn execute_delete(&mut self) -> Result<()> {
        if let Some(delete_info) = self.delete_info.clone() {
            use std::fs;

            self.save_current_state();

            if delete_info.path.is_dir() {
                fs::remove_dir_all(&delete_info.path)?;
            } else {
                fs::remove_file(&delete_info.path)?;
            }

            // Rescan in background — check_refresh_progress restores saved state on completion
            self.start_refresh();
        }

        self.delete_info = None;
        self.mode = AppMode::DirectoryView;
        Ok(())
    }

    pub fn cancel_delete(&mut self) {
        self.delete_info = None;
        self.mode = AppMode::DirectoryView;
    }

    fn save_current_state(&mut self) {
        self.saved_left_selection = self.left_list_state.selected();
        self.saved_right_selection = self.right_list_state.selected();
        self.saved_active_panel = self.active_panel;
        self.saved_filter_mode = Some(self.filter_mode);
        self.saved_expansion_state = Some((
            Self::collect_expansion_map(&self.comparison.left_tree),
            Self::collect_expansion_map(&self.comparison.right_tree),
        ));
    }

    fn collect_expansion_map(node: &FileNode) -> HashMap<PathBuf, bool> {
        let mut map = HashMap::new();
        Self::collect_expansion_recursive(node, &mut map);
        map
    }

    fn collect_expansion_recursive(node: &FileNode, map: &mut HashMap<PathBuf, bool>) {
        if node.is_dir {
            map.insert(node.path.clone(), node.expanded);
            for child in &node.children {
                Self::collect_expansion_recursive(child, map);
            }
        }
    }

    fn restore_saved_state_safe(&mut self) {
        if let Some(original_filter) = self.saved_filter_mode.take() {
            self.filter_mode = original_filter;
        }

        self.active_panel = self.saved_active_panel;

        if let Some((left_map, right_map)) = self.saved_expansion_state.take() {
            Self::restore_expansion_from_map(&mut self.comparison.left_tree, &left_map);
            Self::restore_expansion_from_map(&mut self.comparison.right_tree, &right_map);
        }

        self.comparison.left_tree.expanded = true;
        self.comparison.right_tree.expanded = true;

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

    fn restore_expansion_from_map(node: &mut FileNode, map: &HashMap<PathBuf, bool>) {
        if node.is_dir {
            if let Some(&expanded) = map.get(&node.path) {
                node.expanded = expanded;
            }
            for child in &mut node.children {
                Self::restore_expansion_from_map(child, map);
            }
        }
    }

    pub fn handle_key_event(&mut self, key: crossterm::event::KeyEvent) -> anyhow::Result<bool> {
        if key.kind == KeyEventKind::Press {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => {
                    if self.mode == AppMode::CopyConfirm {
                        self.cancel_copy();
                    } else if self.mode == AppMode::DeleteConfirm {
                        self.cancel_delete();
                    } else {
                        return Ok(true); // Signal to exit
                    }
                }
                KeyCode::Delete => {
                    if self.mode == AppMode::DirectoryView && self.can_delete() {
                        self.prepare_delete();
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
                KeyCode::Char('k') => {
                    if self.mode == AppMode::DirectoryView {
                        self.move_selection(-1);
                    }
                }
                KeyCode::Char('j') => {
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
                KeyCode::Char('h') => {
                    if self.mode == AppMode::DirectoryView {
                        // vim-style navigation: h = left
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
                KeyCode::Char('l') => {
                    if key
                        .modifiers
                        .contains(crossterm::event::KeyModifiers::CONTROL)
                        && self.mode == AppMode::DirectoryView
                        && self.active_panel == 1
                        && self.can_copy()
                    {
                        self.prepare_copy();
                    } else if self.mode == AppMode::DirectoryView {
                        // vim-style navigation: l = right
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
                KeyCode::Char('f') => {
                    if key
                        .modifiers
                        .contains(crossterm::event::KeyModifiers::CONTROL)
                        && self.mode == AppMode::DirectoryView
                    {
                        // Ctrl+F: Page down (forward)
                        let half_page = self.calculate_half_page();
                        self.move_selection(half_page);
                    }
                }
                KeyCode::Char('b') => {
                    if key
                        .modifiers
                        .contains(crossterm::event::KeyModifiers::CONTROL)
                        && self.mode == AppMode::DirectoryView
                    {
                        // Ctrl+B: Page up (backward)
                        let half_page = self.calculate_half_page();
                        self.move_selection(-half_page);
                    }
                }
                KeyCode::Enter => {
                    if self.mode == AppMode::DirectoryView {
                        if let Some(item) = self.get_selected_item() {
                            if item.is_dir {
                                self.toggle_folder();
                            } else if item.path.to_string_lossy() != "" {
                                let status = item.status;
                                let path = item.path.clone();
                                self.handle_file_comparison(status, path)?;
                            }
                        }
                    } else if self.mode == AppMode::CopyConfirm {
                        if let Err(e) = self.execute_copy() {
                            eprintln!("Copy failed: {}", e);
                        }
                    } else if self.mode == AppMode::DeleteConfirm {
                        if let Err(e) = self.execute_delete() {
                            eprintln!("Delete failed: {}", e);
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
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                self.handle_mouse_click(mouse.column, mouse.row);
            }
            MouseEventKind::ScrollUp => {
                if self.mode == AppMode::DirectoryView {
                    self.move_selection(-3); // Scroll up 3 lines
                }
            }
            MouseEventKind::ScrollDown => {
                if self.mode == AppMode::DirectoryView {
                    self.move_selection(3); // Scroll down 3 lines
                }
            }
            _ => {}
        }
    }

    fn handle_file_comparison(&mut self, status: FileStatus, path: PathBuf) -> anyhow::Result<()> {
        let left_path = self.comparison.left_dir.join(&path);
        let right_path = self.comparison.right_dir.join(&path);

        crate::terminal::launch_external_editor(&status, &left_path, &right_path)?;
        Ok(())
    }
}
