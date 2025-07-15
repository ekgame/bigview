/// Utilities for safe text manipulation with proper UTF-8 handling
use ratatui::{style::Style, text::Span};

pub struct TextUtils;

impl TextUtils {
    /// Safely extract a substring using character indices instead of byte indices
    pub fn safe_substring(text: &str, start: usize, end: usize) -> String {
        let chars: Vec<char> = text.chars().collect();
        let char_len = chars.len();
        
        let safe_start = start.min(char_len);
        let safe_end = end.min(char_len);
        
        if safe_start >= safe_end {
            return String::new();
        }
        
        chars[safe_start..safe_end].iter().collect()
    }
    
    /// Split a line into spans based on character ranges
    pub fn split_line_into_spans<'a>(line: &'a str, ranges: &[(usize, usize, Style)]) -> Vec<Span<'a>> {
        let chars: Vec<char> = line.chars().collect();
        let char_len = chars.len();
        let mut spans = Vec::new();
        let mut last_end = 0;
        
        for &(start, end, style) in ranges {
            let safe_start = start.min(char_len);
            let safe_end = end.min(char_len);
            
            // Add text before this range
            if safe_start > last_end {
                let text: String = chars[last_end..safe_start].iter().collect();
                spans.push(Span::raw(text));
            }
            
            // Add the styled range
            if safe_start < safe_end {
                let text: String = chars[safe_start..safe_end].iter().collect();
                spans.push(Span::styled(text, style));
            }
            
            last_end = safe_end;
        }
        
        // Add remaining text
        if last_end < char_len {
            let text: String = chars[last_end..].iter().collect();
            spans.push(Span::raw(text));
        }
        
        spans
    }
    
    /// Get the character length of a string (not byte length)
    pub fn char_len(text: &str) -> usize {
        text.chars().count()
    }
    
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_safe_substring() {
        let text = "Hello, 世界!";
        assert_eq!(TextUtils::safe_substring(text, 0, 5), "Hello");
        assert_eq!(TextUtils::safe_substring(text, 7, 9), "世界");
        assert_eq!(TextUtils::safe_substring(text, 0, 100), text);
        assert_eq!(TextUtils::safe_substring(text, 100, 200), "");
    }
    
    #[test]
    fn test_char_len() {
        assert_eq!(TextUtils::char_len("Hello"), 5);
        assert_eq!(TextUtils::char_len("世界"), 2);
        assert_eq!(TextUtils::char_len(""), 0);
    }
}