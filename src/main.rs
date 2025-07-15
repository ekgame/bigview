mod file_reader;
mod event_handler;
mod text_utils;
mod selection;
mod constants;
mod viewer;

use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use file_reader::{FileReader, ProgressCallback};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use viewer::{Viewer, ViewerAction};
use event_handler::EventHandler;

#[derive(Parser)]
#[command(name = "bigedit")]
#[command(about = "A fast file viewer for large text files")]
struct Args {
    /// Path to the file to view
    file_path: String,
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    // Try to setup terminal, but if it fails, just load the file without UI
    let terminal_setup = enable_raw_mode().and_then(|_| {
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        Terminal::new(backend)
    });
    
    match terminal_setup {
        Ok(mut terminal) => {
            // Run with UI
            let result = load_file_with_progress(&args.file_path, &mut terminal);
            
            // Restore terminal
            disable_raw_mode()?;
            execute!(
                terminal.backend_mut(),
                LeaveAlternateScreen,
                DisableMouseCapture
            )?;
            terminal.show_cursor()?;
            
            result?;
        }
        Err(_) => {
            // Terminal not available, show error message
            eprintln!("Error: Terminal not available. This application requires a terminal to run.");
            std::process::exit(1);
        }
    }
    
    Ok(())
}

fn load_file_with_progress(file_path: &str, terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> Result<()> {
    // For large files, show progress. For small files, load directly.
    let file_size = std::fs::metadata(file_path)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    
    // Show progress bar only for files larger than 10MB
    if file_size > 10 * 1024 * 1024 {
        // Create a channel for progress updates
        let (progress_tx, progress_rx) = mpsc::channel();
        let file_path_clone = file_path.to_string();
        
        // Create a minimal viewer to show progress (no file loaded yet)
        let mut progress_viewer = Viewer::new_empty();
        progress_viewer.show_progress(0.0, "Loading file...");
        
        // Spawn file loading thread
        let loading_thread = thread::spawn(move || {
            let progress_callback: ProgressCallback = Box::new(move |progress, message| {
                let _ = progress_tx.send((progress, message.to_string()));
            });
            
            FileReader::new_with_progress(&file_path_clone, Some(progress_callback))
        });
        
        // Update progress bar while file is loading
        let mut last_update = std::time::Instant::now();
        loop {
            // Check for progress updates
            match progress_rx.try_recv() {
                Ok((progress, message)) => {
                    progress_viewer.show_progress(progress, &message);
                    
                    // Only update UI every 200ms to reduce overhead
                    if last_update.elapsed() >= Duration::from_millis(200) {
                        terminal.draw(|f| progress_viewer.draw(f))?;
                        last_update = std::time::Instant::now();
                    }
                    
                    if progress >= 1.0 {
                        // Final update
                        terminal.draw(|f| progress_viewer.draw(f))?;
                        break;
                    }
                }
                Err(mpsc::TryRecvError::Empty) => {
                    // No progress update, continue
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    // Loading thread finished
                    break;
                }
            }
            
            // Longer delay to reduce CPU usage
            thread::sleep(Duration::from_millis(100));
        }
        
        // Wait for loading thread to complete and get result
        let file_reader = loading_thread.join().map_err(|_| anyhow::anyhow!("Loading thread panicked"))??;
        
        // Create the real viewer
        let mut viewer = Viewer::new(file_reader);
        
        // Run the viewer
        Ok(run_viewer(&mut viewer, terminal)?)
    } else {
        // Small file, load directly without progress bar
        let file_reader = FileReader::new_with_progress(file_path, None)?;
        let mut viewer = Viewer::new(file_reader);
        
        // Run the viewer
        Ok(run_viewer(&mut viewer, terminal)?)
    }
}

fn run_viewer(viewer: &mut Viewer, terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> io::Result<()> {
    loop {
        terminal.draw(|f| viewer.draw(f))?;
        
        // Check if a search was requested
        if viewer.has_search_requested() {
            if let Err(e) = viewer.perform_search_with_ui_progress(terminal) {
                eprintln!("Search error: {}", e);
            }
        }
        
        // Check for events with a timeout to allow search processing
        if crossterm::event::poll(Duration::from_millis(100))? {
            let event = crossterm::event::read()?;
            match EventHandler::handle_event(viewer, event) {
                ViewerAction::Quit => break,
                ViewerAction::None => continue,
            }
        }
    }
    Ok(())
}
