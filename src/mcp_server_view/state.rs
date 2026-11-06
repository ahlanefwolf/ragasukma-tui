use ragasukma_core::agent::models::McpServer;

use super::service;
use super::types::{McpFormField, McpServerTabState};

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
    let config = server.config_value();
    let is_object = config.is_object();

    let detail = match (server.command(), server.url()) {
        (Some(cmd), _) => cmd,
        (None, Some(url)) => url,
        (None, None) => {
            if is_object {
                let keys = config.as_object().map_or(0, serde_json::Map::len);
                format!("{keys} keys")
            } else {
                format!("{}B", server.config.chars().count())
            }
        }
    };

    let extra_keys = config.as_object().map_or(0, |m| {
        m.keys()
            .filter(|k| !matches!(k.as_str(), "command" | "url" | "transport" | "args" | "env"))
            .count()
    });

    let suffix = if extra_keys > 0 {
        format!(" +{extra_keys}")
    } else {
        String::new()
    };

    let raw = format!("{} [{}] {detail}{suffix}", server.name, server.transport());
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
            form_name: String::new(),
            form_json: String::new(),
            form_enabled: true,
            focused_field: McpFormField::Name,
            form_error: None,
            name_cursor: 0,
            json_cursor: 0,
            json_scroll: 0,
            delete_confirm_id: None,
        }
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
        self.form_json = r#"{
  "command": "npx",
  "args": ["-y", "@modelcontextprotocol/server-everything"]
}"#
        .to_string();
        self.form_enabled = true;
        self.focused_field = McpFormField::Name;
        self.form_error = None;
        self.name_cursor = 0;
        self.json_cursor = 0;
        self.json_scroll = 0;
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
        self.form_name = name;

        // Format pretty JSON if valid JSON
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&config) {
            self.form_json = serde_json::to_string_pretty(&val).unwrap_or_else(|_| config.clone());
        } else {
            self.form_json = config;
        }

        self.form_enabled = is_enabled;
        self.focused_field = McpFormField::Name;
        self.form_error = None;
        self.name_cursor = self.form_name.len();
        self.json_cursor = 0;
        self.json_scroll = 0;
    }

    pub fn close_form(&mut self) {
        self.is_form_open = false;
        self.editing_id = None;
        self.form_error = None;
    }

    pub fn next_field(&mut self) {
        self.focused_field = self.focused_field.next();
    }

    pub fn prev_field(&mut self) {
        self.focused_field = self.focused_field.prev();
    }

    pub fn form_next_field(&mut self) {
        self.next_field();
    }

    pub fn form_prev_field(&mut self) {
        self.prev_field();
    }

    pub fn insert_char(&mut self, c: char) {
        self.ensure_valid_cursor();
        match self.focused_field {
            McpFormField::Name => {
                if c != '\n' && c != '\r' {
                    self.form_name.insert(self.name_cursor, c);
                    self.name_cursor += c.len_utf8();
                }
            }
            McpFormField::JsonConfig => {
                self.form_json.insert(self.json_cursor, c);
                self.json_cursor += c.len_utf8();
            }
            McpFormField::Enabled => {
                if c == ' ' {
                    self.form_enabled = !self.form_enabled;
                }
            }
            McpFormField::SaveBtn | McpFormField::CancelBtn => {}
        }
    }

    pub fn insert_newline(&mut self) {
        if self.focused_field == McpFormField::JsonConfig {
            self.insert_char('\n');
        }
    }

    pub fn form_input_char(&mut self, c: char) {
        self.insert_char(c);
    }

    pub fn backspace(&mut self) {
        self.ensure_valid_cursor();
        match self.focused_field {
            McpFormField::Name if self.name_cursor > 0 => {
                let prev = prev_char_boundary(&self.form_name, self.name_cursor);
                self.form_name.drain(prev..self.name_cursor);
                self.name_cursor = prev;
            }
            McpFormField::JsonConfig if self.json_cursor > 0 => {
                let prev = prev_char_boundary(&self.form_json, self.json_cursor);
                self.form_json.drain(prev..self.json_cursor);
                self.json_cursor = prev;
            }
            _ => {}
        }
    }

    pub fn form_backspace(&mut self) {
        self.backspace();
    }

    pub fn delete_forward(&mut self) {
        self.ensure_valid_cursor();
        match self.focused_field {
            McpFormField::Name if self.name_cursor < self.form_name.len() => {
                let next = next_char_boundary(&self.form_name, self.name_cursor);
                self.form_name.drain(self.name_cursor..next);
            }
            McpFormField::JsonConfig if self.json_cursor < self.form_json.len() => {
                let next = next_char_boundary(&self.form_json, self.json_cursor);
                self.form_json.drain(self.json_cursor..next);
            }
            _ => {}
        }
    }

    pub fn paste(&mut self, text: &str) {
        self.ensure_valid_cursor();
        match self.focused_field {
            McpFormField::Name => {
                let cleaned: String = text.chars().filter(|c| *c != '\n' && *c != '\r').collect();
                self.form_name.insert_str(self.name_cursor, &cleaned);
                self.name_cursor += cleaned.len();
            }
            McpFormField::JsonConfig => {
                let cleaned = text.replace("\r\n", "\n").replace('\r', "\n");
                self.form_json.insert_str(self.json_cursor, &cleaned);
                self.json_cursor += cleaned.len();
            }
            _ => {}
        }
    }

    pub fn form_paste(&mut self, text: &str) {
        self.paste(text);
    }

    pub fn cursor_left(&mut self) {
        self.ensure_valid_cursor();
        match self.focused_field {
            McpFormField::Name => {
                self.name_cursor = prev_char_boundary(&self.form_name, self.name_cursor);
            }
            McpFormField::JsonConfig => {
                self.json_cursor = prev_char_boundary(&self.form_json, self.json_cursor);
            }
            McpFormField::CancelBtn => {
                self.focused_field = McpFormField::SaveBtn;
            }
            _ => {}
        }
    }

    pub fn cursor_right(&mut self) {
        self.ensure_valid_cursor();
        match self.focused_field {
            McpFormField::Name => {
                self.name_cursor = next_char_boundary(&self.form_name, self.name_cursor);
            }
            McpFormField::JsonConfig => {
                self.json_cursor = next_char_boundary(&self.form_json, self.json_cursor);
            }
            McpFormField::SaveBtn => {
                self.focused_field = McpFormField::CancelBtn;
            }
            _ => {}
        }
    }

    pub fn cursor_up(&mut self) {
        self.ensure_valid_cursor();
        match self.focused_field {
            McpFormField::Name => {
                self.focused_field = McpFormField::CancelBtn;
            }
            McpFormField::JsonConfig => {
                let (line, col) = get_line_and_col(&self.form_json, self.json_cursor);
                if line > 0 {
                    self.json_cursor =
                        byte_offset_from_line_and_col(&self.form_json, line - 1, col);
                } else {
                    self.focused_field = McpFormField::Name;
                }
            }
            McpFormField::Enabled => {
                self.focused_field = McpFormField::JsonConfig;
            }
            McpFormField::SaveBtn | McpFormField::CancelBtn => {
                self.focused_field = McpFormField::Enabled;
            }
        }
    }

    pub fn cursor_down(&mut self) {
        self.ensure_valid_cursor();
        match self.focused_field {
            McpFormField::Name => {
                self.focused_field = McpFormField::JsonConfig;
            }
            McpFormField::JsonConfig => {
                let total_lines = self.form_json.split('\n').count();
                let (line, col) = get_line_and_col(&self.form_json, self.json_cursor);
                if line + 1 < total_lines {
                    self.json_cursor =
                        byte_offset_from_line_and_col(&self.form_json, line + 1, col);
                } else {
                    self.focused_field = McpFormField::Enabled;
                }
            }
            McpFormField::Enabled => {
                self.focused_field = McpFormField::SaveBtn;
            }
            McpFormField::SaveBtn | McpFormField::CancelBtn => {
                self.focused_field = McpFormField::Name;
            }
        }
    }

    pub fn cursor_home(&mut self) {
        self.ensure_valid_cursor();
        match self.focused_field {
            McpFormField::Name => {
                self.name_cursor = 0;
            }
            McpFormField::JsonConfig => {
                let (line, _) = get_line_and_col(&self.form_json, self.json_cursor);
                let start_of_line = byte_offset_from_line_and_col(&self.form_json, line, 0);
                if self.json_cursor == start_of_line {
                    self.json_cursor = 0;
                } else {
                    self.json_cursor = start_of_line;
                }
            }
            _ => {}
        }
    }

    pub fn cursor_end(&mut self) {
        self.ensure_valid_cursor();
        match self.focused_field {
            McpFormField::Name => {
                self.name_cursor = self.form_name.len();
            }
            McpFormField::JsonConfig => {
                let (line, _) = get_line_and_col(&self.form_json, self.json_cursor);
                let end_of_line = byte_offset_from_line_and_col(&self.form_json, line, usize::MAX);
                if self.json_cursor == end_of_line {
                    self.json_cursor = self.form_json.len();
                } else {
                    self.json_cursor = end_of_line;
                }
            }
            _ => {}
        }
    }

    pub fn ensure_valid_cursor(&mut self) {
        if self.name_cursor > self.form_name.len() {
            self.name_cursor = self.form_name.len();
        }
        while self.name_cursor > 0 && !self.form_name.is_char_boundary(self.name_cursor) {
            self.name_cursor -= 1;
        }

        if self.json_cursor > self.form_json.len() {
            self.json_cursor = self.form_json.len();
        }
        while self.json_cursor > 0 && !self.form_json.is_char_boundary(self.json_cursor) {
            self.json_cursor -= 1;
        }
    }

    pub fn save_form(&mut self) -> Result<String, String> {
        let name = self.form_name.trim().to_string();
        if name.is_empty() {
            self.form_error = Some("Server name cannot be empty".to_string());
            return Err("Server name cannot be empty".to_string());
        }

        let json_str = self.form_json.trim();
        if json_str.is_empty() {
            self.form_error = Some("JSON configuration cannot be empty".to_string());
            return Err("JSON configuration cannot be empty".to_string());
        }

        // Validate JSON
        let parsed: serde_json::Value = match serde_json::from_str(json_str) {
            Ok(v) => v,
            Err(e) => {
                let err_msg = format!("Invalid JSON: {e}");
                self.form_error = Some(err_msg.clone());
                return Err(err_msg);
            }
        };

        // Format raw JSON for storage
        let config_str = serde_json::to_string(&parsed).unwrap_or_else(|_| json_str.to_string());

        let id = match &self.editing_id {
            Some(existing) => existing.clone(),
            None => {
                let slug = service::slugify(&name);
                format!("{slug}-{}", uuid_prefix())
            }
        };

        service::save_server(&id, &name, &config_str, self.form_enabled)?;
        let is_edit = self.editing_id.is_some();
        self.close_form();
        self.reload()?;

        // Select the newly added or updated server
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

pub(crate) fn prev_char_boundary(s: &str, mut idx: usize) -> usize {
    if idx == 0 {
        return 0;
    }
    idx = idx.min(s.len());
    idx -= 1;
    while idx > 0 && !s.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

pub(crate) fn next_char_boundary(s: &str, mut idx: usize) -> usize {
    if idx >= s.len() {
        return s.len();
    }
    idx += 1;
    while idx < s.len() && !s.is_char_boundary(idx) {
        idx += 1;
    }
    idx
}

pub(crate) fn get_line_and_col(s: &str, byte_offset: usize) -> (usize, usize) {
    let offset = byte_offset.min(s.len());
    let mut line = 0;
    let mut col = 0;
    for (i, c) in s.char_indices() {
        if i >= offset {
            break;
        }
        if c == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    (line, col)
}

pub(crate) fn byte_offset_from_line_and_col(
    s: &str,
    target_line: usize,
    target_col: usize,
) -> usize {
    let mut current_line = 0;
    let mut current_col = 0;
    let mut last_idx = 0;

    for (i, c) in s.char_indices() {
        if current_line == target_line {
            if current_col >= target_col || c == '\n' {
                return i;
            }
            current_col += 1;
            last_idx = i + c.len_utf8();
        } else if c == '\n' {
            current_line += 1;
            if current_line == target_line {
                current_col = 0;
                last_idx = i + 1;
            }
        }
    }
    if current_line == target_line {
        return last_idx;
    }
    s.len()
}

fn uuid_prefix() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("{now:x}")
}
