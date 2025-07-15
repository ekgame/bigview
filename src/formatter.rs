use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub struct FileFormatter;

impl FileFormatter {
    /// Determine if a file needs formatting based on extension
    pub fn needs_formatting(file_path: &str) -> bool {
        let path = Path::new(file_path);
        if let Some(extension) = path.extension() {
            match extension.to_str() {
                Some("json") | Some("xml") => true,
                _ => false,
            }
        } else {
            false
        }
    }
    
    /// Create a formatted copy of the file if needed, returns the path to open
    pub fn format_if_needed(file_path: &str) -> Result<String> {
        if !Self::needs_formatting(file_path) {
            return Ok(file_path.to_string());
        }
        
        let path = Path::new(file_path);
        let extension = path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");
        
        // Create formatted filename
        let formatted_path = Self::create_formatted_path(path)?;
        
        // Read original file
        let content = fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read file: {}", file_path))?;
        
        // Format based on file type
        let formatted_content = match extension {
            "json" => Self::format_json(&content)?,
            "xml" => Self::format_xml(&content)?,
            _ => content, // Fallback, should not happen
        };
        
        // Write formatted file
        fs::write(&formatted_path, formatted_content)
            .with_context(|| format!("Failed to write formatted file: {}", formatted_path.display()))?;
        
        Ok(formatted_path.to_string_lossy().to_string())
    }
    
    fn create_formatted_path(original_path: &Path) -> Result<PathBuf> {
        let parent = original_path.parent()
            .ok_or_else(|| anyhow::anyhow!("Cannot determine parent directory"))?;
        
        let stem = original_path.file_stem()
            .ok_or_else(|| anyhow::anyhow!("Cannot determine file stem"))?;
        
        let extension = original_path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");
        
        let formatted_name = format!("{}_formatted.{}", stem.to_string_lossy(), extension);
        Ok(parent.join(formatted_name))
    }
    
    fn format_json(content: &str) -> Result<String> {
        let value: serde_json::Value = serde_json::from_str(content)
            .with_context(|| "Failed to parse JSON")?;
        
        serde_json::to_string_pretty(&value)
            .with_context(|| "Failed to format JSON")
    }
    
    fn format_xml(content: &str) -> Result<String> {
        use quick_xml::events::Event;
        use quick_xml::reader::Reader;
        use quick_xml::writer::Writer;
        use std::io::Cursor;
        
        let mut reader = Reader::from_str(content);
        reader.trim_text(true);
        
        let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 4);
        let mut buf = Vec::new();
        
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Eof) => break,
                Ok(event) => {
                    writer.write_event(event)
                        .with_context(|| "Failed to write XML event")?;
                }
                Err(e) => return Err(anyhow::anyhow!("XML parsing error: {}", e)),
            }
            buf.clear();
        }
        
        let result = writer.into_inner().into_inner();
        String::from_utf8(result)
            .with_context(|| "Failed to convert XML to string")
    }
}