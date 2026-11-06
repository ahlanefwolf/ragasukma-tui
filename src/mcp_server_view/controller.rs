use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::types::McpServerTabState;
use crate::app::Action;

impl McpServerTabState {
    #[must_use]
    pub fn handle_key(&self, key: KeyEvent) -> Option<Action> {
        let code = key.code;

        // 1. Delete confirmation modal
        if self.delete_confirm_id.is_some() {
            return match code {
                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                    Some(Action::McpServerConfirmDelete)
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    Some(Action::McpServerCancelDelete)
                }
                _ => None,
            };
        }

        // 2. Add / Edit form modal
        if self.is_form_open {
            return match code {
                KeyCode::Esc => Some(Action::CancelForm),
                KeyCode::Tab => Some(Action::FormNextField),
                KeyCode::BackTab => Some(Action::FormPrevField),
                KeyCode::Left => Some(Action::FormCursorLeft),
                KeyCode::Right => Some(Action::FormCursorRight),
                KeyCode::Up => Some(Action::FormCursorUp),
                KeyCode::Down => Some(Action::FormCursorDown),
                KeyCode::Home => Some(Action::FormCursorHome),
                KeyCode::End => Some(Action::FormCursorEnd),
                KeyCode::Delete => Some(Action::FormDelete),
                KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    Some(Action::FormCursorHome)
                }
                KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    Some(Action::FormCursorEnd)
                }
                KeyCode::Enter if self.focused_field == super::types::McpFormField::CancelBtn => {
                    Some(Action::CancelForm)
                }
                KeyCode::Enter if self.focused_field == super::types::McpFormField::JsonConfig => {
                    Some(Action::FormNewline)
                }
                KeyCode::Enter => Some(Action::FormSubmit),
                KeyCode::Backspace => Some(Action::FormBackspace),
                KeyCode::Char(c)
                    if !key.modifiers.contains(KeyModifiers::CONTROL)
                        && !key.modifiers.contains(KeyModifiers::ALT) =>
                {
                    Some(Action::FormChar(c))
                }
                _ => None,
            };
        }

        // 3. MCP server list navigation
        match code {
            KeyCode::Down | KeyCode::Char('j') => Some(Action::McpServerSelectNext),
            KeyCode::Up | KeyCode::Char('k') => Some(Action::McpServerSelectPrev),
            KeyCode::Char(' ') | KeyCode::Enter => Some(Action::McpServerToggle),
            KeyCode::Char('a') | KeyCode::Char('n') => Some(Action::McpServerAdd),
            KeyCode::Char('e') => Some(Action::McpServerEdit),
            KeyCode::Char('d') => Some(Action::McpServerDelete),
            KeyCode::Char('r') => Some(Action::McpServerRefresh),
            _ => None,
        }
    }
}
