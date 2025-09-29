# tudiff

<!-- [![Crates.io](https://img.shields.io/crates/v/tudiff)](https://crates.io/crates/tudiff) -->

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-blue.svg)](https://www.rust-lang.org)

<!-- [![GitHub](https://img.shields.io/github/stars/withoutsalsu/tudiff?style=social)](https://github.com/withoutsalsu/tudiff) -->

A TUI-based directory comparison tool written in Rust - Beyond Compare style file comparator.

🇰🇷 [한글 문서](README_KO.md)

## Screenshot

![tudiff in action](https://raw.githubusercontent.com/withoutsalsu/tudiff/main/assets/screenshot.png)

## Features

- **Dual-panel directory trees**: Side-by-side comparison of two directories
- **Color coding**:
  - Gray: Identical files/folders
  - Red: Different files
  - Blue: Files/folders that exist on one side only
- **Folder expansion/collapse**: Use Enter key to expand or collapse folders
- **Synchronized navigation**: Scroll and folder expansion states are synchronized between panels
- **file comparison**: File content comparison using vimdiff
- **Three filter modes**:
  - All files (shows everything)
  - Different files (shows only files that differ)
  - Different files excluding orphans (shows only files that exist on both sides but are different)
- **Interactive toolbar**: Click on toolbar buttons to activate functions
- **Smart copy functionality**: Copy files with state preservation
  - Preserves cursor position and expansion state after copy operations
  - Maintains file attributes (timestamps, permissions) during copy
- **Terminal-safe**: Proper cursor restoration and panic-safe cleanup

## Installation and Usage

### Requirements

- Rust (1.70+)
- vim or nano (for file comparison)
- Terminal with Unicode support (for emoji icons)

### Install from Source

```bash
git clone https://github.com/withoutsalsu/tudiff.git
cd tudiff

# Build only
cargo build --release

# Or install to cargo bin directory
cargo install --path .
```

### Usage

```bash
# For development
cargo run -- <dir1> <dir2>

# If built from source
./target/release/tudiff <dir1> <dir2>

# If installed with cargo install
tudiff <dir1> <dir2>

# Use simple text output instead of TUI
tudiff --simple <dir1> <dir2>
cargo run -- --simple <dir1> <dir2>
```

**Example:**

```bash
# Compare two project directories
tudiff ./project-v1 ./project-v2

# Compare backup with original
tudiff ~/Documents /backup/Documents

# Use simple text output for scripting or piping
tudiff --simple ./project-v1 ./project-v2 | grep "\[L\]"
```

## Controls

### Mouse Controls

- **Toolbar clicking**: Click on toolbar buttons to activate functions
- **Filter modes**: Click on "All Files", "Different", or "Diff Only" to switch filter modes
- **Actions**: Click on "Expand All", "Collapse All", "Refresh", or "Swap Panels"
- **Mouse wheel**: Scroll up/down to navigate through files

### Keyboard Navigation

- `Up/Down` or `j/k`: Navigate files/folders
- `Left/Right` or `h/l`: Switch between left/right panels
- `Enter`:
  - For folders: expand/collapse
  - For files: compare with vimdiff (if exists on both sides) or open single file with vim
- `PageUp/PageDown` or `Ctrl+B/Ctrl+F`: Half-page scroll (based on terminal height)
- `Ctrl+Home`: Scroll to top
- `Ctrl+End`: Scroll to bottom
- `1`: Show all files
- `2`: Show only different files
- `3`: Show only different files (no orphans)
- `+`: Expand all folders
- `-`: Collapse all folders
- `F5`: Refresh directories
- `s`: Swap panel contents
- `Ctrl+R` / `Ctrl+L`: Copy selected file (left-to-right / right-to-left)
- `q` or `Esc`: Exit

### Screen Layout

```
┌────────── 🛠️  Tools ──────────────────────────────────────────────────────────┐
│ 📁 All Files(1) │ 🔍 Different(2) │ ⚡ Diff Only(3) │ 📂 Expand All(+) │
│ 📁 Collapse All(-) │ 🔄 Refresh(F5) │ 🔃 Swap Panels(s) │ ▶️Copy(Ctrl+R) │
│ Filter: All Files                                                         │
└───────────────────────────────────────────────────────────────────────────────┘
┌─────────── Left: /path/to/dir1 ──────────┐┌─────────── Right: /path/to/dir2 ─────────┐
│ 📁 folder1                   2.5K Mar 15││ 📁 folder1                   2.5K Mar 15│
│   📄 file1.txt               1.2K Mar 10││   📄 file1.txt               1.2K Mar 10│
│   📄 file2.txt               3.4K Mar 12││                                         │
│ 📁 folder2                   5.1K Mar 14││ 📁 folder2                   8.2K Mar 16│
│                                         ││   📁 subfolder               1.8K Mar 16│
│                                         ││     📄 newfile.txt           1.8K Mar 16│
└─────────────────────────────────────────┘└─────────────────────────────────────────┘
```

## File Comparison Algorithm

Uses multi-stage comparison for performance optimization:

1. **File size comparison** (fastest)
2. **Modification time comparison** (1-second tolerance to handle filesystem differences)
3. **Empty file handling** (0-byte files are considered identical)
4. **Small files** (< 4KB): Full content comparison
5. **Medium files** (< 1MB): SHA256 hash comparison
6. **Large files** (≥ 1MB): Compare first 4KB only

**Note**: This algorithm provides excellent performance for large directories while maintaining accuracy for most use cases.

## UI Enhancements

### Interactive Interface

- **Clickable toolbar**: All toolbar buttons are clickable with mouse
- **Enhanced keyboard shortcuts**: Shortcuts displayed with highlighted keys in red (e.g., (1), (2), (3))
- **File information display**: Each file/folder shows size and modification date on the right side
- **Color-coded interface**: Different colors for different file states and UI elements

### Default Sorting

- **Folder-first sorting**: Directories always appear before files
- **Case-insensitive alphabetical**: Files and folders sorted alphabetically ignoring case

## Advanced Features

### Smart Folder Status Detection

Folders inherit status from their children:

- **Red**: If any child file/folder is different
- **Blue**: If folder exists only on one side
- **Gray**: If all children are identical

This recursive status detection helps quickly identify directories with changes.

### Terminal State Management

- **Cursor restoration**: Proper cursor blinking restoration after exit
- **Panic-safe cleanup**: Terminal state is restored even on program crashes
- **Cross-platform**: Works on Linux, macOS, and Windows terminals

### Performance Optimizations

- **Background operations**: Large directory scans run in background with progress bars
- **Memory efficient**: Streams directory contents without loading everything into memory
- **Lazy loading**: Tree nodes are populated on-demand
- **Synchronized scrolling**: Both panels scroll together for easy comparison

### Error Handling

- **Smart editor fallbacks**:
  - File comparison: vimdiff → vim -d → diff (with user wait)
  - Single file viewing: vim → vi → nano → cat (read-only with user wait)
- **Permission handling**: Continues operation even with permission-denied files
- **Network filesystems**: Handles slow/unreliable network mounts gracefully
- **Cross-platform compatibility**: Works even on minimal systems without advanced editors

## Use Cases

### Development Workflow

```bash
# Compare branches
tudiff ./main-branch ./feature-branch

# Before/after refactoring
tudiff ./before-refactor ./after-refactor

# Compare deployed vs local
tudiff ./local-project /mnt/server/deployed-project
```

### System Administration

```bash
# Configuration drift detection
tudiff /etc/nginx /backup/etc/nginx

# Backup verification
tudiff /home/user /backup/home/user

# System state comparison
tudiff /var/log /backup/var/log
```

### Data Migration

```bash
# Verify file transfers
tudiff /source/data /destination/data

# Compare directory structures
tudiff /old-system/files /new-system/files
```

## Troubleshooting

### Common Issues

**Terminal cursor not blinking after exit:**

- This is automatically handled by tudiff's terminal restoration
- If issues persist, run: `tput cnorm`

**Unicode icons not displaying:**

- Ensure your terminal supports Unicode/UTF-8
- Most modern terminals support this by default

**Performance with very large directories:**

- Use the "Different Only" filter (key `2`) to reduce displayed items
- The tool is optimized for directories with 100K+ files

**Permission errors:**

- The tool will continue scanning and mark inaccessible files appropriately
- Run with appropriate permissions for full access

## Dependencies

Key Rust crates:

- `ratatui`: TUI interface framework
- `crossterm`: Cross-platform terminal manipulation
- `clap`: Command line argument parsing
- `walkdir`: Efficient directory traversal
- `similar`: Text difference algorithms
- `sha2`: Cryptographic hash functions
- `anyhow`: Error handling and context

## License

MIT License

## Similar Tools

**tudiff** vs other directory comparison tools:

- **Beyond Compare**: Commercial, GUI-based, more features but not terminal-native
- **diff -r**: Command line, text output only, no interactive navigation
- **meld**: GUI-based, requires X11/desktop environment
- **kdiff3**: GUI-based, heavy dependency requirements

**tudiff** provides the best of both worlds: powerful comparison features in a lightweight, terminal-native interface.
