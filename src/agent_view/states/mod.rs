use ratatui::text::Line;
use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct ChatLayoutCache {
    pub width: usize,
    pub message_count: usize,
    pub visible_limit: usize,
    pub last_content_len: usize,
    pub last_reasoning_len: usize,
    pub last_tools_len: usize,
    pub is_running: bool,
    pub expanded_tools_len: usize,
    pub reasoning_expanded: bool,
    pub lines: Vec<Line<'static>>,
}

use super::types::{
    AddFolderModalState, AgentFocusArea, AgentSidebarTab, ApprovalRequest, ChatMessage,
    CreateProjectModalState, FileTagQuery, MessageRole, ModelSelectorModalState, ProjectItem,
    ProjectSwitcherModalState, TagItem, TaskSessionItem, TokenBreakdown, ToolCallInfo,
    ToolExecutionStatus, ToolFilter, WorkspaceFileNode, WorkspaceFolderItem,
};
use crate::core_bridge::PersistedMessageDto;

#[derive(Debug, Clone)]
pub struct AgentWorkspaceState {
    pub messages: Vec<ChatMessage>,
    pub input_buffer: String,
    pub cursor_pos: usize,
    pub input_scroll_line: usize,
    pub input_wrap_width: Cell<usize>,
    pub scroll_offset: usize,
    pub chat_max_scroll: Cell<usize>,
    pub is_input_focused: bool,
    pub focus_area: AgentFocusArea,
    pub is_running: bool,
    pub active_turn_id: Option<String>,
    pub active_model: String,
    pub available_models: Vec<String>,
    pub expanded_tools: HashSet<usize>,
    pub reasoning_expanded: bool,
    pub auto_scroll: bool,
    pub spinner_tick: usize,
    /// Number of most recent messages to render (lazy load window limit).
    pub chat_visible_limit: usize,

    // Phase 5: Prompt History Navigation
    pub history: Vec<String>,
    pub history_index: Option<usize>,
    pub history_draft: String,

    // Multi-Folder Workspace Support (1:1 with web agent)
    pub workspace_folders: Vec<PathBuf>,
    pub active_folder_idx: usize,
    pub add_folder_modal: AddFolderModalState,

    // Phase 1: Workspace Files & Tag Autocomplete
    pub workspace_files: Vec<WorkspaceFileNode>,
    pub selected_file_idx: usize,
    pub expanded_dirs: HashSet<String>,
    pub tag_query: Option<FileTagQuery>,
    pub all_workspace_tag_items: Vec<TagItem>,

    // Phase 3: Tool Execution & Filter
    pub tool_filter: ToolFilter,
    pub pending_approval: Option<ApprovalRequest>,

    // Tasks & Sessions (1:1 with ai-voice AgentSidebar)
    pub sidebar_tab: AgentSidebarTab,
    pub sessions: Vec<TaskSessionItem>,
    pub selected_session_idx: usize,
    pub active_session_id: Option<String>,
    pub total_prompt_tokens: usize,
    pub total_completion_tokens: usize,
    pub max_context_limit: usize,
    pub session_start_instant: std::time::Instant,

    // Projects (Database SQLite migrations/0004)
    pub projects: Vec<ProjectItem>,
    pub selected_project_idx: usize,
    pub active_project_id: Option<String>,
    pub active_project_name: String,
    pub project_modal: ProjectSwitcherModalState,
    pub create_project_modal: CreateProjectModalState,
    pub model_modal: ModelSelectorModalState,

    // MCP Servers (project-scoped)
    pub mcp_servers: Vec<ragasukma_core::agent::models::McpServer>,
    pub mcp_active_ids: Vec<String>,
    pub selected_mcp_idx: usize,

    // Phase 6: Responsive Layout & Inspection
    pub is_sidebar_visible: bool,
    pub is_inspector_visible: bool,

    // Chat Layout Cache to avoid re-wrapping on scroll
    pub chat_layout_cache: RefCell<ChatLayoutCache>,
}

impl Default for AgentWorkspaceState {
    fn default() -> Self {
        let available_models = crate::agent_view::models::fetch_core_available_models();
        let active_model = available_models
            .first()
            .cloned()
            .unwrap_or_else(|| "cc/claude-3-5-sonnet-20241022".to_string());

        Self {
            messages: Vec::new(),
            input_buffer: String::new(),
            cursor_pos: 0,
            input_scroll_line: 0,
            input_wrap_width: Cell::new(80),
            scroll_offset: 0,
            chat_max_scroll: Cell::new(0),
            is_sidebar_visible: true,
            is_inspector_visible: true,
            is_input_focused: false,
            focus_area: AgentFocusArea::Chat,
            is_running: false,
            active_turn_id: None,
            active_model,
            available_models,
            expanded_tools: HashSet::new(),
            reasoning_expanded: false,
            auto_scroll: true,
            spinner_tick: 0,
            chat_visible_limit: crate::agent_view::ui::message_renderer::DEFAULT_CHAT_MESSAGE_LIMIT,
            history: Vec::new(),
            history_index: None,
            history_draft: String::new(),
            workspace_files: Vec::new(),
            selected_file_idx: 0,
            workspace_folders: Vec::new(),
            active_folder_idx: 0,
            add_folder_modal: AddFolderModalState::default(),
            expanded_dirs: HashSet::new(),
            tag_query: None,
            all_workspace_tag_items: Vec::new(),
            tool_filter: ToolFilter::All,
            pending_approval: None,
            sidebar_tab: AgentSidebarTab::Tasks,
            sessions: Vec::new(),
            selected_session_idx: 0,
            active_session_id: None,
            total_prompt_tokens: 0,
            total_completion_tokens: 0,
            max_context_limit: 128_000,
            session_start_instant: std::time::Instant::now(),
            projects: Vec::new(),
            selected_project_idx: 0,
            active_project_id: None,
            active_project_name: "No Project".to_string(),
            project_modal: ProjectSwitcherModalState::default(),
            create_project_modal: CreateProjectModalState::default(),
            model_modal: ModelSelectorModalState::default(),
            mcp_servers: Vec::new(),
            mcp_active_ids: Vec::new(),
            selected_mcp_idx: 0,
            chat_layout_cache: RefCell::new(ChatLayoutCache::default()),
        }
    }
}

pub const MAX_CHAT_MESSAGES: usize = 300;

mod method_input;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputVisualLine {
    pub start_byte: usize,
    pub end_byte: usize,
    pub is_paragraph_start: bool,
}

mod helpers;
mod fs_filter;
mod method_chat;
mod method_models;
mod method_selectors;
mod method_sessions;
mod workspace;

impl AgentWorkspaceState {
    pub fn push_message(&mut self, message: ChatMessage) {
        self.messages.push(message);
        self.enforce_message_limit();
    }

    pub fn set_messages(&mut self, messages: Vec<ChatMessage>) {
        self.messages = messages;
        self.chat_visible_limit = crate::agent_view::ui::message_renderer::DEFAULT_CHAT_MESSAGE_LIMIT;
        self.enforce_message_limit();
    }

    fn enforce_message_limit(&mut self) {
        if self.messages.len() > MAX_CHAT_MESSAGES {
            let overflow = self.messages.len() - MAX_CHAT_MESSAGES;
            self.messages.drain(0..overflow);
            self.expanded_tools.clear();
        }
    }

    pub fn sample() -> Self {
        Default::default()
    }

    pub fn set_messages_from_persisted(&mut self, list: Vec<PersistedMessageDto>) {
        self.messages = list
            .into_iter()
            .map(|dto| {
                let role = match dto.role.to_lowercase().as_str() {
                    "user" => MessageRole::User,
                    "assistant" => MessageRole::Assistant,
                    "system" => MessageRole::System,
                    _ => MessageRole::Tool,
                };

                let mut content = dto.content.unwrap_or_default();
                let mut reasoning = dto.thought;

                // Extract <think>...</think> if thought is missing
                if reasoning.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true)
                    && let Some(start_idx) = content.find("<think>")
                    && let Some(end_idx) = content[start_idx..].find("</think>")
                {
                    let think_content =
                        &content[start_idx + 7..start_idx + end_idx];
                    reasoning = Some(think_content.trim().to_string());
                    let after = &content[start_idx + end_idx + 8..];
                    content = format!("{}{}", &content[..start_idx], after).trim().to_string();
                }

                // Parse persisted tool calls if present
                let mut tool_calls = Vec::new();
                if let Some(tc_json) = dto.tool_calls.as_deref().filter(|s| !s.trim().is_empty())
                    && let Ok(v) = serde_json::from_str::<serde_json::Value>(tc_json)
                    && let Some(arr) = v.as_array()
                {
                    for item in arr {
                        let id = item.get("id").and_then(|x| x.as_str()).unwrap_or_default().to_string();
                        let name = item
                            .get("function")
                            .and_then(|f| f.get("name"))
                            .and_then(|x| x.as_str())
                            .or_else(|| item.get("name").and_then(|x| x.as_str()))
                            .unwrap_or("unknown_tool")
                            .to_string();

                        let args_summary = item
                            .get("function")
                            .and_then(|f| f.get("arguments"))
                            .map(|a| {
                                if let Some(s) = a.as_str() {
                                    s.to_string()
                                } else {
                                    a.to_string()
                                }
                            })
                            .or_else(|| {
                                item.get("args").map(|a| {
                                    if let Some(s) = a.as_str() {
                                        s.to_string()
                                    } else {
                                        a.to_string()
                                    }
                                })
                            })
                            .unwrap_or_default();

                        tool_calls.push(ToolCallInfo {
                            id,
                            name,
                            args_summary,
                            output_summary: None,
                            status: ToolExecutionStatus::Success,
                            duration_ms: None,
                            stdout: None,
                            stderr: None,
                            exit_code: None,
                            diff_preview: None,
                        });
                    }
                }

                ChatMessage {
                    role,
                    sender_name: "Agent".to_string(),
                    content,
                    timestamp: dto.created_at.to_string(),
                    tool_calls,
                    reasoning,
                }
            })
            .collect();
        self.chat_visible_limit = crate::agent_view::ui::message_renderer::DEFAULT_CHAT_MESSAGE_LIMIT;
        self.enforce_message_limit();
        self.scroll_offset = 0;
        self.auto_scroll = true;
    }

    pub fn clear_chat(&mut self) {
        self.messages.clear();
        self.chat_visible_limit = crate::agent_view::ui::message_renderer::DEFAULT_CHAT_MESSAGE_LIMIT;
        self.scroll_offset = 0;
        self.expanded_tools.clear();
        self.auto_scroll = true;
    }

    pub fn clear_input(&mut self) {
        self.input_buffer.clear();
        self.cursor_pos = 0;
        self.tag_query = None;
    }

    pub fn ensure_valid_cursor(&mut self) {
        if self.cursor_pos > self.input_buffer.len() {
            self.cursor_pos = self.input_buffer.len();
        }
        while !self.input_buffer.is_char_boundary(self.cursor_pos) && self.cursor_pos > 0 {
            self.cursor_pos -= 1;
        }
    }
}
