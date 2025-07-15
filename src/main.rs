mod file_reader;
mod viewer;

use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use file_reader::FileReader;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use viewer::Viewer;

#[derive(Parser)]
#[command(name = "bigedit")]
#[command(about = "A fast file viewer for large text files")]
struct Args {
    /// Path to the file to view
    file_path: String,
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    let file_reader = FileReader::new(&args.file_path)?;
    let mut viewer = Viewer::new(file_reader);
    
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    // Run the viewer
    let result = viewer.run(&mut terminal);
    
    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    
    result?;
    Ok(())
}
