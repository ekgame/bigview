use anyhow::{Context, Result};
use memmap2::Mmap;
use std::fs::File;
use std::path::Path;

pub struct FileReader {
    mmap: Mmap,
    lines: Vec<usize>, // Line start positions
}

impl FileReader {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(&path)
            .with_context(|| format!("Failed to open file: {}", path.as_ref().display()))?;
        
        let mmap = unsafe { Mmap::map(&file) }
            .with_context(|| "Failed to memory-map file")?;
        
        // Build line index for fast line-based navigation
        let mut lines = vec![0]; // First line starts at position 0
        for (pos, &byte) in mmap.iter().enumerate() {
            if byte == b'\n' {
                lines.push(pos + 1);
            }
        }
        
        Ok(FileReader { mmap, lines })
    }
    
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }
    
    pub fn get_line(&self, line_num: usize) -> Option<&str> {
        if line_num >= self.lines.len() {
            return None;
        }
        
        let start = self.lines[line_num];
        let end = if line_num + 1 < self.lines.len() {
            self.lines[line_num + 1].saturating_sub(1) // Exclude newline
        } else {
            self.mmap.len()
        };
        
        if start > end || start >= self.mmap.len() {
            return None;
        }
        
        let line_bytes = &self.mmap[start..end];
        std::str::from_utf8(line_bytes).ok()
    }
    
    pub fn get_lines(&self, start: usize, count: usize) -> Vec<String> {
        let mut result = Vec::new();
        for i in start..start + count {
            if let Some(line) = self.get_line(i) {
                result.push(line.to_string());
            } else {
                break;
            }
        }
        result
    }
    
    pub fn search(&self, needle: &str) -> Vec<usize> {
        let mut matches = Vec::new();
        
        for (line_num, _) in self.lines.iter().enumerate() {
            if let Some(line) = self.get_line(line_num) {
                if line.contains(needle) {
                    matches.push(line_num);
                }
            }
        }
        
        matches
    }
    
}