use crate::file_reader::FileReader;
use crossterm::event::{self, Event, KeyCode, KeyModifiers, MouseButton, MouseEventKind};
use clipboard::{ClipboardContext, ClipboardProvider};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use std::io;

#[derive(Debug, Clone)]
struct Selection {
    start_line: usize,
    start_col: usize,
    end_line: usize,
    end_col: usize,
}

#[derive(Debug, Clone)]
struct ContextMenu {
    x: u16,
    y: u16,
    items: Vec<String>,
}

pub struct Viewer {
    file_reader: FileReader,
    current_line: usize,
    search_term: String,
    search_matches: Vec<usize>,
    current_match: usize,
    in_search_mode: bool,
    viewport_height: usize,
    selection: Option<Selection>,
    selecting: bool,
    context_menu: Option<ContextMenu>,
}

impl Viewer {
    pub fn new(file_reader: FileReader) -> Self {
        Self {
            file_reader,
            current_line: 0,
            search_term: String::new(),
            search_matches: Vec::new(),
            current_match: 0,
            in_search_mode: false,
            viewport_height: 20,
            selection: None,
            selecting: false,
            context_menu: None,
        }
    }

    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
        loop {
            terminal.draw(|f| self.draw(f))?;

            match event::read()? {
                Event::Key(key) => {
                    match (self.in_search_mode, key.code) {
                        (true, KeyCode::Esc) => {
                            self.in_search_mode = false;
                            self.search_term.clear();
                        }
                        (true, KeyCode::Enter) => {
                            self.perform_search();
                            self.in_search_mode = false;
                        }
                        (true, KeyCode::Backspace) => {
                            self.search_term.pop();
                        }
                        (true, KeyCode::Char('v')) if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            if let Ok(mut ctx) = ClipboardContext::new() {
                                if let Ok(clipboard_content) = ctx.get_contents() {
                                    // Only paste if clipboard contains text (no newlines)
                                    if !clipboard_content.contains('\n') && !clipboard_content.contains('\r') {
                                        self.search_term.push_str(&clipboard_content);
                                    }
                                }
                            }
                        }
                        (true, KeyCode::Char(c)) => {
                            self.search_term.push(c);
                        }
                        (false, KeyCode::Char('q')) => break,
                        (_, KeyCode::Char('c')) if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                        (false, KeyCode::Char('/')) => {
                            self.in_search_mode = true;
                            self.search_term.clear();
                        }
                        (false, KeyCode::Char('n')) => {
                            self.next_match();
                        }
                        (false, KeyCode::Char('N')) => {
                            self.prev_match();
                        }
                        (false, KeyCode::Up) => {
                            self.scroll_up();
                        }
                        (false, KeyCode::Down) => {
                            self.scroll_down();
                        }
                        (false, KeyCode::PageUp) => {
                            self.page_up();
                        }
                        (false, KeyCode::PageDown) => {
                            self.page_down();
                        }
                        (false, KeyCode::Home) => {
                            self.current_line = 0;
                        }
                        (false, KeyCode::End) => {
                            self.current_line = self.file_reader.line_count().saturating_sub(1);
                        }
                        _ => {}
                    }
                }
                Event::Mouse(mouse) => {
                    if !self.in_search_mode && self.context_menu.is_none() {
                        match mouse.kind {
                            MouseEventKind::ScrollUp => {
                                for _ in 0..3 {
                                    self.scroll_up();
                                }
                            }
                            MouseEventKind::ScrollDown => {
                                for _ in 0..3 {
                                    self.scroll_down();
                                }
                            }
                            MouseEventKind::Down(MouseButton::Left) => {
                                self.handle_mouse_down(mouse.column, mouse.row);
                            }
                            MouseEventKind::Drag(MouseButton::Left) => {
                                self.handle_mouse_drag(mouse.column, mouse.row);
                            }
                            MouseEventKind::Up(MouseButton::Left) => {
                                self.handle_mouse_up(mouse.column, mouse.row);
                            }
                            MouseEventKind::Down(MouseButton::Right) => {
                                self.handle_right_click(mouse.column, mouse.row);
                            }
                            _ => {}
                        }
                    } else if let Some(ref mut _menu) = self.context_menu {
                        match mouse.kind {
                            MouseEventKind::Down(MouseButton::Left) => {
                                if self.is_mouse_in_menu(mouse.column, mouse.row) {
                                    self.handle_menu_click(mouse.column, mouse.row);
                                } else {
                                    self.context_menu = None;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn draw(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(f.size());

        self.viewport_height = chunks[0].height as usize;
        
        // Main content area
        self.draw_content(f, chunks[0]);
        
        // Context menu (if visible)
        if let Some(ref menu) = self.context_menu {
            self.draw_context_menu(f, menu);
        }
        
        // Status bar
        self.draw_status_bar(f, chunks[1]);
    }

    fn draw_content(&self, f: &mut Frame, area: Rect) {
        let lines = self.file_reader.get_lines(self.current_line, self.viewport_height);
        
        let items: Vec<ListItem> = lines
            .iter()
            .enumerate()
            .map(|(i, line)| {
                let line_num = self.current_line + i;
                let line_number = format!("{:6} ", line_num + 1);
                
                let mut line_spans = vec![Span::styled(line_number, Style::default().fg(Color::Yellow))];
                
                // Apply selection highlighting first, then search highlighting
                let text_spans = if self.selection.is_some() {
                    self.apply_selection_highlighting(line, line_num)
                } else {
                    vec![Span::raw(line)]
                };
                
                let final_spans = if !self.search_term.is_empty() && line.contains(&self.search_term) {
                    self.apply_search_highlighting_to_spans(text_spans, &self.search_term, line_num)
                } else {
                    text_spans
                };
                
                line_spans.extend(final_spans);
                ListItem::new(Line::from(line_spans))
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("File Viewer"));

        f.render_widget(list, area);
    }

    fn draw_status_bar(&self, f: &mut Frame, area: Rect) {
        let status = if self.in_search_mode {
            format!("Search: {}", self.search_term)
        } else {
            let total_lines = self.file_reader.line_count();
            let current_pos = self.current_line + 1;
            let match_info = if !self.search_matches.is_empty() {
                format!(" | Match {}/{}", self.current_match + 1, self.search_matches.len())
            } else {
                String::new()
            };
            format!("Line {}/{} | q: quit, /: search, n: next match{}", 
                   current_pos, total_lines, match_info)
        };

        let paragraph = Paragraph::new(status)
            .style(Style::default().bg(Color::Blue).fg(Color::White));
        f.render_widget(paragraph, area);
    }

    fn perform_search(&mut self) {
        if self.search_term.is_empty() {
            self.search_matches.clear();
            return;
        }
        
        self.search_matches = self.file_reader.search(&self.search_term);
        self.current_match = 0;
        
        if !self.search_matches.is_empty() {
            self.current_line = self.search_matches[0].saturating_sub(self.viewport_height / 2);
        }
    }

    fn next_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }
        
        self.current_match = (self.current_match + 1) % self.search_matches.len();
        let target_line = self.search_matches[self.current_match];
        self.current_line = target_line.saturating_sub(self.viewport_height / 2);
    }

    fn prev_match(&mut self) {
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

    fn scroll_up(&mut self) {
        self.current_line = self.current_line.saturating_sub(1);
    }

    fn scroll_down(&mut self) {
        let max_line = self.file_reader.line_count().saturating_sub(self.viewport_height);
        if self.current_line < max_line {
            self.current_line += 1;
        }
    }

    fn page_up(&mut self) {
        self.current_line = self.current_line.saturating_sub(self.viewport_height);
    }

    fn page_down(&mut self) {
        let max_line = self.file_reader.line_count().saturating_sub(self.viewport_height);
        self.current_line = (self.current_line + self.viewport_height).min(max_line);
    }

    fn handle_mouse_down(&mut self, col: u16, row: u16) {
        // Clear any existing context menu
        self.context_menu = None;
        
        // Convert screen coordinates to line/column
        if let Some((line, column)) = self.screen_to_text_coords(col, row) {
            self.selection = Some(Selection {
                start_line: line,
                start_col: column,
                end_line: line,
                end_col: column,
            });
            self.selecting = true;
        }
    }

    fn handle_mouse_drag(&mut self, col: u16, row: u16) {
        if self.selecting {
            if let Some((line, column)) = self.screen_to_text_coords(col, row) {
                if let Some(ref mut selection) = self.selection {
                    selection.end_line = line;
                    selection.end_col = column;
                }
            }
        }
    }

    fn handle_mouse_up(&mut self, _col: u16, _row: u16) {
        self.selecting = false;
        
        // Clear selection if it's empty (same start and end position)
        if let Some(ref selection) = self.selection {
            if selection.start_line == selection.end_line && selection.start_col == selection.end_col {
                self.selection = None;
            }
        }
    }

    fn handle_right_click(&mut self, col: u16, row: u16) {
        // Only show context menu if we have a selection
        if self.selection.is_some() {
            self.context_menu = Some(ContextMenu {
                x: col,
                y: row,
                items: vec![
                    "Copy".to_string(),
                    "Search".to_string(),
                ],
            });
        }
    }

    fn screen_to_text_coords(&self, col: u16, row: u16) -> Option<(usize, usize)> {
        // Account for borders and line numbers
        if row == 0 || col < 7 {
            return None; // Header or line number area
        }
        
        let text_row = (row - 1) as usize;
        let text_col = (col - 7) as usize; // Account for line number width
        
        if text_row >= self.viewport_height {
            return None;
        }
        
        let line_num = self.current_line + text_row;
        if line_num >= self.file_reader.line_count() {
            return None;
        }
        
        Some((line_num, text_col))
    }

    fn is_mouse_in_menu(&self, col: u16, row: u16) -> bool {
        if let Some(ref menu) = self.context_menu {
            let menu_width = 10;
            let menu_height = menu.items.len() as u16; // No borders
            
            col >= menu.x && col < menu.x + menu_width &&
            row >= menu.y && row < menu.y + menu_height
        } else {
            false
        }
    }

    fn handle_menu_click(&mut self, _col: u16, row: u16) {
        if let Some(ref menu) = self.context_menu {
            // No border, so direct calculation
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

    fn copy_selection(&mut self) {
        if let Some(selected_text) = self.get_selected_text() {
            if let Ok(mut ctx) = ClipboardContext::new() {
                let _ = ctx.set_contents(selected_text);
            }
        }
    }

    fn search_selection(&mut self) {
        if let Some(selected_text) = self.get_selected_text() {
            // Only search if it's a single line
            if !selected_text.contains('\n') {
                self.search_term = selected_text;
                self.perform_search();
            }
        }
    }

    fn get_selected_text(&self) -> Option<String> {
        if let Some(ref selection) = self.selection {
            let mut result = String::new();
            
            let (start_line, start_col, end_line, end_col) = self.normalize_selection(selection);
            
            if start_line == end_line {
                // Single line selection
                if let Some(line) = self.file_reader.get_line(start_line) {
                    let chars: Vec<char> = line.chars().collect();
                    let char_len = chars.len();
                    let safe_start = start_col.min(char_len);
                    let safe_end = end_col.min(char_len);
                    
                    if safe_start < char_len && safe_start < safe_end {
                        result = chars[safe_start..safe_end].iter().collect();
                    }
                }
            } else {
                // Multi-line selection
                for line_num in start_line..=end_line {
                    if let Some(line) = self.file_reader.get_line(line_num) {
                        let chars: Vec<char> = line.chars().collect();
                        let char_len = chars.len();
                        
                        if line_num == start_line {
                            let safe_start = start_col.min(char_len);
                            if safe_start < char_len {
                                let text: String = chars[safe_start..].iter().collect();
                                result.push_str(&text);
                            }
                        } else if line_num == end_line {
                            let safe_end = end_col.min(char_len);
                            if safe_end > 0 {
                                let text: String = chars[..safe_end].iter().collect();
                                result.push_str(&text);
                            }
                        } else {
                            result.push_str(line);
                        }
                        
                        if line_num != end_line {
                            result.push('\n');
                        }
                    }
                }
            }
            
            if result.is_empty() {
                None
            } else {
                Some(result)
            }
        } else {
            None
        }
    }

    fn normalize_selection(&self, selection: &Selection) -> (usize, usize, usize, usize) {
        let (start_line, start_col, end_line, end_col) = if selection.start_line < selection.end_line ||
            (selection.start_line == selection.end_line && selection.start_col <= selection.end_col) {
            (selection.start_line, selection.start_col, selection.end_line, selection.end_col)
        } else {
            (selection.end_line, selection.end_col, selection.start_line, selection.start_col)
        };
        
        (start_line, start_col, end_line, end_col)
    }

    fn apply_selection_highlighting<'a>(&self, line: &'a str, line_num: usize) -> Vec<Span<'a>> {
        if let Some(ref selection) = self.selection {
            let (start_line, start_col, end_line, end_col) = self.normalize_selection(selection);
            
            if line_num >= start_line && line_num <= end_line {
                let mut result = Vec::new();
                let chars: Vec<char> = line.chars().collect();
                let char_len = chars.len();
                
                let sel_start = if line_num == start_line { start_col.min(char_len) } else { 0 };
                let sel_end = if line_num == end_line { end_col.min(char_len) } else { char_len };
                
                // Ensure valid range
                if sel_start > sel_end {
                    return vec![Span::raw(line)];
                }
                
                // Before selection
                if sel_start > 0 {
                    let before_text: String = chars[0..sel_start].iter().collect();
                    result.push(Span::raw(before_text));
                }
                
                // Selection
                if sel_start < char_len && sel_start < sel_end {
                    let selected_text: String = chars[sel_start..sel_end].iter().collect();
                    result.push(Span::styled(
                        selected_text,
                        Style::default().bg(Color::Blue).fg(Color::White)
                    ));
                }
                
                // After selection
                if sel_end < char_len {
                    let after_text: String = chars[sel_end..].iter().collect();
                    result.push(Span::raw(after_text));
                }
                
                result
            } else {
                vec![Span::raw(line)]
            }
        } else {
            vec![Span::raw(line)]
        }
    }

    fn apply_search_highlighting_to_spans<'a>(&self, spans: Vec<Span<'a>>, term: &str, line_num: usize) -> Vec<Span<'a>> {
        let mut result = Vec::new();
        
        let is_current_match_line = !self.search_matches.is_empty() && 
            self.search_matches[self.current_match] == line_num;
        
        for span in spans {
            if span.content.contains(term) {
                let content_str = span.content.to_string();
                let mut last_end = 0;
                let mut match_count = 0;
                
                for (start, part) in content_str.match_indices(term) {
                    if start > last_end {
                        result.push(Span::styled(content_str[last_end..start].to_string(), span.style));
                    }
                    
                    let search_style = if is_current_match_line && match_count == 0 {
                        Style::default().bg(Color::Cyan).fg(Color::Black)
                    } else {
                        Style::default().bg(Color::DarkGray).fg(Color::White)
                    };
                    
                    result.push(Span::styled(part.to_string(), search_style));
                    last_end = start + part.len();
                    match_count += 1;
                }
                
                if last_end < content_str.len() {
                    result.push(Span::styled(content_str[last_end..].to_string(), span.style));
                }
            } else {
                result.push(span);
            }
        }
        
        result
    }

    fn draw_context_menu(&self, f: &mut Frame, menu: &ContextMenu) {
        let menu_width = 10;
        let menu_height = menu.items.len() as u16;
        
        // Ensure menu fits within screen bounds
        let screen_width = f.size().width;
        let screen_height = f.size().height;
        
        let menu_x = if menu.x + menu_width > screen_width {
            screen_width.saturating_sub(menu_width)
        } else {
            menu.x
        };
        
        let menu_y = if menu.y + menu_height > screen_height {
            screen_height.saturating_sub(menu_height)
        } else {
            menu.y
        };
        
        // First, draw a solid background rectangle to cover any underlying text
        let menu_area = Rect {
            x: menu_x,
            y: menu_y,
            width: menu_width,
            height: menu_height,
        };
        
        // Clear the background with a solid color
        let background = Block::default()
            .style(Style::default().bg(Color::DarkGray));
        f.render_widget(background, menu_area);
        
        // Then draw each menu item on top
        for (i, item) in menu.items.iter().enumerate() {
            let item_area = Rect {
                x: menu_x,
                y: menu_y + i as u16,
                width: menu_width,
                height: 1,
            };
            
            // Create a fully filled background for each item
            let item_text = format!("{:<width$}", item, width = menu_width as usize);
            let item_paragraph = Paragraph::new(item_text)
                .style(Style::default().bg(Color::DarkGray).fg(Color::White));
            
            f.render_widget(item_paragraph, item_area);
        }
    }
}