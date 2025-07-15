use crossterm::event::{Event, KeyCode, KeyModifiers, MouseButton, MouseEventKind};
use clipboard::{ClipboardContext, ClipboardProvider};
use crate::viewer::{Viewer, ViewerAction};

pub struct EventHandler;

impl EventHandler {
    pub fn handle_event(viewer: &mut Viewer, event: Event) -> ViewerAction {
        match event {
            Event::Key(key) => Self::handle_key_event(viewer, key),
            Event::Mouse(mouse) => Self::handle_mouse_event(viewer, mouse),
            _ => ViewerAction::None,
        }
    }
    
    fn handle_key_event(viewer: &mut Viewer, key: crossterm::event::KeyEvent) -> ViewerAction {
        if viewer.is_in_search_mode() {
            Self::handle_search_key(viewer, key)
        } else if viewer.has_context_menu() {
            Self::handle_context_menu_key(viewer, key)
        } else {
            Self::handle_normal_key(viewer, key)
        }
    }
    
    fn handle_search_key(viewer: &mut Viewer, key: crossterm::event::KeyEvent) -> ViewerAction {
        match key.code {
            KeyCode::Esc => {
                viewer.exit_search_mode();
                ViewerAction::None
            }
            KeyCode::Enter => {
                viewer.request_search();
                viewer.exit_search_mode();
                ViewerAction::None
            }
            KeyCode::Backspace => {
                viewer.backspace_search_term();
                ViewerAction::None
            }
            KeyCode::Char('v') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Self::paste_clipboard_to_search(viewer);
                ViewerAction::None
            }
            KeyCode::Char(c) => {
                viewer.add_to_search_term(c);
                ViewerAction::None
            }
            _ => ViewerAction::None,
        }
    }
    
    fn handle_normal_key(viewer: &mut Viewer, key: crossterm::event::KeyEvent) -> ViewerAction {
        match key.code {
            KeyCode::Char('q') => ViewerAction::Quit,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => ViewerAction::Quit,
            KeyCode::Esc => {
                viewer.clear_search();
                ViewerAction::None
            }
            KeyCode::Char('/') => {
                viewer.enter_search_mode();
                ViewerAction::None
            }
            KeyCode::Char('n') => {
                viewer.next_match();
                ViewerAction::None
            }
            KeyCode::Char('N') => {
                viewer.prev_match();
                ViewerAction::None
            }
            KeyCode::Up => {
                viewer.scroll_up();
                ViewerAction::None
            }
            KeyCode::Down => {
                viewer.scroll_down();
                ViewerAction::None
            }
            KeyCode::PageUp => {
                viewer.page_up();
                ViewerAction::None
            }
            KeyCode::PageDown => {
                viewer.page_down();
                ViewerAction::None
            }
            KeyCode::Home => {
                viewer.goto_start();
                ViewerAction::None
            }
            KeyCode::End => {
                viewer.goto_end();
                ViewerAction::None
            }
            _ => ViewerAction::None,
        }
    }
    
    fn handle_context_menu_key(viewer: &mut Viewer, key: crossterm::event::KeyEvent) -> ViewerAction {
        match key.code {
            KeyCode::Esc => {
                viewer.close_context_menu();
                ViewerAction::None
            }
            _ => ViewerAction::None,
        }
    }
    
    fn handle_mouse_event(viewer: &mut Viewer, mouse: crossterm::event::MouseEvent) -> ViewerAction {
        if viewer.has_context_menu() {
            Self::handle_context_menu_mouse(viewer, mouse)
        } else {
            Self::handle_normal_mouse(viewer, mouse)
        }
    }
    
    fn handle_normal_mouse(viewer: &mut Viewer, mouse: crossterm::event::MouseEvent) -> ViewerAction {
        match mouse.kind {
            MouseEventKind::ScrollUp => {
                viewer.scroll_up_multiple(crate::constants::Constants::SCROLL_LINES_PER_WHEEL);
                ViewerAction::None
            }
            MouseEventKind::ScrollDown => {
                viewer.scroll_down_multiple(crate::constants::Constants::SCROLL_LINES_PER_WHEEL);
                ViewerAction::None
            }
            MouseEventKind::Down(MouseButton::Left) => {
                viewer.start_selection(mouse.column, mouse.row);
                ViewerAction::None
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                viewer.update_selection(mouse.column, mouse.row);
                ViewerAction::None
            }
            MouseEventKind::Up(MouseButton::Left) => {
                viewer.end_selection();
                ViewerAction::None
            }
            MouseEventKind::Down(MouseButton::Right) => {
                viewer.show_context_menu(mouse.column, mouse.row);
                ViewerAction::None
            }
            _ => ViewerAction::None,
        }
    }
    
    fn handle_context_menu_mouse(viewer: &mut Viewer, mouse: crossterm::event::MouseEvent) -> ViewerAction {
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if viewer.is_mouse_in_menu(mouse.column, mouse.row) {
                    viewer.handle_menu_click(mouse.column, mouse.row);
                } else {
                    viewer.close_context_menu();
                }
                ViewerAction::None
            }
            _ => ViewerAction::None,
        }
    }
    
    fn paste_clipboard_to_search(viewer: &mut Viewer) {
        if let Ok(mut ctx) = ClipboardContext::new() {
            if let Ok(content) = ctx.get_contents() {
                if !content.contains('\n') && !content.contains('\r') {
                    viewer.add_to_search_term_str(&content);
                }
            }
        }
    }
}