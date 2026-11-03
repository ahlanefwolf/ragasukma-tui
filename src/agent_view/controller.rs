use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crate::agent_view::state::AgentWorkspaceState;
use crate::agent_view::types::AgentFocusArea;
use crate::app::Action;

impl AgentWorkspaceState {
    /// Controller key handler for the Agent workspace view.
    pub fn handle_key(&self, key: KeyEvent) -> Option<Action> {
        let code = key.code;

        // When the input textarea is focused
        if self.is_input_focused {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                return match code {
                    KeyCode::Char('c') => Some(Action::Quit),
                    KeyCode::Char('u') => Some(Action::AgentClearInput),
                    _ => None,
                };
            }
            return match code {
                KeyCode::Esc => Some(Action::AgentUnfocusInput),
                KeyCode::Enter => Some(Action::AgentSubmitPrompt),
                KeyCode::Backspace => Some(Action::AgentBackspace),
                KeyCode::Char(c) => Some(Action::AgentInsertChar(c)),
                _ => None,
            };
        }

        // When browsing panels
        match code {
            KeyCode::Char('i') => Some(Action::AgentFocusInput),
            KeyCode::Char('s') => Some(Action::AgentFocusSessions),
            KeyCode::Char('f') => Some(Action::AgentFocusFiles),
            KeyCode::Char('c') => {
                if self.focus_area == AgentFocusArea::Chat {
                    Some(Action::AgentClearChat)
                } else {
                    Some(Action::AgentFocusChat)
                }
            }
            KeyCode::Char('n') => Some(Action::AgentCreateNewSession),
            KeyCode::Char('x') => Some(Action::AgentToggleToolExpand),
            KeyCode::Char('r') if self.focus_area == AgentFocusArea::Files => {
                Some(Action::AgentScanFiles)
            }

            KeyCode::Down | KeyCode::Char('j') => match self.focus_area {
                AgentFocusArea::Sessions => Some(Action::AgentSelectSessionNext),
                AgentFocusArea::Files => Some(Action::AgentSelectFileNext),
                AgentFocusArea::Chat => Some(Action::AgentScrollDown),
            },
            KeyCode::Up | KeyCode::Char('k') => match self.focus_area {
                AgentFocusArea::Sessions => Some(Action::AgentSelectSessionPrev),
                AgentFocusArea::Files => Some(Action::AgentSelectFilePrev),
                AgentFocusArea::Chat => Some(Action::AgentScrollUp),
            },
            KeyCode::Enter => match self.focus_area {
                AgentFocusArea::Sessions => Some(Action::AgentSwitchSession),
                AgentFocusArea::Files => Some(Action::AgentInsertSelectedFile),
                AgentFocusArea::Chat => Some(Action::AgentFocusInput),
            },
            KeyCode::Char('d') if self.focus_area == AgentFocusArea::Sessions => {
                Some(Action::AgentDeleteSession)
            }
            _ => None,
        }
    }
}
