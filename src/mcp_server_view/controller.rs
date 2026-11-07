use crossterm::event::{KeyCode, KeyEvent};

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

        // 2. Tools list modal
        if self.is_tools_modal_open {
            return match code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('t') => {
                    Some(Action::McpServerCloseTools)
                }
                KeyCode::Down | KeyCode::Char('j') => Some(Action::McpServerToolsNext),
                KeyCode::Up | KeyCode::Char('k') => Some(Action::McpServerToolsPrev),
                _ => None,
            };
        }

        // 3. Sync Tools modal
        if self.is_sync_modal_open {
            return match code {
                KeyCode::Esc => Some(Action::McpServerSyncModalClose),
                KeyCode::Down | KeyCode::Tab => Some(Action::McpServerSyncModelNext),
                KeyCode::Up | KeyCode::BackTab => Some(Action::McpServerSyncModelPrev),
                KeyCode::Enter => Some(Action::McpServerSyncConfirm),
                _ => Some(Action::McpServerSyncKey(key)),
            };
        }

        // 3. Add / Edit form modal
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

        // 3. MCP server list navigation
        match code {
            KeyCode::Down | KeyCode::Char('j') => Some(Action::McpServerSelectNext),
            KeyCode::Up | KeyCode::Char('k') => Some(Action::McpServerSelectPrev),
            KeyCode::Char(' ') | KeyCode::Enter => Some(Action::McpServerToggle),
            KeyCode::Char('a') | KeyCode::Char('n') => Some(Action::McpServerAdd),
            KeyCode::Char('e') => Some(Action::McpServerEdit),
            KeyCode::Char('d') => Some(Action::McpServerDelete),
            KeyCode::Char('t') | KeyCode::Char('T') => Some(Action::McpServerViewTools),
            KeyCode::Char('s') | KeyCode::Char('S') => Some(Action::McpServerSyncTools),
            KeyCode::Char('r') => Some(Action::McpServerRefresh),
            _ => None,
        }
    }
}
