use crossterm::event::{KeyCode, KeyEvent};

use super::types::{McpPane, McpServerTabState};
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

        // 2. Tool Detail / Config modal
        if self.is_tool_detail_open {
            return match code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter | KeyCode::Char('c') => {
                    Some(Action::McpServerToolDetailClose)
                }
                KeyCode::Down | KeyCode::Char('j') | KeyCode::PageDown => {
                    Some(Action::McpServerToolDetailScrollDown)
                }
                KeyCode::Up | KeyCode::Char('k') | KeyCode::PageUp => {
                    Some(Action::McpServerToolDetailScrollUp)
                }
                _ => None,
            };
        }

        // 3. Tools list modal (legacy popup fallback)
        if self.is_tools_modal_open {
            return match code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('t') => {
                    Some(Action::McpServerCloseTools)
                }
                KeyCode::Down | KeyCode::Char('j') => Some(Action::McpServerToolsNext),
                KeyCode::Up | KeyCode::Char('k') => Some(Action::McpServerToolsPrev),
                KeyCode::Enter | KeyCode::Char('d') | KeyCode::Char('v') => {
                    Some(Action::McpServerToolDetailOpen)
                }
                _ => None,
            };
        }

        // 4. Sync Tools modal
        if self.is_sync_modal_open {
            return match code {
                KeyCode::Esc => Some(Action::McpServerSyncModalClose),
                KeyCode::Down | KeyCode::Tab => Some(Action::McpServerSyncModelNext),
                KeyCode::Up | KeyCode::BackTab => Some(Action::McpServerSyncModelPrev),
                KeyCode::Enter => Some(Action::McpServerSyncConfirm),
                _ => Some(Action::McpServerSyncKey(key)),
            };
        }

        // 5. Add / Edit form modal
        if self.is_form_open {
            return match code {
                KeyCode::Esc => Some(Action::CancelForm),
                KeyCode::Tab => Some(Action::FormNextField),
                KeyCode::BackTab => Some(Action::FormPrevField),
                KeyCode::Enter if self.focused_field == super::types::McpFormField::CancelBtn => {
                    Some(Action::CancelForm)
                }
                KeyCode::Enter if self.focused_field == super::types::McpFormField::SaveBtn => {
                    Some(Action::FormSubmit)
                }
                KeyCode::Enter if self.focused_field == super::types::McpFormField::Name => {
                    Some(Action::FormNextField)
                }
                _ => Some(Action::FormKey(key)),
            };
        }

        // 6. Navigation between Left (Servers) and Right (Tools) Cards
        match code {
            KeyCode::Tab | KeyCode::Left | KeyCode::Right | KeyCode::Char('h') | KeyCode::Char('l') => {
                Some(Action::McpServerTogglePane)
            }
            _ => match self.active_pane {
                McpPane::Servers => match code {
                    KeyCode::Down | KeyCode::Char('j') => Some(Action::McpServerSelectNext),
                    KeyCode::Up | KeyCode::Char('k') => Some(Action::McpServerSelectPrev),
                    KeyCode::Char(' ') | KeyCode::Enter => Some(Action::McpServerToggle),
                    KeyCode::Char('a') | KeyCode::Char('n') => Some(Action::McpServerAdd),
                    KeyCode::Char('e') => Some(Action::McpServerEdit),
                    KeyCode::Char('d') => Some(Action::McpServerDelete),
                    KeyCode::Char('s') | KeyCode::Char('S') => Some(Action::McpServerSyncTools),
                    KeyCode::Char('r') | KeyCode::Char('R') => Some(Action::McpServerRefresh),
                    KeyCode::Char('t') => Some(Action::McpServerTogglePane),
                    _ => None,
                },
                McpPane::Tools => match code {
                    KeyCode::Down | KeyCode::Char('j') => Some(Action::McpServerToolsNext),
                    KeyCode::Up | KeyCode::Char('k') => Some(Action::McpServerToolsPrev),
                    KeyCode::Enter | KeyCode::Char('d') | KeyCode::Char('v') | KeyCode::Char('c') | KeyCode::Char(' ') => {
                        Some(Action::McpServerToolDetailOpen)
                    }
                    KeyCode::Char('s') | KeyCode::Char('S') => Some(Action::McpServerSyncTools),
                    KeyCode::Char('r') | KeyCode::Char('R') => Some(Action::McpServerRefresh),
                    KeyCode::Esc => Some(Action::McpServerTogglePane),
                    _ => None,
                },
            },
        }
    }
}
