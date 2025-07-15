use crate::{
    file_reader::FileReader,
    selection::Selection,
    text_utils::TextUtils,
    constants::Constants,
};
use tui_textarea::TextArea;
use clipboard::ClipboardProvider;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
    Frame,
};

#[derive(Debug, Clone)]
pub struct ContextMenu {
    pub x: u16,
    pub y: u16,
    pub items: Vec<String>,
}

impl ContextMenu {
    pub fn new(x: u16, y: u16) -> Self {
        Self {
            x,
            y,
            items: vec![
                Constants::CONTEXT_MENU_COPY.to_string(),
                Constants::CONTEXT_MENU_SEARCH.to_string(),
            ],
        }
    }
}

pub enum ViewerAction {
    None,
    Quit,
}

pub struct Viewer {
    file_reader: FileReader,
    current_line: usize,
    search_matches: Vec<usize>,
    current_match: usize,
    in_search_mode: bool,
    viewport_height: usize,
    selection: Option<Selection>,
    selecting: bool,
    context_menu: Option<ContextMenu>,
    progress_visible: bool,
    progress_value: f64,
    progress_message: String,
    search_requested: bool,
    search_cancelled: bool,
    last_search_term: String,
    search_textarea: TextArea<'static>,
}

impl Viewer {
    pub fn new(file_reader: FileReader) -> Self {
        let mut search_textarea = TextArea::default();
        search_textarea.set_cursor_line_style(Style::default());
        search_textarea.set_style(Style::default().bg(ratatui::style::Color::Blue));
        search_textarea.set_line_number_style(Style::default());
        search_textarea.remove_line_number();
        
        Self {
            file_reader,
            current_line: 0,
            search_matches: Vec::new(),
            current_match: 0,
            in_search_mode: false,
            viewport_height: Constants::DEFAULT_VIEWPORT_HEIGHT,
            selection: None,
            selecting: false,
            context_menu: None,
            progress_visible: false,
            progress_value: 0.0,
            progress_message: String::new(),
            search_requested: false,
            search_cancelled: false,
            last_search_term: String::new(),
            search_textarea,
        }
    }
    
    pub fn new_empty() -> Self {
        // Create a minimal empty FileReader for progress display
        let empty_file_reader = FileReader::new_empty().unwrap_or_else(|_| {
            // Fallback - this shouldn't happen but just in case
            FileReader::new_with_progress(".", None).unwrap()
        });
        
        let mut search_textarea = TextArea::default();
        search_textarea.set_cursor_line_style(Style::default());
        search_textarea.set_style(Style::default().bg(ratatui::style::Color::Blue));
        search_textarea.set_line_number_style(Style::default());
        search_textarea.remove_line_number();
        
        Self {
            file_reader: empty_file_reader,
            current_line: 0,
            search_matches: Vec::new(),
            current_match: 0,
            in_search_mode: false,
            viewport_height: Constants::DEFAULT_VIEWPORT_HEIGHT,
            selection: None,
            selecting: false,
            context_menu: None,
            progress_visible: false,
            progress_value: 0.0,
            progress_message: String::new(),
            search_requested: false,
            search_cancelled: false,
            last_search_term: String::new(),
            search_textarea,
        }
    }
    
    pub fn draw(&mut self, f: &mut Frame) {
        let chunks = if self.progress_visible {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(0),
                    Constraint::Length(Constants::PROGRESS_BAR_HEIGHT),
                    Constraint::Length(1),
                ])
                .split(f.size())
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0), Constraint::Length(1)])
                .split(f.size())
        };

        self.viewport_height = chunks[0].height as usize;
        
        self.draw_content(f, chunks[0]);
        
        if let Some(ref menu) = self.context_menu {
            self.draw_context_menu(f, menu);
        }
        
        if self.progress_visible {
            self.draw_progress_bar(f, chunks[1]);
            self.draw_status_bar(f, chunks[2]);
        } else {
            self.draw_status_bar(f, chunks[1]);
        }
    }
    
    // State queries
    pub fn is_in_search_mode(&self) -> bool {
        self.in_search_mode
    }
    
    pub fn has_context_menu(&self) -> bool {
        self.context_menu.is_some()
    }
    
    pub fn has_search_requested(&self) -> bool {
        self.search_requested
    }
    
    pub fn request_search(&mut self) {
        self.search_requested = true;
        self.search_cancelled = false;
    }
    
    pub fn clear_search(&mut self) {
        self.search_textarea.delete_line_by_head();
        self.search_textarea.delete_line_by_end();
        self.search_matches.clear();
        self.current_match = 0;
        self.search_requested = false;
        self.search_cancelled = false;
        self.last_search_term.clear();
    }
    
    // Progress bar operations
    pub fn show_progress(&mut self, value: f64, message: &str) {
        self.progress_visible = true;
        self.progress_value = value;
        self.progress_message = message.to_string();
    }
    
    pub fn hide_progress(&mut self) {
        self.progress_visible = false;
        self.progress_value = 0.0;
        self.progress_message.clear();
    }
    
    // Search operations
    pub fn enter_search_mode(&mut self) {
        self.in_search_mode = true;
        self.search_textarea.delete_line_by_head();
        self.search_textarea.delete_line_by_end();
    }
    
    pub fn exit_search_mode(&mut self) {
        self.in_search_mode = false;
    }
    
    pub fn get_search_term(&self) -> String {
        self.search_textarea.lines()[0].clone()
    }
    
    pub fn handle_search_input(&mut self, key: crossterm::event::KeyEvent) -> bool {
        // Handle Ctrl+V for paste
        if key.code == crossterm::event::KeyCode::Char('v') && key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
            if let Ok(mut ctx) = clipboard::ClipboardContext::new() {
                if let Ok(content) = ctx.get_contents() {
                    // Only paste if clipboard contains text (no newlines)
                    if !content.contains('\n') && !content.contains('\r') {
                        self.search_textarea.insert_str(&content);
                        return true;
                    }
                }
            }
        }
        
        // Let TextArea handle all other input
        self.search_textarea.input(key)
    }
    
    pub fn perform_search_with_progress(&mut self) {
        let search_term = self.get_search_term();
        if search_term.is_empty() {
            self.search_matches.clear();
            return;
        }
        
        // Show initial progress
        self.show_progress(0.0, "Searching... (ESC to cancel)");
        
        // Perform search with progress tracking
        self.last_search_term = search_term.clone();
        self.search_matches = self.file_reader.search_with_progress(&search_term, None);
        self.current_match = 0;
        
        if !self.search_matches.is_empty() {
            self.current_line = self.search_matches[0].saturating_sub(self.viewport_height / 2);
        }
        
        self.hide_progress();
    }
    
    pub fn perform_search_with_ui_progress(&mut self, terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>) -> Result<(), std::io::Error> {
        let search_term = self.get_search_term();
        if search_term.is_empty() {
            self.search_matches.clear();
            self.search_requested = false;
            return Ok(());
        }
        
        // Only show progress for searches that might take a while
        let total_lines = self.file_reader.line_count();
        if total_lines > 100_000 { // Show progress for files with more than 100k lines
            // Create a channel for progress updates
            let (progress_tx, progress_rx) = std::sync::mpsc::channel();
            let search_term_for_thread = search_term.clone();
            
            // Create a thread-safe search context
            let search_context = self.file_reader.create_search_context();
            
            let search_thread = std::thread::spawn(move || {
                let progress_callback: crate::file_reader::ProgressCallback = Box::new(move |progress, message| {
                    let _ = progress_tx.send((progress, message.to_string()));
                });
                
                search_context.search_with_progress(&search_term_for_thread, Some(progress_callback))
            });
            
            // Show initial progress
            self.show_progress(0.0, "Searching...");
            
            // Update progress bar while searching
            let mut last_update = std::time::Instant::now();
            loop {
                // Check for keyboard input to allow cancellation
                if crossterm::event::poll(std::time::Duration::from_millis(10))? {
                    if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                        if key.code == crossterm::event::KeyCode::Esc {
                            // Cancel search and return to search mode with preserved text
                            self.search_cancelled = true;
                            self.in_search_mode = true;  // Enter search mode without clearing text
                            self.hide_progress();
                            self.search_requested = false;
                            return Ok(());
                        }
                    }
                }
                
                // Check for progress updates
                match progress_rx.try_recv() {
                    Ok((progress, message)) => {
                        self.show_progress(progress, &message);
                        
                        // Update UI every 200ms to reduce overhead
                        if last_update.elapsed() >= std::time::Duration::from_millis(200) {
                            terminal.draw(|f| self.draw(f))?;
                            last_update = std::time::Instant::now();
                        }
                        
                        if progress >= 1.0 {
                            // Final update
                            terminal.draw(|f| self.draw(f))?;
                            break;
                        }
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        // No progress update, continue
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        // Search thread finished
                        break;
                    }
                }
                
                // Small delay to prevent excessive CPU usage
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            
            // Wait for search thread to complete and get result
            match search_thread.join() {
                Ok(results) => {
                    self.search_matches = results;
                    self.current_match = 0;
                    self.last_search_term = search_term.clone();
                    
                    if !self.search_matches.is_empty() {
                        self.current_line = self.search_matches[0].saturating_sub(self.viewport_height / 2);
                    }
                    
                    self.hide_progress();
                    self.search_requested = false;
                    Ok(())
                }
                Err(_) => {
                    self.hide_progress();
                    self.search_requested = false;
                    Err(std::io::Error::new(std::io::ErrorKind::Other, "Search thread panicked"))
                }
            }
        } else {
            // Small file, search directly without progress
            self.perform_search_with_progress();
            self.search_requested = false;
            Ok(())
        }
    }
    
    pub fn next_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }
        
        self.current_match = (self.current_match + 1) % self.search_matches.len();
        let target_line = self.search_matches[self.current_match];
        self.current_line = target_line.saturating_sub(self.viewport_height / 2);
    }
    
    pub fn prev_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }
        
        self.current_match = if self.current_match == 0 {
            self.search_matches.len() - 1
        } else {
            self.current_match - 1
        };
        
        let target_line = self.search_matches[self.current_match];
        self.current_line = target_line.saturating_sub(self.viewport_height / 2);
    }
    
    // Navigation operations
    pub fn scroll_up(&mut self) {
        self.current_line = self.current_line.saturating_sub(1);
    }
    
    pub fn scroll_down(&mut self) {
        let max_line = self.file_reader.line_count().saturating_sub(self.viewport_height);
        if self.current_line < max_line {
            self.current_line += 1;
        }
    }
    
    pub fn scroll_up_multiple(&mut self, count: usize) {
        for _ in 0..count {
            self.scroll_up();
        }
    }
    
    pub fn scroll_down_multiple(&mut self, count: usize) {
        for _ in 0..count {
            self.scroll_down();
        }
    }
    
    pub fn page_up(&mut self) {
        self.current_line = self.current_line.saturating_sub(self.viewport_height);
    }
    
    pub fn page_down(&mut self) {
        let max_line = self.file_reader.line_count().saturating_sub(self.viewport_height);
        self.current_line = (self.current_line + self.viewport_height).min(max_line);
    }
    
    pub fn goto_start(&mut self) {
        self.current_line = 0;
    }
    
    pub fn goto_end(&mut self) {
        let total_lines = self.file_reader.line_count();
        if total_lines <= self.viewport_height {
            // File fits entirely in viewport, start from beginning
            self.current_line = 0;
        } else {
            // Position so the last line appears at the bottom of viewport
            self.current_line = total_lines - self.viewport_height;
        }
    }
    
    // Selection operations
    pub fn start_selection(&mut self, col: u16, row: u16) {
        self.context_menu = None;
        
        if let Some((line, column)) = self.screen_to_text_coords(col, row) {
            self.selection = Some(Selection::new(line, column));
            self.selecting = true;
        }
    }
    
    pub fn update_selection(&mut self, col: u16, row: u16) {
        if self.selecting {
            if let Some((line, column)) = self.screen_to_text_coords(col, row) {
                if let Some(ref mut selection) = self.selection {
                    selection.update_end(line, column);
                }
            }
        }
    }
    
    pub fn end_selection(&mut self) {
        self.selecting = false;
        
        if let Some(ref selection) = self.selection {
            if selection.is_empty() {
                self.selection = None;
            }
        }
    }
    
    // Context menu operations
    pub fn show_context_menu(&mut self, col: u16, row: u16) {
        if self.selection.is_some() {
            self.context_menu = Some(ContextMenu::new(col, row));
        }
    }
    
    pub fn close_context_menu(&mut self) {
        self.context_menu = None;
    }
    
    pub fn is_mouse_in_menu(&self, col: u16, row: u16) -> bool {
        if let Some(ref menu) = self.context_menu {
            let menu_height = menu.items.len() as u16;
            col >= menu.x && col < menu.x + Constants::CONTEXT_MENU_WIDTH &&
            row >= menu.y && row < menu.y + menu_height
        } else {
            false
        }
    }
    
    pub fn handle_menu_click(&mut self, _col: u16, row: u16) {
        if let Some(ref menu) = self.context_menu {
            let item_index = (row - menu.y) as usize;
            if item_index < menu.items.len() {
                match item_index {
                    0 => self.copy_selection(),
                    1 => self.search_selection(),
                    _ => {}
                }
            }
        }
        self.context_menu = None;
    }
    
    // Utility methods
    fn screen_to_text_coords(&self, col: u16, row: u16) -> Option<(usize, usize)> {
        if row == 0 || col < Constants::LINE_NUMBER_WIDTH {
            return None;
        }
        
        let text_row = (row - 1) as usize;
        let text_col = (col - Constants::LINE_NUMBER_WIDTH) as usize;
        
        if text_row >= self.viewport_height {
            return None;
        }
        
        let line_num = self.current_line + text_row;
        if line_num >= self.file_reader.line_count() {
            return None;
        }
        
        Some((line_num, text_col))
    }
    
    fn copy_selection(&mut self) {
        if let Some(ref selection) = self.selection {
            let _ = selection.copy_to_clipboard(&self.file_reader);
        }
    }
    
    fn search_selection(&mut self) {
        if let Some(ref selection) = self.selection {
            if let Some(text) = selection.get_text(&self.file_reader) {
                if !text.contains('\n') {
                    self.search_textarea.delete_line_by_head();
                    self.search_textarea.delete_line_by_end();
                    self.search_textarea.insert_str(&text);
                    self.request_search();
                }
            }
        }
    }
    
    // Drawing methods
    fn draw_content(&self, f: &mut Frame, area: Rect) {
        let lines = self.file_reader.get_lines(self.current_line, self.viewport_height);
        
        let items: Vec<ListItem> = lines
            .iter()
            .enumerate()
            .map(|(i, line)| {
                let line_num = self.current_line + i;
                let line_number = format!("{:6} ", line_num + 1);
                
                let mut spans = vec![Span::styled(line_number, Style::default().fg(Constants::LINE_NUMBER_COLOR))];
                let text_spans = self.create_line_spans(line, line_num);
                spans.extend(text_spans);
                
                ListItem::new(Line::from(spans))
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("File Viewer"));

        f.render_widget(list, area);
    }
    
    fn create_line_spans<'a>(&self, line: &'a str, line_num: usize) -> Vec<Span<'a>> {
        let mut ranges = Vec::new();
        
        // Add selection highlighting
        if let Some(ref selection) = self.selection {
            if selection.contains_line(line_num) {
                let (start_line, start_col, end_line, end_col) = selection.normalize();
                let sel_start = if line_num == start_line { start_col } else { 0 };
                let sel_end = if line_num == end_line { end_col } else { TextUtils::char_len(line) };
                
                ranges.push((sel_start, sel_end, Style::default()
                    .bg(Constants::SELECTION_BG_COLOR)
                    .fg(Constants::SELECTION_FG_COLOR)));
            }
        }
        
        // Add search highlighting
        let search_term = self.get_search_term();
        if !search_term.is_empty() && line.contains(&search_term) {
            let is_current_match = !self.search_matches.is_empty() && 
                self.search_matches[self.current_match] == line_num;
            
            let mut match_count = 0;
            for (start, part) in line.match_indices(&search_term) {
                let end = start + TextUtils::char_len(part);
                let style = if is_current_match && match_count == 0 {
                    Style::default().bg(Constants::CURRENT_MATCH_BG_COLOR).fg(Constants::CURRENT_MATCH_FG_COLOR)
                } else {
                    Style::default().bg(Constants::OTHER_MATCH_BG_COLOR).fg(Constants::OTHER_MATCH_FG_COLOR)
                };
                
                ranges.push((start, end, style));
                match_count += 1;
            }
        }
        
        if ranges.is_empty() {
            vec![Span::raw(line)]
        } else {
            TextUtils::split_line_into_spans(line, &ranges)
        }
    }
    
    fn draw_progress_bar(&self, f: &mut Frame, area: Rect) {
        let progress = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title("Progress"))
            .gauge_style(Style::default().fg(Constants::PROGRESS_BAR_FG_COLOR).bg(Constants::PROGRESS_BAR_BG_COLOR))
            .ratio(self.progress_value)
            .label(format!("{:.1}% - {}", self.progress_value * 100.0, self.progress_message));
        
        f.render_widget(progress, area);
    }
    
    fn draw_status_bar(&self, f: &mut Frame, area: Rect) {
        if self.in_search_mode {
            // In search mode, show the TextArea for input
            let search_area = Rect {
                x: area.x,
                y: area.y,
                width: area.width,
                height: area.height,
            };
            f.render_widget(self.search_textarea.widget(), search_area);
        } else {
            // Normal mode, show status information
            let total_lines = self.file_reader.line_count();
            let current_pos = self.current_line + 1;
            let match_info = if !self.search_matches.is_empty() {
                format!(" | Match {}/{}", self.current_match + 1, self.search_matches.len())
            } else if !self.last_search_term.is_empty() {
                format!(" | No matches found for '{}'", self.last_search_term)
            } else {
                String::new()
            };
            let esc_hint = if !self.search_matches.is_empty() || !self.last_search_term.is_empty() {
                ", esc: clear search"
            } else {
                ""
            };
            let status = format!("Line {}/{} | q: quit, /: search, n: next match, g: start, G: end{}{}", 
                               current_pos, total_lines, match_info, esc_hint);

            let paragraph = Paragraph::new(status)
                .style(Style::default().bg(Constants::STATUS_BAR_BG_COLOR).fg(Constants::STATUS_BAR_FG_COLOR));
            f.render_widget(paragraph, area);
        }
    }
    
    fn draw_context_menu(&self, f: &mut Frame, menu: &ContextMenu) {
        let menu_height = menu.items.len() as u16;
        
        let screen_width = f.size().width;
        let screen_height = f.size().height;
        
        let menu_x = if menu.x + Constants::CONTEXT_MENU_WIDTH > screen_width {
            screen_width.saturating_sub(Constants::CONTEXT_MENU_WIDTH)
        } else {
            menu.x
        };
        
        let menu_y = if menu.y + menu_height > screen_height {
            screen_height.saturating_sub(menu_height)
        } else {
            menu.y
        };
        
        let menu_area = Rect {
            x: menu_x,
            y: menu_y,
            width: Constants::CONTEXT_MENU_WIDTH,
            height: menu_height,
        };
        
        let background = Block::default()
            .style(Style::default().bg(Constants::CONTEXT_MENU_BG_COLOR));
        f.render_widget(background, menu_area);
        
        for (i, item) in menu.items.iter().enumerate() {
            let item_area = Rect {
                x: menu_x,
                y: menu_y + i as u16,
                width: Constants::CONTEXT_MENU_WIDTH,
                height: 1,
            };
            
            let item_text = format!("{:<width$}", item, width = Constants::CONTEXT_MENU_WIDTH as usize);
            let item_paragraph = Paragraph::new(item_text)
                .style(Style::default().bg(Constants::CONTEXT_MENU_BG_COLOR).fg(Constants::CONTEXT_MENU_FG_COLOR));
            
            f.render_widget(item_paragraph, item_area);
        }
    }
}