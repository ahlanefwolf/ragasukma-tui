use super::types::McpServerTabState;
use crate::app::types::Action;

/// Handles actions related to the MCP Server tab.
/// Returns Some(Option<status_message>) if the action was recognized and handled,
/// or None if it does not belong to the MCP Server tab.
pub fn handle_mcp_server_action(
    state: &mut McpServerTabState,
    action: &Action,
) -> Option<Option<String>> {
    match action {
        Action::FormChar(c) if state.is_form_open => {
            state.form_input_char(*c);
            Some(None)
        }
        Action::FormNewline if state.is_form_open => {
            state.insert_newline();
            Some(None)
        }
        Action::FormDelete if state.is_form_open => {
            state.delete_forward();
            Some(None)
        }
        Action::FormCursorLeft if state.is_form_open => {
            state.cursor_left();
            Some(None)
        }
        Action::FormCursorRight if state.is_form_open => {
            state.cursor_right();
            Some(None)
        }
        Action::FormCursorUp if state.is_form_open => {
            state.cursor_up();
            Some(None)
        }
        Action::FormCursorDown if state.is_form_open => {
            state.cursor_down();
            Some(None)
        }
        Action::FormCursorHome if state.is_form_open => {
            state.cursor_home();
            Some(None)
        }
        Action::FormCursorEnd if state.is_form_open => {
            state.cursor_end();
            Some(None)
        }
        Action::FormPaste(text) if state.is_form_open => {
            state.form_paste(text);
            Some(None)
        }
        Action::FormBackspace if state.is_form_open => {
            state.form_backspace();
            Some(None)
        }
        Action::FormNextField if state.is_form_open => {
            state.form_next_field();
            Some(None)
        }
        Action::FormPrevField if state.is_form_open => {
            state.form_prev_field();
            Some(None)
        }
        Action::FormSubmit => {
            if state.is_form_open {
                match state.save_form() {
                    Ok(msg) => {
                        tracing::info!("MCP server saved: {msg}");
                        Some(Some(msg))
                    }
                    Err(err) => {
                        tracing::warn!("MCP form submit error: {err}");
                        Some(Some(format!("Failed: {err}")))
                    }
                }
            } else {
                None
            }
        }
        Action::CancelForm => {
            if state.is_form_open {
                state.close_form();
                Some(Some("Cancelled".to_string()))
            } else if state.delete_confirm_id.is_some() {
                state.cancel_delete();
                Some(Some("Deletion canceled".to_string()))
            } else {
                None
            }
        }
        Action::McpServerAdd => {
            state.open_add_form();
            Some(Some("Add MCP Server".to_string()))
        }
        Action::McpServerEdit => {
            if state.selected_server().is_some() {
                state.open_edit_form();
                Some(Some("Edit MCP Server".to_string()))
            } else {
                Some(Some("No server selected".to_string()))
            }
        }
        Action::McpServerToggle => {
            let res = state.toggle_enabled();
            match res {
                Ok(msg) => Some(Some(msg)),
                Err(err) => Some(Some(err)),
            }
        }
        Action::McpServerDelete => {
            if state.open_delete_confirm() {
                Some(None)
            } else {
                Some(Some("No MCP server selected".to_string()))
            }
        }
        Action::McpServerConfirmDelete => {
            let res = state.confirm_delete();
            match res {
                Ok(msg) => Some(Some(msg)),
                Err(err) => Some(Some(err)),
            }
        }
        Action::McpServerCancelDelete => {
            state.cancel_delete();
            Some(Some("Deletion canceled".to_string()))
        }
        Action::McpServerRefresh => {
            if let Err(err) = state.reload() {
                tracing::warn!("MCP reload error: {err}");
                return Some(Some(format!("Failed to reload: {err}")));
            }
            let count = state.servers.len();
            Some(Some(format!("MCP server list reloaded ({count} servers)")))
        }
        _ => None,
    }
}