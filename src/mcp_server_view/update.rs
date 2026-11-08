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
        Action::FormKey(key) if state.is_form_open => {
            state.form_handle_key(*key);
            Some(None)
        }
        Action::FormPaste(text) if state.is_form_open => {
            state.form_paste(text);
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
        Action::McpServerViewTools => {
            state.open_tools_modal();
            Some(None)
        }
        Action::McpServerCloseTools => {
            state.close_tools_modal();
            Some(None)
        }
        Action::McpServerToolsNext => {
            state.tools_select_next();
            Some(None)
        }
        Action::McpServerToolsPrev => {
            state.tools_select_prev();
            Some(None)
        }
        Action::McpServerTogglePane => {
            state.toggle_pane();
            Some(None)
        }
        Action::McpServerToolDetailOpen => {
            state.open_tool_detail();
            Some(None)
        }
        Action::McpServerToolDetailClose => {
            state.close_tool_detail();
            Some(None)
        }
        Action::McpServerToolDetailScrollUp => {
            state.tool_detail_scroll_up();
            Some(None)
        }
        Action::McpServerToolDetailScrollDown => {
            state.tool_detail_scroll_down();
            Some(None)
        }
        Action::McpServerSyncTools => {
            state.open_sync_modal();
            Some(None)
        }
        Action::McpServerSyncModalClose => {
            state.close_sync_modal();
            Some(None)
        }
        Action::McpServerSyncModelNext => {
            state.sync_model_next();
            Some(None)
        }
        Action::McpServerSyncModelPrev => {
            state.sync_model_prev();
            Some(None)
        }
        Action::McpServerSyncKey(key) => {
            state.sync_search_handle_key(*key);
            Some(None)
        }
        Action::McpServerSyncConfirm => match state.execute_sync() {
            Ok(msg) => Some(Some(msg)),
            Err(e) => Some(Some(format!("Sync error: {e}"))),
        },
        Action::McpServerSelectNext => {
            if state.is_form_open || state.delete_confirm_id.is_some() {
                return None;
            }
            state.select_next();
            Some(None)
        }
        Action::McpServerSelectPrev => {
            if state.is_form_open || state.delete_confirm_id.is_some() {
                return None;
            }
            state.select_prev();
            Some(None)
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
