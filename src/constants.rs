/// Application constants to avoid magic numbers
pub struct Constants;

impl Constants {
    // UI Layout
    pub const LINE_NUMBER_WIDTH: u16 = 7;
    pub const CONTEXT_MENU_WIDTH: u16 = 10;
    pub const SCROLL_LINES_PER_WHEEL: usize = 3;
    
    // Colors and Styles
    pub const LINE_NUMBER_COLOR: ratatui::style::Color = ratatui::style::Color::Yellow;
    pub const SELECTION_BG_COLOR: ratatui::style::Color = ratatui::style::Color::Blue;
    pub const SELECTION_FG_COLOR: ratatui::style::Color = ratatui::style::Color::White;
    pub const CURRENT_MATCH_BG_COLOR: ratatui::style::Color = ratatui::style::Color::Cyan;
    pub const CURRENT_MATCH_FG_COLOR: ratatui::style::Color = ratatui::style::Color::Black;
    pub const OTHER_MATCH_BG_COLOR: ratatui::style::Color = ratatui::style::Color::DarkGray;
    pub const OTHER_MATCH_FG_COLOR: ratatui::style::Color = ratatui::style::Color::White;
    pub const CONTEXT_MENU_BG_COLOR: ratatui::style::Color = ratatui::style::Color::DarkGray;
    pub const CONTEXT_MENU_FG_COLOR: ratatui::style::Color = ratatui::style::Color::White;
    pub const STATUS_BAR_BG_COLOR: ratatui::style::Color = ratatui::style::Color::Blue;
    pub const STATUS_BAR_FG_COLOR: ratatui::style::Color = ratatui::style::Color::White;
    
    // Context Menu
    pub const CONTEXT_MENU_COPY: &'static str = "Copy";
    pub const CONTEXT_MENU_SEARCH: &'static str = "Search";
    
    // Progress Bar
    pub const PROGRESS_BAR_BG_COLOR: ratatui::style::Color = ratatui::style::Color::DarkGray;
    pub const PROGRESS_BAR_FG_COLOR: ratatui::style::Color = ratatui::style::Color::Green;
    pub const PROGRESS_BAR_HEIGHT: u16 = 3;
    
    // Default Values
    pub const DEFAULT_VIEWPORT_HEIGHT: usize = 20;
}