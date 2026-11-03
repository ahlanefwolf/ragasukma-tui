use std::path::Path;

use super::types::{AgentFocusArea, ChatMessage, ChatSession, MessageRole, ToolCallInfo, ToolExecutionStatus};
use crate::core_bridge::PersistedMessageDto;

#[derive(Debug, Clone)]
pub struct AgentWorkspaceState {
    pub messages: Vec<ChatMessage>,
    pub input_buffer: String,
    pub cursor_pos: usize,
    pub scroll_offset: usize,
    pub is_input_focused: bool,
    pub is_streaming: bool,
    pub active_model: String,
    pub active_provider: String,
    pub temperature: f32,
    pub max_tokens: usize,
    pub total_prompt_tokens: usize,
    pub total_completion_tokens: usize,
    pub sessions: Vec<ChatSession>,
    pub active_session_idx: usize,
    pub selected_session_idx: usize,
    pub workspace_files: Vec<String>,
    pub selected_file_idx: usize,
    pub focus_area: AgentFocusArea,
    pub expanded_tool_idx: Option<usize>,
}

impl Default for AgentWorkspaceState {
    fn default() -> Self {
        Self::sample()
    }
}

impl AgentWorkspaceState {
    #[must_use]
    pub fn sample() -> Self {
        let msg1 = ChatMessage {
            role: MessageRole::System,
            sender_name: "System".to_string(),
            content: "Ragasukma Autonomous Agent engine initialized with memory bank & local tool loop.".to_string(),
            timestamp: "10:00:00".to_string(),
            tool_calls: Vec::new(),
            reasoning: None,
        };

        let msg2 = ChatMessage {
            role: MessageRole::User,
            sender_name: "User".to_string(),
            content: "Check workspace directory structure and summarize the core modules.".to_string(),
            timestamp: "10:01:15".to_string(),
            tool_calls: Vec::new(),
            reasoning: None,
        };

        let msg3 = ChatMessage {
            role: MessageRole::Assistant,
            sender_name: "Claude-3.5-Sonnet".to_string(),
            content: "I will inspect the workspace root and examine Cargo.toml and core source layout.".to_string(),
            timestamp: "10:01:17".to_string(),
            tool_calls: vec![
                ToolCallInfo {
                    id: "call_01".to_string(),
                    name: "workspace_list_dir".to_string(),
                    args_summary: "{\"path\": \".\", \"depth\": 2}".to_string(),
                    output_summary: Some("Found 12 crates and files: Cargo.toml, ragasukma-core, ragasukma-tui...".to_string()),
                    status: ToolExecutionStatus::Success,
                    duration_ms: Some(42),
                },
                ToolCallInfo {
                    id: "call_02".to_string(),
                    name: "workspace_read_file".to_string(),
                    args_summary: "{\"path\": \"ragasukma-core/Cargo.toml\"}".to_string(),
                    output_summary: Some("[package]\nname = \"ragasukma-core\"\nversion = \"0.2.0\"".to_string()),
                    status: ToolExecutionStatus::Success,
                    duration_ms: Some(18),
                },
            ],
            reasoning: Some("Inspected the root configuration. Now summarizing the project modular structure.".to_string()),
        };

        let msg4 = ChatMessage {
            role: MessageRole::Assistant,
            sender_name: "Claude-3.5-Sonnet".to_string(),
            content: "The workspace contains ragasukma-core and ragasukma-tui. Ready for autonomous execution.".to_string(),
            timestamp: "10:01:21".to_string(),
            tool_calls: Vec::new(),
            reasoning: None,
        };

        let initial_messages = vec![msg1, msg2, msg3, msg4];

        let sessions = vec![
            ChatSession {
                id: "sess-default".to_string(),
                title: "Default Session".to_string(),
                created_at: "Today 10:00".to_string(),
                messages: initial_messages.clone(),
            },
            ChatSession {
                id: "sess-refactor".to_string(),
                title: "Feature Refactor".to_string(),
                created_at: "Today 09:30".to_string(),
                messages: vec![ChatMessage {
                    role: MessageRole::System,
                    sender_name: "System".to_string(),
                    content: "Session initialized for code refactoring and architecture polish.".to_string(),
                    timestamp: "09:30:00".to_string(),
                    tool_calls: Vec::new(),
                    reasoning: None,
                }],
            },
            ChatSession {
                id: "sess-quota".to_string(),
                title: "Quota Inspection".to_string(),
                created_at: "Yesterday".to_string(),
                messages: vec![ChatMessage {
                    role: MessageRole::System,
                    sender_name: "System".to_string(),
                    content: "Session initialized for provider rate limits and token tracking.".to_string(),
                    timestamp: "Yesterday".to_string(),
                    tool_calls: Vec::new(),
                    reasoning: None,
                }],
            },
        ];

        let mut state = Self {
            messages: initial_messages,
            input_buffer: String::new(),
            cursor_pos: 0,
            scroll_offset: 0,
            is_input_focused: false,
            is_streaming: false,
            active_model: "claude-3-5-sonnet-20241022".to_string(),
            active_provider: "Claude".to_string(),
            temperature: 0.7,
            max_tokens: 4096,
            total_prompt_tokens: 3420,
            total_completion_tokens: 1180,
            sessions,
            active_session_idx: 0,
            selected_session_idx: 0,
            workspace_files: Vec::new(),
            selected_file_idx: 0,
            focus_area: AgentFocusArea::Chat,
            expanded_tool_idx: None,
        };
        state.scan_workspace_files();
        state
    }

    pub fn focus_input(&mut self) {
        self.is_input_focused = true;
    }

    pub fn unfocus_input(&mut self) {
        self.is_input_focused = false;
    }

    pub fn input_char(&mut self, c: char) {
        self.insert_char(c);
    }

    pub fn insert_char(&mut self, c: char) {
        if self.cursor_pos >= self.input_buffer.len() {
            self.input_buffer.push(c);
            self.cursor_pos = self.input_buffer.len();
        } else {
            let byte_idx = self
                .input_buffer
                .char_indices()
                .nth(self.cursor_pos)
                .map_or(self.input_buffer.len(), |(i, _)| i);
            self.input_buffer.insert(byte_idx, c);
            self.cursor_pos += 1;
        }
    }

    pub fn backspace(&mut self) {
        if self.cursor_pos > 0 && !self.input_buffer.is_empty() {
            let prev_idx = self.cursor_pos - 1;
            let byte_idx = self
                .input_buffer
                .char_indices()
                .nth(prev_idx)
                .map(|(i, _)| i);
            if let Some(b) = byte_idx {
                self.input_buffer.remove(b);
                self.cursor_pos = prev_idx;
            }
        }
    }

    pub fn clear_input(&mut self) {
        self.input_buffer.clear();
        self.cursor_pos = 0;
    }

    pub fn clear_chat(&mut self) {
        self.messages.clear();
        self.scroll_offset = 0;
        self.expanded_tool_idx = None;
    }

    pub fn submit_prompt(&mut self) -> Option<String> {
        let trimmed = self.input_buffer.trim().to_string();
        if trimmed.is_empty() {
            return None;
        }

        let user_msg = ChatMessage {
            role: MessageRole::User,
            sender_name: "User".to_string(),
            content: trimmed.clone(),
            timestamp: "Now".to_string(),
            tool_calls: Vec::new(),
            reasoning: None,
        };
        self.messages.push(user_msg);
        self.input_buffer.clear();
        self.cursor_pos = 0;
        self.scroll_to_bottom();
        Some(trimmed)
    }

    pub fn add_assistant_response(
        &mut self,
        content: &str,
        tool_calls: Vec<ToolCallInfo>,
        reasoning: Option<String>,
    ) {
        let msg = ChatMessage {
            role: MessageRole::Assistant,
            sender_name: self.active_model.clone(),
            content: content.to_string(),
            timestamp: "Now".to_string(),
            tool_calls,
            reasoning,
        };
        self.messages.push(msg);
        self.scroll_to_bottom();
    }

    pub fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }

    pub fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(1);
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = self.messages.len().saturating_sub(1);
    }

    pub fn toggle_tool_expand(&mut self) {
        if self.expanded_tool_idx.is_some() {
            self.expanded_tool_idx = None;
        } else {
            for (idx, msg) in self.messages.iter().enumerate().rev() {
                if !msg.tool_calls.is_empty() {
                    self.expanded_tool_idx = Some(idx);
                    break;
                }
            }
        }
    }

    pub fn scan_workspace_files(&mut self) {
        let mut found = Vec::new();

        // Check local current workspace directory
        let search_dirs = [".", "src", "docs", "../ragasukma-core/src", "../ai-voice/src"];
        for d in search_dirs {
            let p = Path::new(d);
            if let Ok(entries) = std::fs::read_dir(p) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        let name = path.to_string_lossy().to_string();
                        // Ignore git and target build artifacts
                        if !name.contains(".git") && !name.contains("target") {
                            found.push(name);
                        }
                    }
                }
            }
        }

        found.sort();
        if found.is_empty() {
            // Fallback default files if environment has restricted filesystem
            found = vec![
                "Cargo.toml".to_string(),
                "src/lib.rs".to_string(),
                "src/main.rs".to_string(),
                "src/app.rs".to_string(),
                "src/agent_view.rs".to_string(),
                "src/ui.rs".to_string(),
            ];
        }

        self.workspace_files = found;
        if self.selected_file_idx >= self.workspace_files.len() {
            self.selected_file_idx = self.workspace_files.len().saturating_sub(1);
        }
    }

    pub fn select_file_next(&mut self) {
        if !self.workspace_files.is_empty() {
            self.selected_file_idx = (self.selected_file_idx + 1) % self.workspace_files.len();
        }
    }

    pub fn select_file_prev(&mut self) {
        if !self.workspace_files.is_empty() {
            if self.selected_file_idx == 0 {
                self.selected_file_idx = self.workspace_files.len() - 1;
            } else {
                self.selected_file_idx -= 1;
            }
        }
    }

    pub fn insert_selected_file_to_input(&mut self) {
        if let Some(file) = self.workspace_files.get(self.selected_file_idx) {
            let ref_str = format!("@{} ", file);
            for c in ref_str.chars() {
                self.insert_char(c);
            }
            self.is_input_focused = true;
            self.focus_area = AgentFocusArea::Chat;
        }
    }

    pub fn select_session_next(&mut self) {
        if !self.sessions.is_empty() {
            self.selected_session_idx = (self.selected_session_idx + 1) % self.sessions.len();
        }
    }

    pub fn select_session_prev(&mut self) {
        if !self.sessions.is_empty() {
            if self.selected_session_idx == 0 {
                self.selected_session_idx = self.sessions.len() - 1;
            } else {
                self.selected_session_idx -= 1;
            }
        }
    }

    pub fn switch_to_selected_session(&mut self) -> Option<String> {
        if self.selected_session_idx >= self.sessions.len() {
            return None;
        }

        // Save current active session messages
        if let Some(active_sess) = self.sessions.get_mut(self.active_session_idx) {
            active_sess.messages = self.messages.clone();
        }

        // Switch to new session
        self.active_session_idx = self.selected_session_idx;

        self.sessions.get(self.active_session_idx).map(|s| {
            self.messages = s.messages.clone();
            self.scroll_offset = self.messages.len().saturating_sub(1);
            s.id.clone()
        })
    }

    pub fn create_new_session(&mut self) -> String {
        // Save current active session
        if let Some(active_sess) = self.sessions.get_mut(self.active_session_idx) {
            active_sess.messages = self.messages.clone();
        }

        let num = self.sessions.len() + 1;
        let new_id = format!("sess-{}", num);
        let new_title = format!("Session #{}", num);
        let init_msg = ChatMessage {
            role: MessageRole::System,
            sender_name: "System".to_string(),
            content: format!("Started autonomous agent session [{}].", new_title),
            timestamp: "Now".to_string(),
            tool_calls: Vec::new(),
            reasoning: None,
        };

        let new_session = ChatSession {
            id: new_id,
            title: new_title.clone(),
            created_at: "Just now".to_string(),
            messages: vec![init_msg.clone()],
        };

        self.sessions.push(new_session);
        self.active_session_idx = self.sessions.len() - 1;
        self.selected_session_idx = self.active_session_idx;
        self.messages = vec![init_msg];
        self.scroll_offset = 0;

        new_title
    }

    pub fn delete_selected_session(&mut self) -> Option<String> {
        if self.sessions.len() <= 1 {
            // Keep at least one session
            self.messages.clear();
            let _ = self.create_new_session();
            if self.sessions.len() > 1 {
                let removed = self.sessions.remove(0);
                self.active_session_idx = 0;
                self.selected_session_idx = 0;
                return Some(removed.id);
            }
            return None;
        }

        let del_idx = self.selected_session_idx;
        let removed_session = self.sessions.remove(del_idx);

        if self.active_session_idx == del_idx {
            self.active_session_idx = self.active_session_idx.min(self.sessions.len() - 1);
            if let Some(sess) = self.sessions.get(self.active_session_idx) {
                self.messages = sess.messages.clone();
                self.scroll_offset = self.messages.len().saturating_sub(1);
            }
        } else if self.active_session_idx > del_idx {
            self.active_session_idx -= 1;
        }

        if self.selected_session_idx >= self.sessions.len() {
            self.selected_session_idx = self.sessions.len().saturating_sub(1);
        }

        Some(removed_session.id)
    }

    pub fn load_persisted_sessions(&mut self, persisted: Vec<(String, String)>, active_id: &str) {
        if persisted.is_empty() {
            return;
        }

        let mut loaded = Vec::with_capacity(persisted.len());
        let mut active_index = 0;

        for (idx, (id, title)) in persisted.into_iter().enumerate() {
            if id == active_id {
                active_index = idx;
            }
            loaded.push(ChatSession {
                id,
                title,
                created_at: "Persisted".to_string(),
                messages: Vec::new(),
            });
        }

        self.sessions = loaded;
        self.active_session_idx = active_index;
        self.selected_session_idx = active_index;
    }

    pub fn load_persisted_messages(
        &mut self,
        session_id: &str,
        raw_msgs: Vec<PersistedMessageDto>,
    ) {
        let mut converted: Vec<ChatMessage> = Vec::new();

        for m in raw_msgs {
            let role = match m.role.as_str() {
                "user" => MessageRole::User,
                "assistant" => MessageRole::Assistant,
                _ => MessageRole::System,
            };

            let tool_calls = m
                .tool_calls
                .and_then(|tc_str| serde_json::from_str::<serde_json::Value>(&tc_str).ok())
                .and_then(|val| {
                    val.as_array().map(|arr| {
                        arr.iter()
                            .filter_map(|item| {
                                let name = item.get("name").and_then(|v| v.as_str())?.to_string();
                                let args = item
                                    .get("args")
                                    .map(|v| v.to_string())
                                    .unwrap_or_else(|| "{}".to_string());
                                Some(ToolCallInfo {
                                    id: item
                                        .get("id")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("call")
                                        .to_string(),
                                    name,
                                    args_summary: args,
                                    output_summary: None,
                                    status: ToolExecutionStatus::Success,
                                    duration_ms: None,
                                })
                            })
                            .collect::<Vec<_>>()
                    })
                })
                .unwrap_or_default();

            let ts_str = if m.created_at > 0 {
                let secs = if m.created_at > 1_000_000_000_000 {
                    m.created_at / 1000
                } else {
                    m.created_at
                };
                let s = secs % 60;
                let min = (secs / 60) % 60;
                let h = (secs / 3600) % 24;
                format!("{:02}:{:02}:{:02}", h, min, s)
            } else {
                "Saved".to_string()
            };

            converted.push(ChatMessage {
                role,
                sender_name: if m.role == "assistant" {
                    self.active_model.clone()
                } else if m.role == "user" {
                    "User".to_string()
                } else {
                    "System".to_string()
                },
                content: m.content.unwrap_or_default(),
                timestamp: ts_str,
                tool_calls,
                reasoning: m.thought,
            });
        }

        // Cache into matching session and set active if it matches active session
        for sess in &mut self.sessions {
            if sess.id == session_id {
                sess.messages = converted.clone();
                break;
            }
        }

        if let Some(active_sess) = self.sessions.get(self.active_session_idx)
            && active_sess.id == session_id
        {
            self.messages = converted;
            self.scroll_to_bottom();
        }
    }

    pub fn cycle_focus_next(&mut self) {
        self.focus_area = match self.focus_area {
            AgentFocusArea::Sessions => AgentFocusArea::Files,
            AgentFocusArea::Files => AgentFocusArea::Chat,
            AgentFocusArea::Chat => AgentFocusArea::Sessions,
        };
    }

    pub fn cycle_focus_prev(&mut self) {
        self.focus_area = match self.focus_area {
            AgentFocusArea::Sessions => AgentFocusArea::Chat,
            AgentFocusArea::Files => AgentFocusArea::Sessions,
            AgentFocusArea::Chat => AgentFocusArea::Files,
        };
    }

    pub fn new_session(&mut self) {
        self.create_new_session();
    }

    pub fn append_chunk(&mut self, chunk: &str) {
        self.is_streaming = true;
        if let Some(last) = self.messages.last_mut()
            && last.role == MessageRole::Assistant
        {
            last.content.push_str(chunk);
            self.scroll_to_bottom();
            return;
        }
        let msg = ChatMessage {
            role: MessageRole::Assistant,
            sender_name: self.active_model.clone(),
            content: chunk.to_string(),
            timestamp: "Now".to_string(),
            tool_calls: Vec::new(),
            reasoning: None,
        };
        self.messages.push(msg);
        self.scroll_to_bottom();
    }

    pub fn append_thought(&mut self, thought: &str) {
        self.is_streaming = true;
        if let Some(last) = self.messages.last_mut()
            && last.role == MessageRole::Assistant
        {
            if let Some(ref mut r) = last.reasoning {
                r.push_str(thought);
            } else {
                last.reasoning = Some(thought.to_string());
            }
            return;
        }
        let msg = ChatMessage {
            role: MessageRole::Assistant,
            sender_name: self.active_model.clone(),
            content: String::new(),
            timestamp: "Now".to_string(),
            tool_calls: Vec::new(),
            reasoning: Some(thought.to_string()),
        };
        self.messages.push(msg);
        self.scroll_to_bottom();
    }

    pub fn finish_turn(&mut self) {
        self.is_streaming = false;
        self.scroll_to_bottom();
    }

    pub fn clear_messages(&mut self) {
        self.messages.clear();
        self.scroll_offset = 0;
    }
}
