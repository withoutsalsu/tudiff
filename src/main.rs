use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use tudiff::terminal::{run_tui, simple_compare, ensure_cursor_visible};

#[derive(Parser)]
#[command(name = "tudiff")]
#[command(about = "TUI-based directory and file comparison tool")]
struct Args {
    #[arg(help = "First directory to compare")]
    dir1: Option<PathBuf>,

    #[arg(help = "Second directory to compare")]
    dir2: Option<PathBuf>,

    #[arg(long, help = "Use simple text output instead of TUI")]
    simple: bool,

    #[arg(short, long, help = "Enable verbose logging")]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging based on verbose flag
    tudiff::utils::init_logging(args.verbose);

    let (dir1, dir2) = match (args.dir1, args.dir2) {
        (Some(d1), Some(d2)) => (d1, d2),
        _ => {
            eprintln!("Usage: tudiff <dir1> <dir2>");
            eprintln!("\nCompares two directories in a TUI interface");
            eprintln!("\nNavigation:");
            eprintln!("  Up/Down     - Navigate files");
            eprintln!("  PageUp/Down - Fast scroll (10 lines)");
            eprintln!("  Left/Right  - Switch between panels");
            eprintln!("  Enter       - Toggle folder / Compare file with vimdiff");
            eprintln!("  Esc/q       - Exit");
            std::process::exit(1);
        }
    };

    if !dir1.exists() || !dir1.is_dir() {
        eprintln!("Error: '{}' is not a valid directory", dir1.display());
        std::process::exit(1);
    }

    if !dir2.exists() || !dir2.is_dir() {
        eprintln!("Error: '{}' is not a valid directory", dir2.display());
        std::process::exit(1);
    }

    let result = if args.simple {
        simple_compare(dir1, dir2)
    } else {
        match run_tui(dir1.clone(), dir2.clone()) {
            Ok(_) => Ok(()),
            Err(e) => {
                eprintln!("TUI Error: {}", e);
                eprintln!("Trying to use terminal in forced mode...");
                match std::process::Command::new("tty").output() {
                    Ok(output) => {
                        if output.status.success() {
                            eprintln!("Terminal detected, but TUI failed. Falling back...");
                        } else {
                            eprintln!("No terminal detected. Falling back...");
                        }
                    }
                    Err(_) => eprintln!("Cannot detect terminal. Falling back..."),
                }
                simple_compare(dir1, dir2)
            }
        }
    };

    ensure_cursor_visible();

    result
}

