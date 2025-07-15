use anyhow::{Context, Result};
use memmap2::Mmap;
use std::fs::File;
use std::path::Path;

pub type ProgressCallback = Box<dyn Fn(f64, &str) + Send + Sync>;

pub struct FileReader {
    mmap: Mmap,
    lines: Vec<usize>, // Line start positions
}

impl FileReader {
    pub fn new_empty() -> Result<Self> {
        // Create a temporary empty file for the empty reader
        let temp_file = std::env::temp_dir().join("bigedit_empty");
        std::fs::write(&temp_file, "")?;
        
        let file = File::open(&temp_file)?;
        let empty_mmap = unsafe { Mmap::map(&file) }
            .map_err(|e| anyhow::anyhow!("Failed to create empty memory map: {}", e))?;
        
        // Clean up temp file
        let _ = std::fs::remove_file(&temp_file);
        
        Ok(FileReader {
            mmap: empty_mmap,
            lines: vec![0],
        })
    }
    
    pub fn new_with_progress<P: AsRef<Path>>(path: P, progress_callback: Option<ProgressCallback>) -> Result<Self> {
        let file = File::open(&path)
            .with_context(|| format!("Failed to open file: {}", path.as_ref().display()))?;
        
        let mmap = unsafe { Mmap::map(&file) }
            .with_context(|| "Failed to memory-map file")?;
        
        // Build line index for fast line-based navigation
        let mut lines = vec![0]; // First line starts at position 0
        let total_bytes = mmap.len();
        let mut last_progress_pos = 0;
        let progress_interval = total_bytes / 20; // Update every 5%
        
        if let Some(ref callback) = progress_callback {
            callback(0.0, "Indexing file...");
        }
        
        for (pos, &byte) in mmap.iter().enumerate() {
            if byte == b'\n' {
                lines.push(pos + 1);
            }
            
            // Report progress every 5% to reduce overhead
            if let Some(ref callback) = progress_callback {
                if pos > last_progress_pos + progress_interval {
                    let progress = pos as f64 / total_bytes as f64;
                    callback(progress, "Indexing file...");
                    last_progress_pos = pos;
                }
            }
        }
        
        if let Some(ref callback) = progress_callback {
            callback(1.0, "File indexing complete");
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
    
    pub fn search_with_progress(&self, needle: &str, progress_callback: Option<ProgressCallback>) -> Vec<usize> {
        let mut matches = Vec::new();
        let total_lines = self.lines.len();
        let mut last_progress_line = 0;
        let progress_interval = total_lines / 20; // Update every 5%
        
        if let Some(ref callback) = progress_callback {
            callback(0.0, "Searching...");
        }
        
        for (line_num, _) in self.lines.iter().enumerate() {
            if let Some(line) = self.get_line(line_num) {
                if line.contains(needle) {
                    matches.push(line_num);
                }
            }
            
            // Report progress every 5% to reduce overhead
            if let Some(ref callback) = progress_callback {
                if line_num > last_progress_line + progress_interval {
                    let progress = line_num as f64 / total_lines as f64;
                    callback(progress, "Searching...");
                    last_progress_line = line_num;
                }
            }
        }
        
        if let Some(ref callback) = progress_callback {
            callback(1.0, "Search complete");
        }
        
        matches
    }
    
}