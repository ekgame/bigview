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
    
    /// Split a line into spans based on character ranges, handling overlapping ranges
    pub fn split_line_into_spans<'a>(line: &'a str, ranges: &[(usize, usize, Style)]) -> Vec<Span<'a>> {
        let chars: Vec<char> = line.chars().collect();
        let char_len = chars.len();
        let mut spans = Vec::new();
        
        if ranges.is_empty() {
            return vec![Span::raw(line)];
        }
        
        // Sort ranges by start position
        let mut sorted_ranges = ranges.to_vec();
        sorted_ranges.sort_by(|a, b| a.0.cmp(&b.0));
        
        // Create a priority map where later styles have higher priority
        let mut style_map: Vec<(usize, usize, Style)> = Vec::new();
        
        // Process ranges to handle overlaps by giving priority to later ranges
        for &(start, end, style) in &sorted_ranges {
            let safe_start = start.min(char_len);
            let safe_end = end.min(char_len);
            
            if safe_start < safe_end {
                style_map.push((safe_start, safe_end, style));
            }
        }
        
        // Create segments with their styles
        let mut segments: Vec<(usize, usize, Option<Style>)> = Vec::new();
        let mut events: Vec<(usize, bool, Style)> = Vec::new(); // (position, is_start, style)
        
        // Create start/end events
        for &(start, end, style) in &style_map {
            events.push((start, true, style));
            events.push((end, false, style));
        }
        
        // Sort events by position, with end events before start events at same position
        events.sort_by(|a, b| {
            match a.0.cmp(&b.0) {
                std::cmp::Ordering::Equal => a.1.cmp(&b.1), // false (end) before true (start)
                other => other,
            }
        });
        
        let mut active_styles: Vec<Style> = Vec::new();
        let mut last_pos = 0;
        
        for (pos, is_start, style) in events {
            // Add segment before this event
            if pos > last_pos {
                let current_style = active_styles.last().copied();
                segments.push((last_pos, pos, current_style));
            }
            
            if is_start {
                active_styles.push(style);
            } else {
                active_styles.retain(|&s| s != style);
            }
            
            last_pos = pos;
        }
        
        // Add final segment
        if last_pos < char_len {
            segments.push((last_pos, char_len, None));
        }
        
        // Convert segments to spans
        for (start, end, style_opt) in segments {
            if start < end {
                let text: String = chars[start..end].iter().collect();
                if let Some(style) = style_opt {
                    spans.push(Span::styled(text, style));
                } else {
                    spans.push(Span::raw(text));
                }
            }
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