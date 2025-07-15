use crate::file_reader::FileReader;
use crossterm::event::{self, Event, KeyCode, KeyModifiers, MouseEventKind};
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

pub struct Viewer {
    file_reader: FileReader,
    current_line: usize,
    search_term: String,
    search_matches: Vec<usize>,
    current_match: usize,
    in_search_mode: bool,
    viewport_height: usize,
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
                    if !self.in_search_mode {
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
                
                if !self.search_term.is_empty() && line.contains(&self.search_term) {
                    let highlighted_spans = self.highlight_search_term(line, &self.search_term, line_num);
                    let mut line_spans = vec![Span::styled(line_number, Style::default().fg(Color::Yellow))];
                    line_spans.extend(highlighted_spans);
                    ListItem::new(Line::from(line_spans))
                } else {
                    ListItem::new(Line::from(vec![
                        Span::styled(line_number, Style::default().fg(Color::Yellow)),
                        Span::raw(line),
                    ]))
                }
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

    fn highlight_search_term<'a>(&self, line: &'a str, term: &str, line_num: usize) -> Vec<Span<'a>> {
        if term.is_empty() {
            return vec![Span::raw(line)];
        }

        let mut result = Vec::new();
        let mut last_end = 0;
        
        // Check if this line contains the current match
        let is_current_match_line = !self.search_matches.is_empty() && 
            self.search_matches[self.current_match] == line_num;
        
        let mut match_count = 0;
        for (start, part) in line.match_indices(term) {
            if start > last_end {
                result.push(Span::raw(&line[last_end..start]));
            }
            
            // For the current match line, only highlight the first occurrence in cyan
            // All other matches (including other occurrences on the same line) are more muted
            let style = if is_current_match_line && match_count == 0 {
                Style::default().bg(Color::Cyan).fg(Color::Black)
            } else {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            };
            
            result.push(Span::styled(part, style));
            last_end = start + part.len();
            match_count += 1;
        }
        
        if last_end < line.len() {
            result.push(Span::raw(&line[last_end..]));
        }
        
        result
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
}