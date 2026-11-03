use crossterm::event::{KeyCode, KeyEvent};
use crate::app::Action;
use super::types::ProviderTabState;

impl ProviderTabState {
    pub fn handle_key(&self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Char('a') | KeyCode::Char('n') | KeyCode::Char('o') => {
                if self.provider_name.contains("Antigravity") {
                    Some(Action::StartAntigravityOAuth)
                } else if self.provider_name.contains("Codex") {
                    Some(Action::StartCodexOAuth)
                } else {
                    None
                }
            }
            KeyCode::Down | KeyCode::Char('j') => Some(Action::FocusNext),
            KeyCode::Up | KeyCode::Char('k') => Some(Action::FocusPrev),
            KeyCode::Char('t') | KeyCode::Char(' ') => Some(Action::ToggleAccountActive),
            KeyCode::Enter => Some(Action::SetAccountActive),
            KeyCode::Char('d') | KeyCode::Delete => Some(Action::ConfirmDelete),
            KeyCode::Char('r') | KeyCode::Char('R') => Some(Action::RefreshQuota),
            _ => None,
        }
    }
}
