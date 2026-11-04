use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crate::app::Action;
use super::types::{ProviderFocusArea, ProviderTabState};

impl ProviderTabState {
    pub fn handle_key(&self, key: KeyEvent) -> Option<Action> {
        // Global in provider view: Ctrl+j / Ctrl+k to scroll quota at any time
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                    return Some(Action::QuotaScrollDown);
                }
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                    return Some(Action::QuotaScrollUp);
                }
                _ => {}
            }
        }

        match self.focus_area {
            ProviderFocusArea::Accounts => match key.code {
                KeyCode::Tab => Some(Action::ProviderToggleFocus),
                KeyCode::Char('a') => {
                    if self.provider_name.contains("Antigravity") {
                        Some(Action::StartAntigravityOAuth)
                    } else if self.provider_name.contains("Codex") {
                        Some(Action::StartCodexOAuth)
                    } else if self.provider_name.contains("Claude") {
                        Some(Action::StartClaudeOAuth)
                    } else {
                        None
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => Some(Action::FocusNext),
                KeyCode::Up | KeyCode::Char('k') => Some(Action::FocusPrev),
                KeyCode::Char(' ') => Some(Action::ToggleAccountActive),
                KeyCode::Enter => Some(Action::SetAccountActive),
                KeyCode::Char('d') => Some(Action::ConfirmDelete),
                KeyCode::Char('r') | KeyCode::Char('R') => Some(Action::RefreshQuota),
                _ => None,
            },
            ProviderFocusArea::Quotas => match key.code {
                KeyCode::Tab | KeyCode::BackTab => Some(Action::ProviderToggleFocus),
                KeyCode::Down | KeyCode::Char('j') | KeyCode::PageDown => {
                    Some(Action::QuotaScrollDown)
                }
                KeyCode::Up | KeyCode::Char('k') | KeyCode::PageUp => {
                    Some(Action::QuotaScrollUp)
                }
                KeyCode::Char('r') | KeyCode::Char('R') => Some(Action::RefreshQuota),
                _ => None,
            },
        }
    }
}
