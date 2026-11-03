use crossterm::event::{KeyCode, KeyEvent};

use crate::app::Action;
use super::types::CustomLlmTabState;

impl CustomLlmTabState {
    #[must_use]
    pub fn handle_key(&self, key: KeyEvent) -> Option<Action> {
        let code = key.code;
        if self.is_form_open {
            return match code {
                KeyCode::Esc => Some(Action::CancelForm),
                KeyCode::Tab | KeyCode::Down => Some(Action::FormNextField),
                KeyCode::BackTab | KeyCode::Up => Some(Action::FormPrevField),
                KeyCode::Enter => Some(Action::FormSubmit),
                KeyCode::Backspace => Some(Action::FormBackspace),
                KeyCode::Char(c) => Some(Action::FormChar(c)),
                _ => None,
            };
        }

        match code {
            KeyCode::Down | KeyCode::Char('j') => Some(Action::FocusNext),
            KeyCode::Up | KeyCode::Char('k') => Some(Action::FocusPrev),
            KeyCode::Enter | KeyCode::Char(' ') => Some(Action::CustomLlmToggle),
            KeyCode::Char('a') | KeyCode::Char('n') => Some(Action::CustomLlmAdd),
            KeyCode::Char('e') => Some(Action::CustomLlmEdit),
            KeyCode::Char('d') => Some(Action::ConfirmDelete),
            _ => None,
        }
    }
}
