use crossterm::event::{KeyCode, KeyEvent};

use super::types::CustomLlmTabState;
use crate::app::Action;

impl CustomLlmTabState {
    #[must_use]
    pub fn handle_key(&self, key: KeyEvent) -> Option<Action> {
        let code = key.code;
        if self.delete_confirm_prefix.is_some() {
            return match code {
                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                    Some(Action::CustomLlmConfirmDelete)
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    Some(Action::CustomLlmCancelDelete)
                }
                _ => None,
            };
        }

        if self.is_form_open {
            return match code {
                KeyCode::Esc => Some(Action::CancelForm),
                KeyCode::Tab | KeyCode::Down => Some(Action::FormNextField),
                KeyCode::BackTab | KeyCode::Up => Some(Action::FormPrevField),
                KeyCode::Enter => Some(Action::FormSubmit),
                _ => Some(Action::FormKey(key)),
            };
        }

        match code {
            KeyCode::Tab | KeyCode::BackTab => Some(Action::CustomLlmToggleFocus),
            KeyCode::Left | KeyCode::Char('h')
                if self.focus == super::types::CustomLlmFocus::Models =>
            {
                Some(Action::CustomLlmToggleFocus)
            }
            KeyCode::Right | KeyCode::Char('l')
                if self.focus == super::types::CustomLlmFocus::Providers =>
            {
                Some(Action::CustomLlmToggleFocus)
            }
            KeyCode::Down | KeyCode::Char('j') => Some(Action::FocusNext),
            KeyCode::Up | KeyCode::Char('k') => Some(Action::FocusPrev),
            KeyCode::Enter | KeyCode::Char(' ') => match self.focus {
                super::types::CustomLlmFocus::Providers => Some(Action::CustomLlmToggle),
                super::types::CustomLlmFocus::Models => None,
            },
            KeyCode::Char('a') | KeyCode::Char('n') => Some(Action::CustomLlmAdd),
            KeyCode::Char('e') => Some(Action::CustomLlmEdit),
            KeyCode::Char('d') => Some(Action::CustomLlmDelete),
            KeyCode::Char('r') => Some(Action::CustomLlmRefresh),
            _ => None,
        }
    }
}
