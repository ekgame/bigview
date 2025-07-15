use clipboard::{ClipboardContext, ClipboardProvider};
use crate::{file_reader::FileReader, text_utils::TextUtils};

#[derive(Debug, Clone)]
pub struct Selection {
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
}

impl Selection {
    pub fn new(line: usize, col: usize) -> Self {
        Self {
            start_line: line,
            start_col: col,
            end_line: line,
            end_col: col,
        }
    }
    
    pub fn update_end(&mut self, line: usize, col: usize) {
        self.end_line = line;
        self.end_col = col;
    }
    
    pub fn is_empty(&self) -> bool {
        self.start_line == self.end_line && self.start_col == self.end_col
    }
    
    pub fn normalize(&self) -> (usize, usize, usize, usize) {
        if self.start_line < self.end_line || 
           (self.start_line == self.end_line && self.start_col <= self.end_col) {
            (self.start_line, self.start_col, self.end_line, self.end_col)
        } else {
            (self.end_line, self.end_col, self.start_line, self.start_col)
        }
    }
    
    pub fn contains_line(&self, line: usize) -> bool {
        let (start_line, _, end_line, _) = self.normalize();
        line >= start_line && line <= end_line
    }
    
    pub fn get_text(&self, file_reader: &FileReader) -> Option<String> {
        let (start_line, start_col, end_line, end_col) = self.normalize();
        let mut result = String::new();
        
        if start_line == end_line {
            // Single line selection
            if let Some(line) = file_reader.get_line(start_line) {
                result = TextUtils::safe_substring(line, start_col, end_col);
            }
        } else {
            // Multi-line selection
            for line_num in start_line..=end_line {
                if let Some(line) = file_reader.get_line(line_num) {
                    if line_num == start_line {
                        let text = TextUtils::safe_substring(line, start_col, TextUtils::char_len(line));
                        result.push_str(&text);
                    } else if line_num == end_line {
                        let text = TextUtils::safe_substring(line, 0, end_col);
                        result.push_str(&text);
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
    }
    
    pub fn copy_to_clipboard(&self, file_reader: &FileReader) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(text) = self.get_text(file_reader) {
            let mut ctx = ClipboardContext::new()?;
            ctx.set_contents(text)?;
        }
        Ok(())
    }
}