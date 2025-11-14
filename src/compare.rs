use anyhow::Result;
use crc32fast::Hasher as Crc32Hasher;
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use walkdir::WalkDir;

#[allow(dead_code)]
pub trait ProgressCallback: Send + Sync {
    fn update(&self, message: &str);
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FileStatus {
    Same,      // File exists on both sides and is identical
    Different, // File exists on both sides but is different
    LeftOnly,  // File exists only on the left side
    RightOnly, // File exists only on the right side
}

#[derive(Debug, Clone)]
pub struct FileNode {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub status: FileStatus,
    pub children: Vec<FileNode>,
    pub expanded: bool,
    pub size: Option<u64>,
    pub modified: Option<SystemTime>,
}

impl FileNode {
    pub fn new(name: String, path: PathBuf, is_dir: bool, status: FileStatus) -> Self {
        Self {
            name,
            path,
            is_dir,
            status,
            children: Vec::new(),
            expanded: false, // All directories start collapsed by default
            size: None,
            modified: None,
        }
    }

    pub fn new_with_metadata(
        name: String,
        path: PathBuf,
        is_dir: bool,
        status: FileStatus,
        metadata: Option<&fs::Metadata>,
    ) -> Self {
        let (size, modified) = if let Some(meta) = metadata {
            let size = if is_dir { None } else { Some(meta.len()) };
            let modified = meta.modified().ok();
            (size, modified)
        } else {
            (None, None)
        };

        Self {
            name,
            path,
            is_dir,
            status,
            children: Vec::new(),
            expanded: false,
            size,
            modified,
        }
    }

    pub fn toggle_expanded(&mut self) {
        if self.is_dir {
            self.expanded = !self.expanded;
        }
    }
}

pub struct DirectoryComparison {
    pub left_tree: FileNode,
    pub right_tree: FileNode,
    pub left_dir: PathBuf,
    pub right_dir: PathBuf,
}

impl DirectoryComparison {
    pub fn new(left_dir: PathBuf, right_dir: PathBuf) -> Result<Self> {
        Self::new_with_logging(left_dir, right_dir, true)
    }

    #[allow(dead_code)]
    pub fn new_silent(left_dir: PathBuf, right_dir: PathBuf) -> Result<Self> {
        Self::new_with_logging(left_dir, right_dir, false)
    }

    pub fn new_with_progress<F>(
        left_dir: PathBuf,
        right_dir: PathBuf,
        mut progress_callback: F,
    ) -> Result<Self>
    where
        F: FnMut(&str),
    {
        crate::utils::log_debug(&format!(
            "Starting comparison: {} vs {}",
            left_dir.display(),
            right_dir.display()
        ));

        progress_callback("Starting directory scan...");

        progress_callback("Scanning left directory...");
        let left_files = match Self::collect_files_with_progress(&left_dir, &mut progress_callback)
        {
            Ok(files) => files,
            Err(e) => {
                crate::utils::log_error(&format!(
                    "Failed to collect left files from {}: {}",
                    left_dir.display(),
                    e
                ));
                return Err(e);
            }
        };

        progress_callback("Scanning right directory...");
        let right_files =
            match Self::collect_files_with_progress(&right_dir, &mut progress_callback) {
                Ok(files) => files,
                Err(e) => {
                    crate::utils::log_error(&format!(
                        "Failed to collect right files from {}: {}",
                        right_dir.display(),
                        e
                    ));
                    return Err(e);
                }
            };

        progress_callback("Comparing files...");
        let (left_tree, right_tree) = match Self::compare_trees_with_progress(
            &left_dir,
            &right_dir,
            &left_files,
            &right_files,
            &mut progress_callback,
        ) {
            Ok(trees) => trees,
            Err(e) => {
                crate::utils::log_error(&format!("Failed to compare trees: {}", e));
                return Err(e);
            }
        };

        progress_callback("Complete!");
        crate::utils::log_debug("Comparison completed successfully");

        Ok(Self {
            left_tree,
            right_tree,
            left_dir,
            right_dir,
        })
    }

    fn new_with_logging(
        left_dir: PathBuf,
        right_dir: PathBuf,
        enable_logging: bool,
    ) -> Result<Self> {
        let left_files = Self::collect_files(&left_dir, enable_logging)?;
        let right_files = Self::collect_files(&right_dir, enable_logging)?;
        let (left_tree, right_tree) = Self::compare_trees(
            &left_dir,
            &right_dir,
            &left_files,
            &right_files,
            enable_logging,
        )?;

        Ok(Self {
            left_tree,
            right_tree,
            left_dir,
            right_dir,
        })
    }

    fn collect_files(dir: &Path, enable_logging: bool) -> Result<HashMap<PathBuf, fs::Metadata>> {
        let mut files = HashMap::new();
        let mut count = 0;

        for entry in WalkDir::new(dir) {
            let entry = entry?;
            let relative_path = entry.path().strip_prefix(dir)?.to_path_buf();
            let metadata = entry.metadata()?;
            files.insert(relative_path, metadata);

            count += 1;
            if enable_logging && count % 100 == 0 {
                eprint!(".");
            }
        }

        if enable_logging && count >= 100 {
            eprintln!();
        }

        Ok(files)
    }

    fn collect_files_with_progress<F>(
        dir: &Path,
        progress_callback: &mut F,
    ) -> Result<HashMap<PathBuf, fs::Metadata>>
    where
        F: FnMut(&str),
    {
        let mut files = HashMap::new();
        let mut count = 0;

        for entry in WalkDir::new(dir) {
            let entry = entry?;
            let relative_path = entry.path().strip_prefix(dir)?.to_path_buf();
            let metadata = entry.metadata()?;
            files.insert(relative_path, metadata);

            count += 1;
            if count % 50 == 0 {
                progress_callback(&format!("Scanning... {} files", count));
            }
        }

        Ok(files)
    }

    fn compare_trees(
        left_dir: &Path,
        right_dir: &Path,
        left_files: &HashMap<PathBuf, fs::Metadata>,
        right_files: &HashMap<PathBuf, fs::Metadata>,
        enable_logging: bool,
    ) -> Result<(FileNode, FileNode)> {
        let left_name = left_dir
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let right_name = right_dir
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let mut left_root =
            FileNode::new(left_name, left_dir.to_path_buf(), true, FileStatus::Same);
        let mut right_root =
            FileNode::new(right_name, right_dir.to_path_buf(), true, FileStatus::Same);

        // Root directory starts expanded
        left_root.expanded = true;
        right_root.expanded = true;

        // Collect all unique paths
        let mut all_paths = std::collections::BTreeSet::new();
        all_paths.extend(left_files.keys().cloned());
        all_paths.extend(right_files.keys().cloned());

        let total_paths = all_paths.len();
        if enable_logging {
            eprintln!("🔀 Processing {} unique paths...", total_paths);
        }

        // Convert paths to tree structure
        let mut processed = 0;
        for path in all_paths {
            if path.as_os_str().is_empty() {
                continue; // Skip root path
            }

            let left_exists = left_files.contains_key(&path);
            let right_exists = right_files.contains_key(&path);
            let left_meta = left_files.get(&path);
            let right_meta = right_files.get(&path);

            let status = match (left_exists, right_exists) {
                (true, true) => {
                    if left_meta.unwrap().is_file() && right_meta.unwrap().is_file() {
                        // Compare file contents
                        let left_path = left_dir.join(&path);
                        let right_path = right_dir.join(&path);

                        if enable_logging && processed % 100 == 0 && processed > 0 {
                            eprintln!("   🔍 Comparing file: {}", path.display());
                        }

                        if Self::files_are_same(
                            &left_path,
                            &right_path,
                            left_meta.unwrap(),
                            right_meta.unwrap(),
                        )? {
                            FileStatus::Same
                        } else {
                            FileStatus::Different
                        }
                    } else {
                        FileStatus::Same // Assume directories are same for now
                    }
                }
                (true, false) => FileStatus::LeftOnly,
                (false, true) => FileStatus::RightOnly,
                (false, false) => unreachable!(),
            };

            let is_dir = left_meta
                .map(|m| m.is_dir())
                .or(right_meta.map(|m| m.is_dir()))
                .unwrap_or(false);
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            // Insert only items that exist in each panel
            // For LeftOnly/RightOnly, insert empty nodes on opposite side for alignment
            match status {
                FileStatus::Same | FileStatus::Different => {
                    // Exists on both sides
                    Self::insert_into_tree(
                        &mut left_root,
                        &path,
                        name.clone(),
                        is_dir,
                        status,
                        true,
                        left_meta,
                    )?;
                    Self::insert_into_tree(
                        &mut right_root,
                        &path,
                        name,
                        is_dir,
                        status,
                        true,
                        right_meta,
                    )?;
                }
                FileStatus::LeftOnly => {
                    // Left side only
                    Self::insert_into_tree(
                        &mut left_root,
                        &path,
                        name.clone(),
                        is_dir,
                        status,
                        true,
                        left_meta,
                    )?;
                    Self::insert_into_tree(
                        &mut right_root,
                        &path,
                        "".to_string(),
                        is_dir,
                        status,
                        false,
                        None,
                    )?;
                }
                FileStatus::RightOnly => {
                    // Right side only
                    Self::insert_into_tree(
                        &mut left_root,
                        &path,
                        "".to_string(),
                        is_dir,
                        status,
                        false,
                        None,
                    )?;
                    Self::insert_into_tree(
                        &mut right_root,
                        &path,
                        name,
                        is_dir,
                        status,
                        true,
                        right_meta,
                    )?;
                }
            }

            processed += 1;
        }

        // Sort children at all levels after tree construction
        Self::sort_tree_recursive(&mut left_root);
        Self::sort_tree_recursive(&mut right_root);

        // Update folder status based on children's status
        Self::update_folder_status(&mut left_root);
        Self::update_folder_status(&mut right_root);

        Ok((left_root, right_root))
    }

    fn compare_trees_with_progress<F>(
        left_dir: &Path,
        right_dir: &Path,
        left_files: &HashMap<PathBuf, fs::Metadata>,
        right_files: &HashMap<PathBuf, fs::Metadata>,
        progress_callback: &mut F,
    ) -> Result<(FileNode, FileNode)>
    where
        F: FnMut(&str),
    {
        let left_name = left_dir
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let right_name = right_dir
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let mut left_root =
            FileNode::new(left_name, left_dir.to_path_buf(), true, FileStatus::Same);
        let mut right_root =
            FileNode::new(right_name, right_dir.to_path_buf(), true, FileStatus::Same);

        // Root folder always starts expanded
        left_root.expanded = true;
        right_root.expanded = true;

        // Collect all unique paths
        let mut all_paths = std::collections::BTreeSet::new();
        all_paths.extend(left_files.keys().cloned());
        all_paths.extend(right_files.keys().cloned());

        let total_paths = all_paths.len();
        progress_callback(&format!("Files to compare: {}", total_paths));
        progress_callback("Processing paths...");

        // Convert paths to tree structure
        let mut processed = 0;
        for path in all_paths {
            if path.as_os_str().is_empty() {
                continue; // Skip empty paths
            }

            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let left_meta = left_files.get(&path);
            let right_meta = right_files.get(&path);
            let is_dir = left_meta
                .map(|m| m.is_dir())
                .or_else(|| right_meta.map(|m| m.is_dir()))
                .unwrap_or(false);

            crate::utils::log_debug(&format!(
                "Path analysis: {} -> left_meta exists: {}, right_meta exists: {}, is_dir: {}",
                path.display(),
                left_meta.is_some(),
                right_meta.is_some(),
                is_dir
            ));

            if let Some(left) = left_meta {
                crate::utils::log_debug(&format!(
                    "Left metadata for {}: is_dir={}, is_file={}, len={}",
                    path.display(),
                    left.is_dir(),
                    left.is_file(),
                    left.len()
                ));
            }
            if let Some(right) = right_meta {
                crate::utils::log_debug(&format!(
                    "Right metadata for {}: is_dir={}, is_file={}, len={}",
                    path.display(),
                    right.is_dir(),
                    right.is_file(),
                    right.len()
                ));
            }

            let status = match (left_meta, right_meta) {
                (Some(_), None) => FileStatus::LeftOnly,
                (None, Some(_)) => FileStatus::RightOnly,
                (Some(left), Some(right)) => {
                    if is_dir {
                        FileStatus::Same
                    } else {
                        // Compare file contents
                        let left_path = left_dir.join(&path);
                        let right_path = right_dir.join(&path);

                        if processed % 100 == 0 && processed > 0 {
                            progress_callback(&format!(
                                "Comparing... {}/{}",
                                processed, total_paths
                            ));
                        }

                        crate::utils::log_debug(&format!(
                            "About to compare files: {} vs {}",
                            left_path.display(),
                            right_path.display()
                        ));

                        if match Self::files_are_same(&left_path, &right_path, left, right) {
                            Ok(same) => {
                                crate::utils::log_debug(&format!(
                                    "File comparison completed: {} vs {} -> {}",
                                    left_path.display(),
                                    right_path.display(),
                                    same
                                ));
                                same
                            }
                            Err(e) => {
                                crate::utils::log_error(&format!(
                                    "CRITICAL ERROR in files_are_same: {} vs {} - {}",
                                    left_path.display(),
                                    right_path.display(),
                                    e
                                ));
                                return Err(e);
                            }
                        } {
                            FileStatus::Same
                        } else {
                            FileStatus::Different
                        }
                    }
                }
                (None, None) => FileStatus::Same, // This shouldn't actually happen
            };

            // Insert node into tree
            match status {
                FileStatus::LeftOnly => {
                    Self::insert_into_tree(
                        &mut left_root,
                        &path,
                        name.clone(),
                        is_dir,
                        status,
                        false,
                        left_meta,
                    )?;
                    Self::insert_into_tree(
                        &mut right_root,
                        &path,
                        String::new(),
                        is_dir,
                        status,
                        true,
                        None,
                    )?;
                }
                FileStatus::RightOnly => {
                    Self::insert_into_tree(
                        &mut left_root,
                        &path,
                        String::new(),
                        is_dir,
                        status,
                        true,
                        None,
                    )?;
                    Self::insert_into_tree(
                        &mut right_root,
                        &path,
                        name.clone(),
                        is_dir,
                        status,
                        false,
                        right_meta,
                    )?;
                }
                _ => {
                    Self::insert_into_tree(
                        &mut left_root,
                        &path,
                        name.clone(),
                        is_dir,
                        status,
                        false,
                        left_meta,
                    )?;
                    Self::insert_into_tree(
                        &mut right_root,
                        &path,
                        name,
                        is_dir,
                        status,
                        false,
                        right_meta,
                    )?;
                }
            }

            processed += 1;
            if processed % 10 == 0 || processed == total_paths {
                progress_callback(&format!("Progress: {}/{}", processed, total_paths));
            }
        }

        // Sort children at all levels after tree construction
        Self::sort_tree_recursive(&mut left_root);
        Self::sort_tree_recursive(&mut right_root);

        // Update folder status based on children's status
        Self::update_folder_status(&mut left_root);
        Self::update_folder_status(&mut right_root);

        Ok((left_root, right_root))
    }

    fn sort_tree_recursive(node: &mut FileNode) {
        // Sort children: folders first, then case-insensitive alphabetical
        node.children.sort_by(|a, b| {
            let a_name = if a.name.is_empty() {
                a.path
                    .file_name()
                    .unwrap_or_default()
                    .to_str()
                    .unwrap_or("")
            } else {
                &a.name
            };
            let b_name = if b.name.is_empty() {
                b.path
                    .file_name()
                    .unwrap_or_default()
                    .to_str()
                    .unwrap_or("")
            } else {
                &b.name
            };

            // Folders first, then files
            match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less, // folder < file
                (false, true) => std::cmp::Ordering::Greater, // file > folder
                _ => {
                    // Same type (both folders or both files) - case insensitive alphabetical
                    a_name.to_lowercase().cmp(&b_name.to_lowercase())
                }
            }
        });

        // Recursively sort all child nodes
        for child in &mut node.children {
            Self::sort_tree_recursive(child);
        }
    }

    fn update_folder_status(node: &mut FileNode) -> FileStatus {
        if !node.is_dir {
            return node.status;
        }

        // Recursively update and collect children's status
        let mut child_statuses = Vec::new();
        for child in &mut node.children {
            let child_status = Self::update_folder_status(child);
            child_statuses.push(child_status);
        }

        // Folder status is determined by children's status
        let new_status = if child_statuses.is_empty() {
            // Empty folders maintain original status
            node.status
        } else {
            // Analyze children's status
            let has_different = child_statuses.iter().any(|&s| s == FileStatus::Different);
            let has_left_only = child_statuses.iter().any(|&s| s == FileStatus::LeftOnly);
            let has_right_only = child_statuses.iter().any(|&s| s == FileStatus::RightOnly);
            let has_same = child_statuses.iter().any(|&s| s == FileStatus::Same);

            if has_different {
                // If any child is Different, folder is Different
                FileStatus::Different
            } else if has_left_only && has_right_only {
                // If has both LeftOnly and RightOnly children, folder is Different
                FileStatus::Different
            } else if has_left_only && has_same {
                // If has both LeftOnly and Same children, folder is Different
                FileStatus::Different
            } else if has_right_only && has_same {
                // If has both RightOnly and Same children, folder is Different
                FileStatus::Different
            } else if has_left_only {
                // If all children are LeftOnly, folder is LeftOnly
                FileStatus::LeftOnly
            } else if has_right_only {
                // If all children are RightOnly, folder is RightOnly
                FileStatus::RightOnly
            } else {
                // If all children are Same, folder is Same
                FileStatus::Same
            }
        };

        node.status = new_status;
        new_status
    }

    #[allow(dead_code)]
    fn debug_print_tree(node: &FileNode, depth: usize) {
        let indent = "  ".repeat(depth);
        eprintln!(
            "{}🔍 '{}' (empty: {}, status: {:?}, is_dir: {})",
            indent,
            node.name,
            node.name.is_empty(),
            node.status,
            node.is_dir
        );
        for child in &node.children {
            Self::debug_print_tree(child, depth + 1);
        }
    }

    pub fn files_are_same_public(
        left: &Path,
        right: &Path,
        left_meta: &fs::Metadata,
        right_meta: &fs::Metadata,
    ) -> Result<bool> {
        Self::files_are_same(left, right, left_meta, right_meta)
    }

    fn files_are_same(
        left: &Path,
        right: &Path,
        left_meta: &fs::Metadata,
        right_meta: &fs::Metadata,
    ) -> Result<bool> {
        crate::utils::log_debug(&format!(
            "files_are_same: Starting comparison - {} vs {}",
            left.display(),
            right.display()
        ));

        crate::utils::log_debug(&format!("files_are_same: File type check - {} (is_dir: {}, is_file: {}) vs {} (is_dir: {}, is_file: {})",
                               left.display(), left_meta.is_dir(), left_meta.is_file(),
                               right.display(), right_meta.is_dir(), right_meta.is_file()));

        // Double check if either path is actually a directory by checking the filesystem directly
        let left_real_meta = match fs::metadata(left) {
            Ok(meta) => {
                crate::utils::log_debug(&format!(
                    "files_are_same: Real filesystem check for {}: is_dir={}, is_file={}",
                    left.display(),
                    meta.is_dir(),
                    meta.is_file()
                ));
                Some(meta)
            }
            Err(e) => {
                crate::utils::log_debug(&format!(
                    "files_are_same: Failed to get real metadata for {}: {}",
                    left.display(),
                    e
                ));
                None
            }
        };

        let right_real_meta = match fs::metadata(right) {
            Ok(meta) => {
                crate::utils::log_debug(&format!(
                    "files_are_same: Real filesystem check for {}: is_dir={}, is_file={}",
                    right.display(),
                    meta.is_dir(),
                    meta.is_file()
                ));
                Some(meta)
            }
            Err(e) => {
                crate::utils::log_debug(&format!(
                    "files_are_same: Failed to get real metadata for {}: {}",
                    right.display(),
                    e
                ));
                None
            }
        };

        // If either is actually a directory, return false immediately
        if left_real_meta.as_ref().map_or(false, |m| m.is_dir())
            || right_real_meta.as_ref().map_or(false, |m| m.is_dir())
        {
            crate::utils::log_debug(&format!("files_are_same: At least one path is actually a directory - {} (is_dir: {}) vs {} (is_dir: {})",
                                   left.display(),
                                   left_real_meta.as_ref().map_or(false, |m| m.is_dir()),
                                   right.display(),
                                   right_real_meta.as_ref().map_or(false, |m| m.is_dir())));
            return Ok(false);
        }

        if !left.exists() || !right.exists() {
            crate::utils::log_debug(&format!(
                "files_are_same: One file doesn't exist - {} (exists: {}) vs {} (exists: {})",
                left.display(),
                left.exists(),
                right.display(),
                right.exists()
            ));
            return Ok(false);
        }

        // Stage 1: File size comparison (fastest)
        if left_meta.len() != right_meta.len() {
            crate::utils::log_debug(&format!(
                "files_are_same: Different sizes - {} ({} bytes) vs {} ({} bytes)",
                left.display(),
                left_meta.len(),
                right.display(),
                right_meta.len()
            ));
            return Ok(false);
        }

        // Stage 2: Skip time comparison (removed for reliability)

        // Stage 3: Zero-size files are considered same
        if left_meta.len() == 0 {
            crate::utils::log_debug(&format!(
                "files_are_same: Zero-size files considered same - {} vs {}",
                left.display(),
                right.display()
            ));
            return Ok(true);
        }

        // Stage 4: Small files (<4KB) - content comparison
        if left_meta.len() < 4096 {
            crate::utils::log_debug(&format!(
                "files_are_same: Using content comparison for small files ({} bytes) - {} vs {}",
                left_meta.len(),
                left.display(),
                right.display()
            ));
            let left_content = match fs::read(left) {
                Ok(content) => {
                    crate::utils::log_debug(&format!(
                        "files_are_same: Left content read successfully - {} ({} bytes)",
                        left.display(),
                        content.len()
                    ));
                    content
                }
                Err(e) => {
                    crate::utils::log_error(&format!(
                        "CRITICAL ERROR reading left small file: {} - {}",
                        left.display(),
                        e
                    ));
                    return Err(e.into());
                }
            };
            let right_content = match fs::read(right) {
                Ok(content) => {
                    crate::utils::log_debug(&format!(
                        "files_are_same: Right content read successfully - {} ({} bytes)",
                        right.display(),
                        content.len()
                    ));
                    content
                }
                Err(e) => {
                    crate::utils::log_error(&format!(
                        "CRITICAL ERROR reading right small file: {} - {}",
                        right.display(),
                        e
                    ));
                    return Err(e.into());
                }
            };
            let result = left_content == right_content;
            crate::utils::log_debug(&format!(
                "files_are_same: Small file content comparison result: {} - {} vs {}",
                result,
                left.display(),
                right.display()
            ));
            return Ok(result);
        }

        // Stage 5: Medium files (<1MB) - CRC32 comparison (faster than SHA256)
        if left_meta.len() < 1024 * 1024 {
            crate::utils::log_debug(&format!(
                "files_are_same: Using CRC32 comparison for medium files ({} bytes) - {} vs {}",
                left_meta.len(),
                left.display(),
                right.display()
            ));
            return Self::compare_file_crc32(left, right);
        }

        // Stage 6: Large files (≥1MB) - compare first 4KB only (quick check)
        crate::utils::log_debug(&format!(
            "files_are_same: Using head comparison for large files ({} bytes) - {} vs {}",
            left_meta.len(),
            left.display(),
            right.display()
        ));
        Self::compare_file_heads(left, right, 4096)
    }

    fn compare_file_crc32(left: &Path, right: &Path) -> Result<bool> {
        crate::utils::log_debug(&format!(
            "Starting CRC32 comparison: {} vs {}",
            left.display(),
            right.display()
        ));

        let left_crc = match Self::calculate_file_crc32(left) {
            Ok(crc) => {
                crate::utils::log_debug(&format!(
                    "Left CRC32 calculated successfully: {} (0x{:08x})",
                    left.display(),
                    crc
                ));
                crc
            }
            Err(e) => {
                crate::utils::log_error(&format!(
                    "Failed to calculate left CRC32 for {}: {}",
                    left.display(),
                    e
                ));
                return Err(e);
            }
        };

        let right_crc = match Self::calculate_file_crc32(right) {
            Ok(crc) => {
                crate::utils::log_debug(&format!(
                    "Right CRC32 calculated successfully: {} (0x{:08x})",
                    right.display(),
                    crc
                ));
                crc
            }
            Err(e) => {
                crate::utils::log_error(&format!(
                    "Failed to calculate right CRC32 for {}: {}",
                    right.display(),
                    e
                ));
                return Err(e);
            }
        };

        let result = left_crc == right_crc;
        crate::utils::log_debug(&format!(
            "CRC32 comparison result: {} (left: 0x{:08x}, right: 0x{:08x})",
            result,
            left_crc,
            right_crc
        ));
        Ok(result)
    }


    fn calculate_file_crc32(path: &Path) -> Result<u32> {
        crate::utils::log_debug(&format!("Calculating CRC32 for: {}", path.display()));

        // Check if path is a directory first
        let metadata = match fs::metadata(path) {
            Ok(meta) => {
                crate::utils::log_debug(&format!(
                    "Metadata obtained for: {} (is_dir: {}, is_file: {})",
                    path.display(),
                    meta.is_dir(),
                    meta.is_file()
                ));
                meta
            }
            Err(e) => {
                crate::utils::log_error(&format!(
                    "Failed to get metadata for path: {} - {}",
                    path.display(),
                    e
                ));
                return Err(e.into());
            }
        };

        if metadata.is_dir() {
            // For directories, return a fixed CRC32 based on the path
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};

            let mut hasher = DefaultHasher::new();
            path.hash(&mut hasher);
            crate::utils::log_debug(&format!("Using directory CRC32 for: {}", path.display()));
            return Ok(hasher.finish() as u32);
        }

        // For files, calculate content CRC32
        crate::utils::log_debug(&format!(
            "Opening file for CRC32 calculation: {}",
            path.display()
        ));
        let mut file = match fs::File::open(path) {
            Ok(f) => {
                crate::utils::log_debug(&format!("File opened successfully: {}", path.display()));
                f
            }
            Err(e) => {
                crate::utils::log_error(&format!(
                    "CRITICAL: Failed to open file: {} - {}",
                    path.display(),
                    e
                ));
                return Err(e.into());
            }
        };

        let mut hasher = Crc32Hasher::new();
        let mut buffer = [0; 8192];
        let mut total_bytes = 0;

        loop {
            let bytes_read = match file.read(&mut buffer) {
                Ok(n) => n,
                Err(e) => {
                    crate::utils::log_error(&format!(
                        "Failed to read file: {} after {} bytes - {}",
                        path.display(),
                        total_bytes,
                        e
                    ));
                    return Err(e.into());
                }
            };
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
            total_bytes += bytes_read;
        }

        let crc = hasher.finalize();
        crate::utils::log_debug(&format!(
            "CRC32 calculation completed for: {} ({} bytes) -> 0x{:08x}",
            path.display(),
            total_bytes,
            crc
        ));
        Ok(crc)
    }


    fn compare_file_heads(left: &Path, right: &Path, bytes_to_read: usize) -> Result<bool> {
        crate::utils::log_debug(&format!(
            "Starting file head comparison: {} vs {} ({} bytes)",
            left.display(),
            right.display(),
            bytes_to_read
        ));

        // Check if either path is a directory
        let left_metadata = match fs::metadata(left) {
            Ok(meta) => {
                crate::utils::log_debug(&format!(
                    "Left metadata: {} (is_dir: {}, is_file: {})",
                    left.display(),
                    meta.is_dir(),
                    meta.is_file()
                ));
                meta
            }
            Err(e) => {
                crate::utils::log_error(&format!(
                    "Failed to get left metadata for: {} - {}",
                    left.display(),
                    e
                ));
                return Err(e.into());
            }
        };

        let right_metadata = match fs::metadata(right) {
            Ok(meta) => {
                crate::utils::log_debug(&format!(
                    "Right metadata: {} (is_dir: {}, is_file: {})",
                    right.display(),
                    meta.is_dir(),
                    meta.is_file()
                ));
                meta
            }
            Err(e) => {
                crate::utils::log_error(&format!(
                    "Failed to get right metadata for: {} - {}",
                    right.display(),
                    e
                ));
                return Err(e.into());
            }
        };

        if left_metadata.is_dir() || right_metadata.is_dir() {
            // If either is a directory, they can't have the same content
            crate::utils::log_debug(&format!(
                "Skipping directory comparison: {} (is_dir: {}) vs {} (is_dir: {})",
                left.display(),
                left_metadata.is_dir(),
                right.display(),
                right_metadata.is_dir()
            ));
            return Ok(false);
        }

        crate::utils::log_debug(&format!(
            "Opening left file for head comparison: {}",
            left.display()
        ));
        let mut left_file = match fs::File::open(left) {
            Ok(f) => {
                crate::utils::log_debug(&format!(
                    "Left file opened successfully: {}",
                    left.display()
                ));
                f
            }
            Err(e) => {
                crate::utils::log_error(&format!(
                    "CRITICAL: Failed to open left file: {} - {}",
                    left.display(),
                    e
                ));
                crate::utils::log_error(&format!(
                    "Left file type check - is_file: {}, is_dir: {}",
                    left_metadata.is_file(),
                    left_metadata.is_dir()
                ));
                return Err(e.into());
            }
        };

        crate::utils::log_debug(&format!(
            "Opening right file for head comparison: {}",
            right.display()
        ));
        let mut right_file = match fs::File::open(right) {
            Ok(f) => {
                crate::utils::log_debug(&format!(
                    "Right file opened successfully: {}",
                    right.display()
                ));
                f
            }
            Err(e) => {
                crate::utils::log_error(&format!(
                    "CRITICAL: Failed to open right file: {} - {}",
                    right.display(),
                    e
                ));
                crate::utils::log_error(&format!(
                    "Right file type check - is_file: {}, is_dir: {}",
                    right_metadata.is_file(),
                    right_metadata.is_dir()
                ));
                return Err(e.into());
            }
        };

        let mut left_buffer = vec![0; bytes_to_read];
        let mut right_buffer = vec![0; bytes_to_read];

        let left_bytes = left_file.read(&mut left_buffer)?;
        let right_bytes = right_file.read(&mut right_buffer)?;

        if left_bytes != right_bytes {
            return Ok(false);
        }

        Ok(left_buffer[..left_bytes] == right_buffer[..right_bytes])
    }

    fn insert_into_tree(
        root: &mut FileNode,
        path: &Path,
        name: String,
        is_dir: bool,
        status: FileStatus,
        _exists: bool,
        metadata: Option<&fs::Metadata>,
    ) -> Result<()> {
        let components: Vec<_> = path.components().collect();
        let mut current = root;

        for (i, component) in components.iter().enumerate() {
            let component_name = component.as_os_str().to_string_lossy().to_string();
            let is_last = i == components.len() - 1;

            // Find or create child with matching name at current level (path-based matching)
            let child_index = current.children.iter().position(|child| {
                // Match based on path (same path = same node even with empty name)
                child.path.file_name().unwrap_or_default() == component.as_os_str()
            });

            if let Some(index) = child_index {
                if is_last {
                    // Update status if this is the last component
                    current.children[index].status = status;
                }
                current = &mut current.children[index];
            } else {
                // Create new node
                let child_path = if i == 0 {
                    PathBuf::from(&component_name)
                } else {
                    components.iter().take(i + 1).collect()
                };

                let child_status = if is_last { status } else { FileStatus::Same };
                let child_is_dir = if is_last { is_dir } else { true };

                // Handle names for non-existent files
                let actual_name = if is_last {
                    // Always use the actual name instead of empty string
                    name.clone()
                } else {
                    component_name.clone()
                };

                let new_child = if is_last {
                    FileNode::new_with_metadata(
                        actual_name,
                        child_path,
                        child_is_dir,
                        child_status,
                        metadata,
                    )
                } else {
                    FileNode::new(actual_name, child_path, child_is_dir, child_status)
                };

                current.children.push(new_child);
                let new_index = current.children.len() - 1;
                current = &mut current.children[new_index];
            }
        }

        Ok(())
    }

    #[allow(dead_code)]
    pub fn get_file_diff(&self, left_path: &Path, right_path: &Path) -> Result<String> {
        if !left_path.exists() {
            return Ok(format!(
                "File only exists in right directory: {}",
                right_path.display()
            ));
        }
        if !right_path.exists() {
            return Ok(format!(
                "File only exists in left directory: {}",
                left_path.display()
            ));
        }

        let left_content = fs::read_to_string(left_path)?;
        let right_content = fs::read_to_string(right_path)?;

        let diff = similar::TextDiff::from_lines(&left_content, &right_content);
        let mut output = String::new();

        for change in diff.iter_all_changes() {
            let sign = match change.tag() {
                similar::ChangeTag::Delete => "-",
                similar::ChangeTag::Insert => "+",
                similar::ChangeTag::Equal => " ",
            };
            output.push_str(&format!("{}{}", sign, change));
        }

        Ok(output)
    }
}
