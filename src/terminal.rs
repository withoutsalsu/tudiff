use anyhow::Result;
use crossterm::event::{self, Event};
use ratatui::{backend::Backend, Terminal};
use std::path::Path;
use std::time::Duration;

use crate::app::App;
use crate::compare::{DirectoryComparison, FileStatus};
use crate::ui::draw_ui;

#[derive(Clone)]
pub struct TerminalState;

impl TerminalState {
    pub fn save() -> Result<Self> {
        Ok(Self)
    }

    pub fn restore(&self) -> Result<()> {
        crossterm::terminal::disable_raw_mode().ok();

        crossterm::execute!(std::io::stdout(), crossterm::cursor::Show)?;

        std::process::Command::new("tput")
            .arg("cnorm")
            .status()
            .ok();

        print!("\x1b[?25h");
        print!("\x1b[?12h");
        use std::io::Write;
        std::io::stdout().flush().ok();

        Ok(())
    }
}

pub struct TerminalManager {
    original_state: TerminalState,
}

impl TerminalManager {
    pub fn new() -> Result<Self> {
        let original_state = TerminalState::save()?;

        let restore_state = original_state.clone();
        std::panic::set_hook(Box::new(move |panic_info| {
            let _ = crossterm::terminal::disable_raw_mode();
            let _ = crossterm::execute!(
                std::io::stdout(),
                crossterm::terminal::LeaveAlternateScreen,
                crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
                crossterm::event::DisableMouseCapture
            );
            let _ = restore_state.restore();

            println!("{}", panic_info);
        }));

        crossterm::terminal::enable_raw_mode()?;
        let mut stdout = std::io::stdout();
        crossterm::execute!(
            stdout,
            crossterm::terminal::EnterAlternateScreen,
            crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
            crossterm::cursor::Hide,
            crossterm::event::EnableMouseCapture
        )?;

        print!("\x1b[?12l");
        use std::io::Write;
        stdout.flush()?;

        Ok(Self { original_state })
    }

    pub fn restore(self) -> Result<()> {
        crossterm::terminal::disable_raw_mode()?;
        crossterm::execute!(
            std::io::stdout(),
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
            crossterm::event::DisableMouseCapture
        )?;

        self.original_state.restore()?;

        let _ = std::panic::take_hook();

        Ok(())
    }
}

pub fn run_tui(dir1: std::path::PathBuf, dir2: std::path::PathBuf) -> Result<()> {
    let comparison = DirectoryComparison::new(dir1, dir2)?;
    let _terminal_manager = TerminalManager::new()?;

    let backend = ratatui::backend::CrosstermBackend::new(std::io::stdout());
    let mut terminal = ratatui::Terminal::new(backend)?;

    terminal.clear()?;

    let result = run_app(&mut terminal, comparison);

    _terminal_manager.restore()?;
    ensure_cursor_visible();

    result
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, comparison: DirectoryComparison) -> Result<()> {
    let mut app = App::new(comparison);
    let mut need_redraw = true;

    loop {
        app.check_refresh_progress();

        if need_redraw {
            terminal.clear()?;
            need_redraw = false;
        }

        draw_ui(terminal, &mut app)?;

        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) => {
                    if app.handle_key_event(key)? {
                        return Ok(());
                    }

                    if let crossterm::event::KeyCode::Enter = key.code {
                        if key.kind == crossterm::event::KeyEventKind::Press {
                            need_redraw = true;
                        }
                    }
                }
                Event::Mouse(mouse) => {
                    app.handle_mouse_event(mouse);
                }
                _ => {}
            }
        }
    }
}

pub fn launch_external_editor(status: &FileStatus, left_path: &Path, right_path: &Path) -> Result<()> {
    let _ = crossterm::terminal::disable_raw_mode();
    let _ = crossterm::execute!(
        std::io::stdout(),
        crossterm::terminal::LeaveAlternateScreen
    );

    print!("\x1b[2J\x1b[H");
    use std::io::Write;
    let _ = std::io::stdout().flush();

    match status {
        FileStatus::LeftOnly => {
            if left_path.exists() {
                let editors = ["vim", "vi", "nano"];
                let mut success = false;

                for editor in &editors {
                    let result = std::process::Command::new(editor)
                        .arg(left_path)
                        .status();
                    if result.is_ok() {
                        success = true;
                        break;
                    }
                }

                if !success {
                    eprintln!("No editor found. Displaying file content with cat...\n");
                    let _ = std::process::Command::new("cat")
                        .arg(left_path)
                        .status();
                    eprintln!("\n\nPress Enter to continue...");
                    let _ = std::io::stdin().read_line(&mut String::new());
                }
            }
        }
        FileStatus::RightOnly => {
            if right_path.exists() {
                let editors = ["vim", "vi", "nano"];
                let mut success = false;

                for editor in &editors {
                    let result = std::process::Command::new(editor)
                        .arg(right_path)
                        .status();
                    if result.is_ok() {
                        success = true;
                        break;
                    }
                }

                if !success {
                    eprintln!("No editor found. Displaying file content with cat...\n");
                    let _ = std::process::Command::new("cat")
                        .arg(right_path)
                        .status();
                    eprintln!("\n\nPress Enter to continue...");
                    let _ = std::io::stdin().read_line(&mut String::new());
                }
            }
        }
        _ => {
            let mut success = false;

            let result = std::process::Command::new("vimdiff")
                .arg(left_path)
                .arg(right_path)
                .status();

            if result.is_ok() {
                success = true;
            } else {
                let result = std::process::Command::new("vim")
                    .arg("-d")
                    .arg(left_path)
                    .arg(right_path)
                    .status();

                if result.is_ok() {
                    success = true;
                }
            }

            if !success {
                eprintln!("No visual diff tool found. Using diff command...\n");
                let _ = std::process::Command::new("diff")
                    .arg("-u")
                    .arg("--color=always")
                    .arg(left_path)
                    .arg(right_path)
                    .status();
                eprintln!("\n\nPress Enter to continue...");
                let _ = std::io::stdin().read_line(&mut String::new());
            }
        }
    }

    std::thread::sleep(Duration::from_millis(200));

    if let Err(e) = crossterm::terminal::enable_raw_mode() {
        eprintln!("Failed to enable raw mode: {}", e);
    }
    if let Err(e) = crossterm::execute!(
        std::io::stdout(),
        crossterm::terminal::EnterAlternateScreen
    ) {
        eprintln!("Failed to enter alternate screen: {}", e);
    }

    if let Err(e) = crossterm::execute!(
        std::io::stdout(),
        crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
        crossterm::terminal::Clear(crossterm::terminal::ClearType::Purge),
        crossterm::cursor::MoveTo(0, 0),
        crossterm::cursor::Hide
    ) {
        eprintln!("Failed to clear terminal: {}", e);
    }

    print!("\x1b[?12l");
    let _ = std::io::stdout().flush();

    Ok(())
}

pub fn ensure_cursor_visible() {
    let _ = crossterm::execute!(std::io::stdout(), crossterm::cursor::Show);
    let _ = std::process::Command::new("tput").arg("cnorm").status();
    print!("\x1b[?25h");
    print!("\x1b[?12h");
    use std::io::Write;
    let _ = std::io::stdout().flush();
}

pub fn simple_compare(dir1: std::path::PathBuf, dir2: std::path::PathBuf) -> Result<()> {
    let comparison = DirectoryComparison::new(dir1, dir2)?;

    crossterm::execute!(std::io::stdout(), crossterm::cursor::Show).ok();

    println!("Directory Comparison Results:");
    println!("Left:  {}", comparison.left_dir.display());
    println!("Right: {}", comparison.right_dir.display());
    println!();

    fn print_tree(node: &crate::compare::FileNode, depth: usize) {
        let indent = "  ".repeat(depth);

        if node.name.is_empty() {
            println!("{}", indent);
        } else {
            let icon = if node.is_dir { "📁" } else { "📄" };
            let status_char = match node.status {
                FileStatus::Same => "=",
                FileStatus::Different => "≠",
                FileStatus::LeftOnly => "L",
                FileStatus::RightOnly => "R",
            };

            println!("{}{} {} [{}]", indent, icon, node.name, status_char);
        }

        if node.is_dir && !node.name.is_empty() {
            for child in &node.children {
                print_tree(child, depth + 1);
            }
        }
    }

    println!("Legend: [=] Same, [≠] Different, [L] Left only, [R] Right only");
    println!();

    println!("=== LEFT PANEL ===");
    print_tree(&comparison.left_tree, 0);
    println!();

    println!("=== RIGHT PANEL ===");
    print_tree(&comparison.right_tree, 0);

    Ok(())
}