use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crate::agent_view::state::AgentWorkspaceState;
use crate::agent_view::types::AgentFocusArea;
use crate::app::Action;

impl AgentWorkspaceState {
    /// Controller key handler for the Agent workspace view.
    pub fn handle_key(&self, key: KeyEvent) -> Option<Action> {
        let code = key.code;

        // Modal: Add Folder to Workspace
        if self.add_folder_modal.is_open {
            return match code {
                KeyCode::Esc => Some(Action::AgentCloseAddFolderModal),
                KeyCode::Enter => Some(Action::AgentSubmitAddFolder),
                KeyCode::Backspace => Some(Action::AgentAddFolderBackspace),
                KeyCode::Char(c) => Some(Action::AgentAddFolderChar(c)),
                _ => None,
            };
        }

        // When the input textarea is focused
        if self.is_input_focused {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                return match code {
                    KeyCode::Char('c') => Some(Action::Quit),
                    KeyCode::Char('u') => Some(Action::AgentClearInput),
                    _ => None,
                };
            }

            // Tag query navigation if active
            if self.tag_query.is_some() {
                match code {
                    KeyCode::Esc => return Some(Action::AgentCloseTagQuery),
                    KeyCode::Down => return Some(Action::AgentTagQueryNext),
                    KeyCode::Up => return Some(Action::AgentTagQueryPrev),
                    KeyCode::Tab | KeyCode::Enter => return Some(Action::AgentApplyTag),
                    _ => {}
                }
            }

            return match code {
                KeyCode::Esc => Some(Action::AgentUnfocusInput),
                KeyCode::Enter => Some(Action::AgentSubmitPrompt),
                KeyCode::Backspace => Some(Action::AgentBackspace),
                KeyCode::Tab => Some(Action::AgentFocusNextPanel),
                KeyCode::BackTab => Some(Action::AgentFocusPrevPanel),
                KeyCode::Char(c) => Some(Action::AgentInsertChar(c)),
                _ => None,
            };
        }

        // When browsing panels
        match code {
            KeyCode::Char('i') => Some(Action::AgentFocusInput),
            KeyCode::Tab => Some(Action::AgentFocusNextPanel),
            KeyCode::BackTab => Some(Action::AgentFocusPrevPanel),
            KeyCode::Char('f') => Some(Action::AgentFocusFiles),
            KeyCode::Char('c') => {
                if self.focus_area == AgentFocusArea::Chat {
                    Some(Action::AgentClearChat)
                } else {
                    Some(Action::AgentFocusChat)
                }
            }
            KeyCode::Char('b') => Some(Action::AgentToggleSidebar),
            KeyCode::Char('m') => Some(Action::AgentCycleModel),
            KeyCode::Char('x') => Some(Action::AgentToggleToolExpand),
            KeyCode::Char('r') => match self.focus_area {
                AgentFocusArea::Files => Some(Action::AgentScanFiles),
                _ => Some(Action::AgentToggleReasoning),
            },
            KeyCode::Char('p') => Some(Action::AgentToggleSidebar),
            KeyCode::Char('t') => Some(Action::AgentCycleToolFilter),
            KeyCode::Char('y') => Some(Action::AgentApprovePending),
            KeyCode::Char('n') => Some(Action::AgentRejectPending),

            KeyCode::Down | KeyCode::Char('j') => match self.focus_area {
                AgentFocusArea::Files => Some(Action::AgentSelectFileNext),
                AgentFocusArea::Chat => Some(Action::AgentScrollDown),
                AgentFocusArea::Inspector => Some(Action::AgentNextContextFile),
            },
            KeyCode::Up | KeyCode::Char('k') => match self.focus_area {
                AgentFocusArea::Files => Some(Action::AgentSelectFilePrev),
                AgentFocusArea::Chat => Some(Action::AgentScrollUp),
                AgentFocusArea::Inspector => Some(Action::AgentPrevContextFile),
            },
            KeyCode::Enter => match self.focus_area {
                AgentFocusArea::Files => Some(Action::AgentToggleFileTreeExpand),
                AgentFocusArea::Chat => Some(Action::AgentFocusInput),
                AgentFocusArea::Inspector => Some(Action::AgentTogglePinSelectedFile),
            },
            KeyCode::Char('+') | KeyCode::Char('a') if self.focus_area == AgentFocusArea::Files => {
                Some(Action::AgentOpenAddFolderModal)
            }
            KeyCode::Char('-') | KeyCode::Char('d') if self.focus_area == AgentFocusArea::Files => {
                Some(Action::AgentRemoveActiveFolder)
            }
            KeyCode::Char('[') if self.focus_area == AgentFocusArea::Files => {
                Some(Action::AgentPrevFolder)
            }
            KeyCode::Char(']') if self.focus_area == AgentFocusArea::Files => {
                Some(Action::AgentNextFolder)
            }
            KeyCode::Char(' ') if self.focus_area == AgentFocusArea::Files => {
                Some(Action::AgentInsertSelectedFile)
            }
            KeyCode::Char('d') if self.focus_area == AgentFocusArea::Inspector => {
                Some(Action::AgentRemoveSelectedContextFile)
            }
            _ => None,
        }
    }
}
