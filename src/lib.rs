//! retcon - Retroactive Continuity CLI for editing git history
//!
//! This crate provides a terminal user interface for editing git commit metadata,
//! including author/committer information, dates, and commit messages.

pub mod app;
pub mod error;
pub mod git;
pub mod state;
pub mod ui;

pub use app::App;
pub use error::{HistError, Result};
pub use git::Repository;

use clap::Parser;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::{self, stdout};
use std::panic;
use std::path::PathBuf;

/// Command-line arguments for retcon.
#[derive(Parser, Debug)]
#[command(name = "retcon")]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the git repository (default: current directory)
    #[arg(short, long)]
    path: Option<PathBuf>,

    /// Maximum number of commits to load
    #[arg(short = 'n', long, default_value = "50")]
    limit: usize,

    /// Skip validation checks (dangerous!)
    #[arg(long, hide = true)]
    force: bool,

    /// Keep author and committer fields separate (by default, editing author
    /// fields also updates the corresponding committer fields)
    #[arg(long, short = 's')]
    separate_author_committer: bool,
}

/// Main entry point for the retcon application.
///
/// This function is called by both `retcon` and `ret` binaries.
pub fn main() {
    // Set up panic hook to restore terminal on panic
    setup_panic_hook();

    // Parse arguments
    let args = Args::parse();

    // Run the app
    if let Err(e) = run(args) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run(args: Args) -> Result<()> {
    // Open repository
    let repo = match &args.path {
        Some(path) => Repository::open(path)?,
        None => Repository::open_current_dir()?,
    };

    // Create app
    // When separate_author_committer is true, we DON'T want to sync (sync = false)
    let sync_author_to_committer = !args.separate_author_committer;
    let mut app = App::new(repo, args.limit, sync_author_to_committer)?;

    // Set up terminal
    let mut terminal = setup_terminal()?;

    // Run the app
    let result = app.run(&mut terminal);

    // Restore terminal
    restore_terminal(&mut terminal)?;

    result
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode().map_err(|e| HistError::Terminal(e.to_string()))?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
        .map_err(|e| HistError::Terminal(e.to_string()))?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend).map_err(|e| HistError::Terminal(e.to_string()))
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode().map_err(|e| HistError::Terminal(e.to_string()))?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .map_err(|e| HistError::Terminal(e.to_string()))?;
    terminal
        .show_cursor()
        .map_err(|e| HistError::Terminal(e.to_string()))?;
    Ok(())
}

fn setup_panic_hook() {
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        // Restore terminal first
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture);

        // Then call original hook to print panic info
        original_hook(panic_info);
    }));
}
