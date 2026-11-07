use ragasukma_core::agent::models::McpServer;

use super::service;
use super::types::{McpFormField, McpServerTabState};
use crate::ui::TextInput;

/// Maximum characters rendered for the `config_preview_summary` hint.
///
/// Keeps the chat input footer from overflowing narrow terminals.
pub const CONFIG_PREVIEW_MAX_CHARS: usize = 56;

/// Truncate `s` to at most `max` characters, appending `…` when shortened.
///
/// Uses char counts (not bytes) so multi-byte server names never panic or
/// produce invalid UTF-8 boundaries.
pub fn truncate_chars(s: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    let mut out = String::new();
    for (count, ch) in s.chars().enumerate() {
        if count + 1 >= max {
            out.push('…');
            return out;
        }
        out.push(ch);
    }
    out
}

/// Build a short single-line summary of a server's config.
///
/// The summary shows `<name> [<transport>]` plus human detail: the command for
/// `stdio` servers, the URL for `sse`/`http` servers, and the number of extra
/// top-level JSON keys. It is already truncated to
/// [`CONFIG_PREVIEW_MAX_CHARS`] so callers can render it directly.
///
/// Returns `None` when no server is selected at `selected_idx`.
#[must_use]
pub fn config_preview_summary(servers: &[McpServer], selected_idx: usize) -> Option<String> {
    let server = servers.get(selected_idx)?;

    let detail = match (server.command(), server.url()) {
        (Some(cmd), _) => cmd,
        (None, Some(url)) => url,
        (None, None) => {
            let mut preview = String::new();
            for line in server.config.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() && !trimmed.starts_with('#') && !trimmed.starts_with("```") {
                    preview = trimmed.to_string();
                    break;
                }
            }
            if preview.is_empty() {
                format!("{}B markdown", server.config.chars().count())
            } else {
                truncate_chars(&preview, 24)
            }
        }
    };

    let raw = format!("{} [{}] {detail}", server.name, server.transport());
    Some(truncate_chars(&raw, CONFIG_PREVIEW_MAX_CHARS))
}

/// Full JSON keys preview, used when the user explicitly opens the config.
///
/// Returns the sorted list of top-level object keys of the selected server's
/// config, or `None` when nothing is selected or the config is not an object.
#[must_use]
pub fn config_preview_keys(servers: &[McpServer], selected_idx: usize) -> Option<Vec<String>> {
    let server = servers.get(selected_idx)?;
    let mut keys: Vec<String> = server.config_value().as_object()?.keys().cloned().collect();
    keys.sort();
    Some(keys)
}

impl Default for McpServerTabState {
    fn default() -> Self {
        Self::new()
    }
}

impl McpServerTabState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            servers: Vec::new(),
            selected_idx: 0,
            is_form_open: false,
            editing_id: None,
            form_name: TextInput::new_single_line().with_placeholder("Server display name"),
            form_json: TextInput::new_multiline_editor().with_placeholder("# MCP Configuration (Markdown)\n\n- transport: stdio\n- command: npx\n- args: -y ...\n\n## Capabilities\nServer capabilities and specifications."),
            form_enabled: true,
            focused_field: McpFormField::Name,
            form_error: None,
            is_tools_modal_open: false,
            server_tools: Vec::new(),
            tools_selected_idx: 0,
            is_sync_modal_open: false,
            available_models: Vec::new(),
            selected_model_idx: 0,
            sync_search_input: TextInput::new_single_line()
                .with_placeholder("Type model name or keyword to filter...")
                .with_focus(true),
            is_syncing: false,
            sync_status_message: None,
            delete_confirm_id: None,
        }
    }

    pub fn update_field_focus(&mut self) {
        self.form_name.is_focused = self.focused_field == McpFormField::Name;
        self.form_json.is_focused = self.focused_field == McpFormField::Config;
    }

    pub fn set_servers(&mut self, servers: Vec<McpServer>) {
        self.servers = servers;
        if self.servers.is_empty() {
            self.selected_idx = 0;
        } else if self.selected_idx >= self.servers.len() {
            self.selected_idx = self.servers.len() - 1;
        }
    }

    pub fn reload(&mut self) -> Result<(), String> {
        let servers = service::load_servers()?;
        self.set_servers(servers);
        Ok(())
    }

    #[must_use]
    pub fn selected_server(&self) -> Option<&McpServer> {
        self.servers.get(self.selected_idx)
    }

    pub fn select_next(&mut self) {
        if !self.servers.is_empty() {
            self.selected_idx = (self.selected_idx + 1) % self.servers.len();
        }
    }

    pub fn select_prev(&mut self) {
        if !self.servers.is_empty() {
            self.selected_idx = if self.selected_idx == 0 {
                self.servers.len() - 1
            } else {
                self.selected_idx - 1
            };
        }
    }

    pub fn toggle_enabled(&mut self) -> Result<String, String> {
        let Some(server) = self.selected_server() else {
            return Err("No server selected".to_string());
        };
        let id = server.id.clone();
        let name = server.name.clone();
        let config = server.config.clone();
        let new_status = server.is_enabled == 0;

        service::save_server(&id, &name, &config, new_status)?;
        self.reload()?;

        let status_str = if new_status { "enabled" } else { "disabled" };
        Ok(format!("MCP Server '{name}' set to {status_str}"))
    }

    pub fn open_add_form(&mut self) {
        self.is_form_open = true;
        self.editing_id = None;
        self.form_name.clear();
        let default_config = r#"# MCP Server Configuration

## Runtime Execution
```exec
transport: stdio
command: npx
args:
  - "-y"
  - "@modelcontextprotocol/server-everything"
```

## Notes & Capabilities
Server capabilities and parameter specifications.
"#;
        self.form_json.set_value(default_config);
        self.form_enabled = true;
        self.focused_field = McpFormField::Name;
        self.form_error = None;
        self.update_field_focus();
    }

    pub fn open_edit_form(&mut self) {
        let (id, name, config, is_enabled) = {
            let Some(server) = self.selected_server() else {
                return;
            };
            (
                server.id.clone(),
                server.name.clone(),
                server.config.clone(),
                server.is_enabled != 0,
            )
        };

        self.is_form_open = true;
        self.editing_id = Some(id);
        self.form_name.set_value(&name);
        self.form_json.set_value(&config);
        self.form_enabled = is_enabled;
        self.focused_field = McpFormField::Name;
        self.form_error = None;
        self.update_field_focus();
    }

    pub fn close_form(&mut self) {
        self.is_form_open = false;
        self.editing_id = None;
        self.form_error = None;
    }

    pub fn open_tools_modal(&mut self) {
        if let Some(srv) = self.selected_server() {
            let id = srv.id.clone();
            match service::load_server_tools(&id) {
                Ok(tools) => {
                    self.server_tools = tools;
                    self.tools_selected_idx = 0;
                    self.is_tools_modal_open = true;
                }
                Err(e) => {
                    self.form_error = Some(e);
                }
            }
        }
    }

    pub fn close_tools_modal(&mut self) {
        self.is_tools_modal_open = false;
        self.server_tools.clear();
        self.tools_selected_idx = 0;
    }

    pub fn tools_select_next(&mut self) {
        if !self.server_tools.is_empty() {
            self.tools_selected_idx =
                (self.tools_selected_idx + 1).min(self.server_tools.len() - 1);
        }
    }

    pub fn tools_select_prev(&mut self) {
        self.tools_selected_idx = self.tools_selected_idx.saturating_sub(1);
    }

    pub fn open_sync_modal(&mut self) {
        if self.selected_server().is_some() {
            let mut models = crate::agent_view::models::fetch_core_available_models();
            if models.is_empty() {
                models.push("gemini-2.5-flash".to_string());
            }
            self.available_models = models;
            self.selected_model_idx = 0;
            self.sync_search_input.clear();
            self.sync_search_input.focus();
            self.sync_status_message = None;
            self.is_syncing = false;
            self.is_sync_modal_open = true;
        }
    }

    pub fn close_sync_modal(&mut self) {
        self.is_sync_modal_open = false;
        self.is_syncing = false;
        self.sync_search_input.clear();
        self.sync_search_input.unfocus();
        self.sync_status_message = None;
    }

    pub fn filtered_sync_models(&self) -> Vec<String> {
        let q = self.sync_search_input.value.trim().to_lowercase();
        if q.is_empty() {
            self.available_models.clone()
        } else {
            self.available_models
                .iter()
                .filter(|m| m.to_lowercase().contains(&q))
                .cloned()
                .collect()
        }
    }

    pub fn sync_model_next(&mut self) {
        let filtered = self.filtered_sync_models();
        if !filtered.is_empty() {
            if self.selected_model_idx + 1 >= filtered.len() {
                self.selected_model_idx = 0;
            } else {
                self.selected_model_idx += 1;
            }
        }
    }

    pub fn sync_model_prev(&mut self) {
        let filtered = self.filtered_sync_models();
        if !filtered.is_empty() {
            if self.selected_model_idx == 0 {
                self.selected_model_idx = filtered.len() - 1;
            } else {
                self.selected_model_idx -= 1;
            }
        }
    }

    pub fn sync_search_handle_key(&mut self, key: crossterm::event::KeyEvent) {
        self.sync_search_input.handle_key(key);
        let filtered = self.filtered_sync_models();
        if self.selected_model_idx >= filtered.len() && !filtered.is_empty() {
            self.selected_model_idx = filtered.len() - 1;
        }
    }

    pub fn execute_sync(&mut self) -> Result<String, String> {
        let Some(server) = self.selected_server().cloned() else {
            return Err("No server selected for sync".to_string());
        };

        let filtered = self.filtered_sync_models();
        let chosen_model = filtered
            .get(self.selected_model_idx)
            .map(|s| s.as_str())
            .or_else(|| self.available_models.first().map(|s| s.as_str()));

        self.is_syncing = true;
        match service::sync_tools_for_server(&server, chosen_model) {
            Ok(tools) => {
                let count = tools.len();
                self.server_tools = tools;
                self.is_syncing = false;
                self.is_sync_modal_open = false;
                self.sync_search_input.clear();
                self.sync_search_input.unfocus();
                let msg = format!("Successfully synced {count} tools for server '{}'", server.name);
                self.sync_status_message = Some(msg.clone());
                Ok(msg)
            }
            Err(e) => {
                self.is_syncing = false;
                self.sync_status_message = Some(format!("Sync failed: {e}"));
                Err(e)
            }
        }
    }

    pub fn next_field(&mut self) {
        self.focused_field = self.focused_field.next();
        self.update_field_focus();
    }

    pub fn prev_field(&mut self) {
        self.focused_field = self.focused_field.prev();
        self.update_field_focus();
    }

    pub fn form_next_field(&mut self) {
        self.next_field();
    }

    pub fn form_prev_field(&mut self) {
        self.prev_field();
    }

    pub fn form_handle_key(&mut self, key: crossterm::event::KeyEvent) {
        match self.focused_field {
            McpFormField::Name => {
                self.form_name.handle_key(key);
            }
            McpFormField::Config => {
                self.form_json.handle_key(key);
            }
            McpFormField::Enabled => {
                if key.code == crossterm::event::KeyCode::Char(' ') {
                    self.form_enabled = !self.form_enabled;
                }
            }
            McpFormField::SaveBtn | McpFormField::CancelBtn => {}
        }
    }

    pub fn form_paste(&mut self, text: &str) {
        match self.focused_field {
            McpFormField::Name => {
                self.form_name.paste(text);
            }
            McpFormField::Config => {
                self.form_json.paste(text);
            }
            _ => {}
        }
    }

    pub fn form_toggle_enabled(&mut self) {
        self.form_enabled = !self.form_enabled;
    }

    pub fn save_form(&mut self) -> Result<String, String> {
        let name = self.form_name.value.trim().to_string();
        if name.is_empty() {
            self.form_error = Some("Server name cannot be empty".to_string());
            return Err("Server name cannot be empty".to_string());
        }

        let config_str = self.form_json.value.trim();
        if config_str.is_empty() {
            let err_msg = "Configuration cannot be empty".to_string();
            self.form_error = Some(err_msg.clone());
            return Err(err_msg);
        }

        let id = match &self.editing_id {
            Some(existing) => existing.clone(),
            None => {
                let slug = service::slugify(&name);
                format!("{slug}-{}", uuid_prefix())
            }
        };

        service::save_server(&id, &name, config_str, self.form_enabled)?;
        let is_edit = self.editing_id.is_some();
        self.close_form();
        self.reload()?;

        if let Some(pos) = self.servers.iter().position(|s| s.id == id) {
            self.selected_idx = pos;
        }

        let action_str = if is_edit { "updated" } else { "added" };
        Ok(format!("MCP Server '{name}' {action_str} successfully"))
    }

    pub fn open_delete_confirm(&mut self) -> bool {
        if let Some(server) = self.selected_server() {
            self.delete_confirm_id = Some(server.id.clone());
            true
        } else {
            false
        }
    }

    pub fn cancel_delete(&mut self) {
        self.delete_confirm_id = None;
    }

    /// Short single-line summary of the currently selected server's JSON config.
    ///
    /// Used as a compact indicator in the chat input title bar, since the
    /// previous "JSON Configuration Preview" sidebar card was removed.
    #[must_use]
    pub fn config_preview_summary(&self) -> Option<String> {
        config_preview_summary(&self.servers, self.selected_idx)
    }

    pub fn confirm_delete(&mut self) -> Result<String, String> {
        let Some(id) = self.delete_confirm_id.take() else {
            return Err("No server marked for deletion".to_string());
        };

        let server_name = self
            .servers
            .iter()
            .find(|s| s.id == id)
            .map(|s| s.name.clone())
            .unwrap_or_else(|| id.clone());

        service::remove_server(&id)?;
        self.reload()?;
        Ok(format!("MCP Server '{server_name}' deleted"))
    }
}

fn uuid_prefix() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("{now:x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_form_markdown_editing() {
        let mut state = McpServerTabState::default();
        state.open_add_form();
        assert!(state.is_form_open);
        assert_eq!(state.focused_field, McpFormField::Name);

        // Edit Name
        for c in "MySrv".chars() {
            state.form_name.insert_char(c);
        }
        assert_eq!(state.form_name.value, "MySrv");

        // Switch to Markdown Config
        state.next_field();
        assert_eq!(state.focused_field, McpFormField::Config);

        // Input Markdown Content
        state.form_json.clear();
        for c in "# Header".chars() {
            state.form_json.insert_char(c);
        }
        state.form_json.insert_char('\n');
        for c in "Some markdown description".chars() {
            state.form_json.insert_char(c);
        }

        assert_eq!(state.form_json.value, "# Header\nSome markdown description");
    }

    #[test]
    fn test_sync_modal_flow() {
        let mut state = McpServerTabState::default();
        let server = McpServer {
            id: "srv-1".into(),
            name: "Server 1".into(),
            config: "# Demo".into(),
            is_enabled: 1,
            created_at: 1000,
        };
        state.set_servers(vec![server]);
        state.open_sync_modal();
        assert!(state.is_sync_modal_open);
        assert!(!state.available_models.is_empty());
        assert_eq!(state.selected_model_idx, 0);

        state.available_models = vec!["claude-3-5".to_string(), "deepseek-coder".to_string()];
        state.sync_model_next();
        assert_eq!(state.selected_model_idx, 1);

        // Wrap around cycling
        state.sync_model_next();
        assert_eq!(state.selected_model_idx, 0);

        state.sync_model_prev();
        assert_eq!(state.selected_model_idx, 1);

        // Test filtering
        state.sync_search_input.set_value("deep");
        let filtered = state.filtered_sync_models();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0], "deepseek-coder");

        state.close_sync_modal();
        assert!(!state.is_sync_modal_open);
    }

    #[test]
    fn test_markdown_summary_preview() {
        let md = "# My MCP Server\n\n```exec\ntransport: stdio\ncommand: npx\n```\n";
        let server = McpServer {
            id: "1".into(),
            name: "Test".into(),
            config: md.into(),
            is_enabled: 1,
            created_at: 1000,
        };
        let servers = vec![server];
        let summary = config_preview_summary(&servers, 0);
        assert!(summary.is_some());
        let text = summary.unwrap();
        assert!(text.contains("stdio") || text.contains("npx") || text.contains("transport"));
    }
}
