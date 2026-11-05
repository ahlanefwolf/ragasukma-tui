use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use ratatui::text::Line;

#[derive(Debug, Clone, Default)]
pub struct ChatLayoutCache {
    pub width: usize,
    pub message_count: usize,
    pub last_content_len: usize,
    pub is_running: bool,
    pub expanded_tools_len: usize,
    pub lines: Vec<Line<'static>>,
}

use super::types::{
    AddFolderModalState, AgentFocusArea, AgentSidebarTab, ApprovalRequest, ChatMessage,
    CreateProjectModalState, FileTagQuery, MessageRole, ModelSelectorModalState, ProjectItem,
    ProjectSwitcherModalState, TagItem, TaskSessionItem, TokenBreakdown, ToolFilter,
    WorkspaceFileNode, WorkspaceFolderItem,
};
use crate::core_bridge::PersistedMessageDto;

#[derive(Debug, Clone)]
pub struct AgentWorkspaceState {
    pub messages: Vec<ChatMessage>,
    pub input_buffer: String,
    pub cursor_pos: usize,
    pub scroll_offset: usize,
    pub chat_max_scroll: Cell<usize>,
    pub is_input_focused: bool,
    pub focus_area: AgentFocusArea,
    pub is_running: bool,
    pub active_turn_id: Option<String>,
    pub active_model: String,
    pub available_models: Vec<String>,
    pub expanded_tools: HashSet<usize>,
    pub expanded_tool_idx: Option<usize>,
    pub reasoning_expanded: bool,
    pub auto_scroll: bool,
    pub spinner_tick: usize,

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

    // Projects (Database SQLite migrations/0004)
    pub projects: Vec<ProjectItem>,
    pub selected_project_idx: usize,
    pub active_project_id: Option<String>,
    pub active_project_name: String,
    pub project_modal: ProjectSwitcherModalState,
    pub create_project_modal: CreateProjectModalState,
    pub model_modal: ModelSelectorModalState,

    // Phase 6: Responsive Layout & Inspection
    pub is_sidebar_visible: bool,
    pub is_inspector_visible: bool,
    pub inspector_scroll_offset: usize,

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
            scroll_offset: 0,
            chat_max_scroll: Cell::new(0),
            inspector_scroll_offset: 0,
            is_sidebar_visible: true,
            is_inspector_visible: true,
            is_input_focused: false,
            focus_area: AgentFocusArea::Chat,
            is_running: false,
            active_turn_id: None,
            active_model,
            available_models,
            expanded_tools: HashSet::new(),
            expanded_tool_idx: None,
            reasoning_expanded: false,
            auto_scroll: true,
            spinner_tick: 0,
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
            projects: Vec::new(),
            selected_project_idx: 0,
            active_project_id: None,
            active_project_name: "No Project".to_string(),
            project_modal: ProjectSwitcherModalState::default(),
            create_project_modal: CreateProjectModalState::default(),
            model_modal: ModelSelectorModalState::default(),
            chat_layout_cache: RefCell::new(ChatLayoutCache::default()),
        }
    }
}

impl AgentWorkspaceState {
    pub fn sample() -> Self {
        Self::default()
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
                ChatMessage {
                    role,
                    sender_name: "Agent".to_string(),
                    content: dto.content.unwrap_or_default(),
                    timestamp: dto.created_at.to_string(),
                    tool_calls: Vec::new(),
                    reasoning: dto.thought,
                }
            })
            .collect();
        self.scroll_offset = 0;
        self.auto_scroll = true;
    }

    pub fn clear_chat(&mut self) {
        self.messages.clear();
        self.scroll_offset = 0;
        self.expanded_tools.clear();
        self.expanded_tool_idx = None;
        self.auto_scroll = true;
    }

    pub fn clear_input(&mut self) {
        self.input_buffer.clear();
        self.cursor_pos = 0;
        self.tag_query = None;
    }

    pub fn insert_char(&mut self, c: char) {
        let prefix = &self.input_buffer[..self.cursor_pos];
        let suffix = &self.input_buffer[self.cursor_pos..];
        self.input_buffer = format!("{}{}{}", prefix, c, suffix);
        self.cursor_pos += c.len_utf8();
        self.check_and_update_tag_query();
    }

    pub fn backspace(&mut self) {
        if self.cursor_pos > 0 {
            let prev = self.input_buffer[..self.cursor_pos]
                .chars()
                .last()
                .map(|c| c.len_utf8())
                .unwrap_or(1);
            let start = self.cursor_pos - prev;
            let end = self.cursor_pos;
            self.input_buffer.replace_range(start..end, "");
            self.cursor_pos = start;
            self.check_and_update_tag_query();
        }
    }

    pub fn append_chunk(&mut self, chunk: &str) {
        self.auto_scroll = true;
        if let Some(last) = self
            .messages
            .last_mut()
            .filter(|m| m.role == MessageRole::Assistant)
        {
            last.content.push_str(chunk);
            return;
        }

        self.messages.push(ChatMessage {
            role: MessageRole::Assistant,
            sender_name: "Agent".to_string(),
            content: chunk.to_string(),
            timestamp: "now".to_string(),
            tool_calls: Vec::new(),
            reasoning: None,
        });
    }

    pub fn append_thought(&mut self, thought: &str) {
        self.auto_scroll = true;
        if let Some(last) = self
            .messages
            .last_mut()
            .filter(|m| m.role == MessageRole::Assistant)
        {
            if let Some(ref mut r) = last.reasoning {
                r.push_str(thought);
            } else {
                last.reasoning = Some(thought.to_string());
            }
        }
    }

    pub fn finish_turn(&mut self) {
        self.is_running = false;
        self.active_turn_id = None;
        if let Some(active_sid) = &self.active_session_id
            && let Some(s) = self.sessions.iter_mut().find(|s| &s.id == active_sid) {
                s.is_running = false;
            }
    }

    pub fn fail_turn(&mut self, _err: String) {
        self.is_running = false;
        self.active_turn_id = None;
        if let Some(active_sid) = &self.active_session_id
            && let Some(s) = self.sessions.iter_mut().find(|s| &s.id == active_sid) {
                s.is_running = false;
            }
    }

    pub fn submit_prompt(&mut self) -> Option<String> {
        let trimmed = self.input_buffer.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            self.input_buffer.clear();
            self.cursor_pos = 0;
            self.tag_query = None;
            self.scroll_offset = 0;
            self.auto_scroll = true;
            Some(trimmed)
        }
    }

    pub fn focus_input(&mut self) {
        self.is_input_focused = true;
        self.focus_area = AgentFocusArea::Input;
    }

    pub fn unfocus_input(&mut self) {
        self.is_input_focused = false;
        if self.focus_area == AgentFocusArea::Input {
            self.focus_area = AgentFocusArea::Chat;
        }
    }

    pub fn cycle_focus_next(&mut self) {
        let mut order = Vec::new();
        if self.is_sidebar_visible {
            match self.sidebar_tab {
                AgentSidebarTab::Tasks => order.push(AgentFocusArea::Tasks),
                AgentSidebarTab::Files => order.push(AgentFocusArea::Files),
            }
            order.push(AgentFocusArea::Folders);
        }
        order.push(AgentFocusArea::Chat);
        order.push(AgentFocusArea::Input);
        if self.is_inspector_visible {
            order.push(AgentFocusArea::ModelTelemetry);
            order.push(AgentFocusArea::McpTools);
        }

        if let Some(pos) = order.iter().position(|&f| f == self.focus_area) {
            let next_pos = (pos + 1) % order.len();
            self.focus_area = order[next_pos];
        } else {
            self.focus_area = order.first().copied().unwrap_or(AgentFocusArea::Chat);
        }

        self.is_input_focused = self.focus_area == AgentFocusArea::Input;
    }

    pub fn cycle_focus_prev(&mut self) {
        let mut order = Vec::new();
        if self.is_sidebar_visible {
            match self.sidebar_tab {
                AgentSidebarTab::Tasks => order.push(AgentFocusArea::Tasks),
                AgentSidebarTab::Files => order.push(AgentFocusArea::Files),
            }
            order.push(AgentFocusArea::Folders);
        }
        order.push(AgentFocusArea::Chat);
        order.push(AgentFocusArea::Input);
        if self.is_inspector_visible {
            order.push(AgentFocusArea::ModelTelemetry);
            order.push(AgentFocusArea::McpTools);
        }

        if let Some(pos) = order.iter().position(|&f| f == self.focus_area) {
            let prev_pos = if pos == 0 { order.len() - 1 } else { pos - 1 };
            self.focus_area = order[prev_pos];
        } else {
            self.focus_area = order.first().copied().unwrap_or(AgentFocusArea::Chat);
        }

        self.is_input_focused = self.focus_area == AgentFocusArea::Input;
    }

    pub fn scroll_up(&mut self) {
        if self.focus_area == AgentFocusArea::McpTools {
            self.inspector_scroll_offset = self.inspector_scroll_offset.saturating_sub(1);
        } else {
            let max = self.chat_max_scroll.get();
            self.scroll_offset = (self.scroll_offset.saturating_add(1)).min(max);
            self.auto_scroll = false;
        }
    }

    pub fn scroll_down(&mut self) {
        if self.focus_area == AgentFocusArea::McpTools {
            self.inspector_scroll_offset = self.inspector_scroll_offset.saturating_add(1);
        } else {
            let max = self.chat_max_scroll.get();
            let current = self.scroll_offset.min(max);
            self.scroll_offset = current.saturating_sub(1);
            if self.scroll_offset == 0 {
                self.auto_scroll = true;
            }
        }
    }

    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = self.chat_max_scroll.get();
        self.auto_scroll = false;
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
        self.auto_scroll = true;
    }

    pub fn toggle_tool_expand(&mut self) {
        if self.expanded_tool_idx.is_some() {
            self.expanded_tool_idx = None;
        } else {
            self.expanded_tool_idx = Some(0);
        }
    }

    pub fn toggle_tool_card(&mut self, idx: usize) {
        if self.expanded_tools.contains(&idx) {
            self.expanded_tools.remove(&idx);
        } else {
            self.expanded_tools.insert(idx);
        }
    }

    pub fn toggle_reasoning(&mut self) {
        self.reasoning_expanded = !self.reasoning_expanded;
    }

    pub fn toggle_sidebar(&mut self) {
        self.is_sidebar_visible = !self.is_sidebar_visible;
        if !self.is_sidebar_visible && self.focus_area == AgentFocusArea::Files {
            self.focus_area = AgentFocusArea::Chat;
        }
    }

    pub fn refresh_available_models(&mut self) -> usize {
        let fetched = crate::agent_view::models::fetch_core_available_models();
        let count = fetched.len();
        if !fetched.is_empty() {
            if !fetched.contains(&self.active_model)
                && let Some(first) = fetched.first()
            {
                self.active_model = first.clone();
            }
            self.available_models = fetched;
        }
        count
    }

    pub fn cycle_model(&mut self) {
        if self.available_models.is_empty() {
            return;
        }
        if let Some(pos) = self.available_models.iter().position(|m| m == &self.active_model) {
            let next = (pos + 1) % self.available_models.len();
            self.active_model = self.available_models[next].clone();
        } else {
            self.active_model = self.available_models[0].clone();
        }
    }

    pub fn filtered_modal_models(&self) -> Vec<String> {
        let q = self.model_modal.search_query.trim().to_lowercase();
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

    pub fn open_model_modal(&mut self) {
        self.model_modal.is_open = true;
        self.model_modal.search_query.clear();
        let filtered = self.filtered_modal_models();
        if let Some(idx) = filtered.iter().position(|m| m == &self.active_model) {
            self.model_modal.selected_idx = idx;
        } else {
            self.model_modal.selected_idx = 0;
        }
    }

    pub fn close_model_modal(&mut self) {
        self.model_modal.is_open = false;
        self.model_modal.search_query.clear();
    }

    pub fn model_modal_push_char(&mut self, c: char) {
        self.model_modal.search_query.push(c);
        let filtered = self.filtered_modal_models();
        if self.model_modal.selected_idx >= filtered.len() {
            self.model_modal.selected_idx = 0;
        }
    }

    pub fn model_modal_backspace(&mut self) {
        self.model_modal.search_query.pop();
        let filtered = self.filtered_modal_models();
        if self.model_modal.selected_idx >= filtered.len() {
            self.model_modal.selected_idx = 0;
        }
    }

    pub fn select_next_modal_model(&mut self) {
        let filtered = self.filtered_modal_models();
        if !filtered.is_empty() {
            self.model_modal.selected_idx =
                (self.model_modal.selected_idx + 1) % filtered.len();
        }
    }

    pub fn select_prev_modal_model(&mut self) {
        let filtered = self.filtered_modal_models();
        if !filtered.is_empty() {
            if self.model_modal.selected_idx == 0 {
                self.model_modal.selected_idx = filtered.len() - 1;
            } else {
                self.model_modal.selected_idx -= 1;
            }
        }
    }

    pub fn apply_modal_model(&mut self) {
        let filtered = self.filtered_modal_models();
        if let Some(m) = filtered.get(self.model_modal.selected_idx) {
            self.active_model = m.clone();
        }
        self.model_modal.is_open = false;
        self.model_modal.search_query.clear();
    }

    pub fn start_turn(&mut self) {
        self.is_running = true;
        self.scroll_offset = 0;
        self.auto_scroll = true;
        if let Some(active_sid) = &self.active_session_id
            && let Some(s) = self.sessions.iter_mut().find(|s| &s.id == active_sid) {
                s.is_running = true;
            }
    }

    pub fn cancel_turn(&mut self) {
        self.is_running = false;
        self.active_turn_id = None;
        if let Some(active_sid) = &self.active_session_id
            && let Some(s) = self.sessions.iter_mut().find(|s| &s.id == active_sid) {
                s.is_running = false;
            }
    }

    pub fn tick_spinner(&mut self) {
        self.spinner_tick = (self.spinner_tick + 1) % 10;
    }

    pub fn spinner_char(&self) -> &'static str {
        const FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        FRAMES[self.spinner_tick % FRAMES.len()]
    }

    pub fn check_and_update_tag_query(&mut self) {
        if let Some(at_pos) = self.input_buffer[..self.cursor_pos].rfind('@') {
            let slice_between = &self.input_buffer[at_pos + 1..self.cursor_pos];
            // If user typed space after '@', tag query autocomplete has ended
            if slice_between.contains(' ') || slice_between.contains('\t') {
                self.tag_query = None;
                return;
            }
            let query = slice_between.to_string();
            let query_lower = query.to_lowercase();

            if self.all_workspace_tag_items.is_empty() && !self.workspace_folders.is_empty() {
                self.refresh_workspace_tag_items();
            }

            let mut all_matches: Vec<TagItem> = self
                .all_workspace_tag_items
                .iter()
                .filter(|item| {
                    if query_lower.is_empty() {
                        true
                    } else {
                        item.name.to_lowercase().contains(&query_lower)
                            || item.path.to_lowercase().contains(&query_lower)
                    }
                })
                .cloned()
                .collect();

            // Prioritize items starting with query, then shorter paths
            all_matches.sort_by(|a, b| {
                let a_starts = a.name.to_lowercase().starts_with(&query_lower);
                let b_starts = b.name.to_lowercase().starts_with(&query_lower);
                match (a_starts, b_starts) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.path.len().cmp(&b.path.len()).then_with(|| a.name.cmp(&b.name)),
                }
            });

            let prev_selected = self.tag_query.as_ref().map(|tq| tq.selected_idx).unwrap_or(0);
            let mut tq = FileTagQuery::new(query, all_matches, at_pos);
            if !tq.matched_items.is_empty() {
                tq.selected_idx = prev_selected.min(tq.matched_items.len() - 1);
            }

            self.tag_query = Some(tq);
        } else {
            self.tag_query = None;
        }
    }

    pub fn select_tag_query_next(&mut self) {
        if let Some(ref mut tq) = self.tag_query {
            if tq.matched_items.is_empty() {
                return;
            }
            // Lazy load next batch when scrolling near or at the end
            if tq.selected_idx + 1 >= tq.matched_items.len() && tq.has_more() {
                tq.load_next_batch();
            }
            if tq.selected_idx + 1 < tq.matched_items.len() {
                tq.selected_idx += 1;
            }
        }
    }

    pub fn select_tag_query_prev(&mut self) {
        if let Some(ref mut tq) = self.tag_query
            && tq.selected_idx > 0
        {
            tq.selected_idx -= 1;
        }
    }

    pub fn apply_selected_tag(&mut self) {
        if let Some(ref tq) = self.tag_query
            && !tq.matched_items.is_empty()
        {
            let item = &tq.matched_items[tq.selected_idx.min(tq.matched_items.len() - 1)];
            let tag_insert = format!("@{} ", item.path);

            let start = tq.tag_start_idx;
            let end = self.cursor_pos;
            if start <= end && end <= self.input_buffer.len() {
                let before = &self.input_buffer[..start];
                let after = &self.input_buffer[end..];
                self.input_buffer = format!("{}{}{}", before, tag_insert, after);
                self.cursor_pos = start + tag_insert.len();
            }
        }
        self.tag_query = None;
    }

    pub fn close_tag_query(&mut self) {
        self.tag_query = None;
    }

    pub fn switch_sidebar_tab(&mut self) {
        self.sidebar_tab = match self.sidebar_tab {
            AgentSidebarTab::Tasks => AgentSidebarTab::Files,
            AgentSidebarTab::Files => AgentSidebarTab::Tasks,
        };
    }

    pub fn next_session(&mut self) {
        if !self.sessions.is_empty() {
            self.selected_session_idx = (self.selected_session_idx + 1) % self.sessions.len();
            self.active_session_id = self.sessions.get(self.selected_session_idx).map(|s| s.id.clone());
        }
    }

    pub fn prev_session(&mut self) {
        if !self.sessions.is_empty() {
            if self.selected_session_idx == 0 {
                self.selected_session_idx = self.sessions.len() - 1;
            } else {
                self.selected_session_idx -= 1;
            }
            self.active_session_id = self.sessions.get(self.selected_session_idx).map(|s| s.id.clone());
        }
    }

    pub fn create_new_task(&mut self, title: Option<String>) {
        let task_num = self.sessions.len() + 1;
        let id = format!("task-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis());
        let new_task = TaskSessionItem {
            id: id.clone(),
            title: title.unwrap_or_else(|| format!("Task #{}: New Agent Task", task_num)),
            status: "ready".to_string(),
            model: Some(self.active_model.clone()),
            total_tokens_input: 0,
            total_tokens_output: 0,
            updated_at: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs() as i64,
            is_running: false,
        };
        self.sessions.push(new_task);
        self.selected_session_idx = self.sessions.len() - 1;
        self.active_session_id = Some(id);
    }

    pub fn delete_selected_task(&mut self) {
        if !self.sessions.is_empty() && self.selected_session_idx < self.sessions.len() {
            self.sessions.remove(self.selected_session_idx);
            if self.selected_session_idx >= self.sessions.len() && !self.sessions.is_empty() {
                self.selected_session_idx = self.sessions.len() - 1;
            }
            self.active_session_id = self.sessions.get(self.selected_session_idx).map(|s| s.id.clone());
        }
    }

    pub fn calculate_token_breakdown(&self) -> TokenBreakdown {
        let chat_tokens = self.messages.iter().map(|m| m.content.len() / 4).sum();
        let session_tokens = self.sessions.iter().map(|s| (s.total_tokens_input + s.total_tokens_output) as usize).sum();

        let mut user_prompts = 0;
        let mut assistant_turns = 0;
        let mut msg_in_tokens = 0;
        let mut msg_out_tokens = 0;

        for msg in &self.messages {
            let tok = (msg.content.len() / 4).max(1);
            match msg.role {
                crate::agent_view::types::MessageRole::User => {
                    user_prompts += 1;
                    msg_in_tokens += tok;
                }
                crate::agent_view::types::MessageRole::Assistant => {
                    assistant_turns += 1;
                    msg_out_tokens += tok;
                }
                crate::agent_view::types::MessageRole::Tool => {
                    // Tool output diumpankan kembali sebagai input context ke pemanggilan model berikutnya
                    msg_in_tokens += tok;
                }
                _ => {}
            }
        }

        // Hitung total LLM interaction turns (request ke model):
        // Setiap respon assistant merepresentasikan 1 kali siklus panggilan LLM API yang selesai dieksekusi.
        // Jika request sedang aktif atau user baru mengirim prompt dan belum ada respon assistant,
        // minimal prompt user dihitung sebagai request berjalan.
        let task_requests = if assistant_turns > 0 {
            assistant_turns
        } else {
            user_prompts
        };

        let (stored_in, stored_out) = if let Some(s) = self.sessions.get(self.selected_session_idx) {
            (s.total_tokens_input as usize, s.total_tokens_output as usize)
        } else {
            (0, 0)
        };

        let task_input_tokens = stored_in.max(msg_in_tokens);
        let task_output_tokens = stored_out.max(msg_out_tokens);

        TokenBreakdown {
            system_tokens: 1500,
            chat_history_tokens: chat_tokens,
            session_tokens,
            completion_tokens: self.total_completion_tokens,
            max_context_limit: self.max_context_limit,
            task_input_tokens,
            task_output_tokens,
            task_requests,
        }
    }

    pub fn cycle_tool_filter(&mut self) {
        self.tool_filter = match self.tool_filter {
            ToolFilter::All => ToolFilter::RunningOnly,
            ToolFilter::RunningOnly => ToolFilter::FailedOnly,
            ToolFilter::FailedOnly => ToolFilter::All,
        };
    }

    pub fn approve_pending(&mut self) -> Option<ApprovalRequest> {
        self.pending_approval.take()
    }

    pub fn reject_pending(&mut self) -> Option<ApprovalRequest> {
        self.pending_approval.take()
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

    pub fn toggle_file_tree_expand(&mut self) {
        if let Some(file) = self.workspace_files.get(self.selected_file_idx).filter(|f| f.is_dir) {
            let path = file.path.clone();
            if self.expanded_dirs.contains(&path) {
                self.expanded_dirs.remove(&path);
            } else {
                self.expanded_dirs.insert(path);
            }
            self.scan_workspace_files();
        }
    }

    pub fn insert_selected_file_to_input(&mut self) {
        if let Some(file) = self.workspace_files.get(self.selected_file_idx) {
            let insertion = format!("@{} ", file.path);
            let prefix = &self.input_buffer[..self.cursor_pos];
            let suffix = &self.input_buffer[self.cursor_pos..];
            let new_cursor = prefix.len() + insertion.len();
            self.input_buffer = format!("{}{}{}", prefix, insertion, suffix);
            self.cursor_pos = new_cursor;
        }
    }

    pub fn get_workspace_folders_list(&self) -> Vec<WorkspaceFolderItem> {
        self.workspace_folders
            .iter()
            .map(|p| {
                let name = p
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| p.to_string_lossy().to_string());
                WorkspaceFolderItem {
                    path: p.clone(),
                    name,
                    is_valid: p.is_dir(),
                }
            })
            .collect()
    }

    pub fn add_workspace_folder(&mut self, path: PathBuf) -> Result<(), String> {
        let expanded = match path.strip_prefix("~") {
            Ok(rest) => {
                let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"));
                if let Some(home_path) = home {
                    PathBuf::from(home_path).join(rest)
                } else {
                    path.clone()
                }
            }
            Err(_) => path.clone(),
        };

        if !expanded.is_dir() {
            return Err(format!("Directory not found: {}", expanded.display()));
        }

        let canonical = match expanded.canonicalize() {
            Ok(c) => c,
            Err(_) => expanded,
        };

        if self.workspace_folders.iter().any(|f| {
            if let Ok(c) = f.canonicalize() {
                c == canonical
            } else {
                f == &canonical
            }
        }) {
            return Err("Folder is already in workspace".to_string());
        }

        let folder_str = canonical.to_string_lossy().to_string();
        self.expanded_dirs.insert(folder_str);
        self.workspace_folders.push(canonical);
        self.active_folder_idx = self.workspace_folders.len() - 1;
        self.scan_workspace_files();
        Ok(())
    }

    pub fn remove_workspace_folder(&mut self, idx: usize) -> Result<(), String> {
        if self.workspace_folders.len() <= 1 {
            return Err("Cannot remove the only workspace folder".to_string());
        }
        if idx < self.workspace_folders.len() {
            let removed = self.workspace_folders.remove(idx);
            self.expanded_dirs.remove(&removed.to_string_lossy().to_string());
            if self.active_folder_idx >= self.workspace_folders.len() {
                self.active_folder_idx = self.workspace_folders.len().saturating_sub(1);
            }
            self.scan_workspace_files();
            Ok(())
        } else {
            Err("Invalid folder index".to_string())
        }
    }

    pub fn select_next_folder(&mut self) {
        if !self.workspace_folders.is_empty() {
            self.active_folder_idx = (self.active_folder_idx + 1) % self.workspace_folders.len();
        }
    }

    pub fn select_prev_folder(&mut self) {
        if !self.workspace_folders.is_empty() {
            if self.active_folder_idx == 0 {
                self.active_folder_idx = self.workspace_folders.len() - 1;
            } else {
                self.active_folder_idx -= 1;
            }
        }
    }

    pub fn open_add_folder_modal(&mut self) {
        self.add_folder_modal.is_open = true;
        self.add_folder_modal.input_path.clear();
        self.add_folder_modal.validation_message = None;
    }

    pub fn close_add_folder_modal(&mut self) {
        self.add_folder_modal.is_open = false;
        self.add_folder_modal.input_path.clear();
        self.add_folder_modal.validation_message = None;
    }

    pub fn submit_add_folder(&mut self) -> Result<(), String> {
        let input = self.add_folder_modal.input_path.trim().to_string();
        if input.is_empty() {
            self.add_folder_modal.validation_message =
                Some((false, "Path cannot be empty".to_string()));
            return Err("Path cannot be empty".to_string());
        }
        let path = PathBuf::from(&input);
        match self.add_workspace_folder(path) {
            Ok(()) => {
                self.close_add_folder_modal();
                Ok(())
            }
            Err(err) => {
                self.add_folder_modal.validation_message = Some((false, err.clone()));
                Err(err)
            }
        }
    }

    pub fn scan_workspace_files(&mut self) {
        if self.workspace_folders.is_empty() {
            self.workspace_files.clear();
            self.selected_file_idx = 0;
            return;
        }

        let mut nodes = Vec::new();
        for folder in &self.workspace_folders {
            let folder_str = folder.to_string_lossy().to_string();
            let name = folder
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| folder_str.clone());
            let is_expanded = self.expanded_dirs.contains(&folder_str);
            nodes.push(WorkspaceFileNode {
                path: folder_str.clone(),
                name,
                is_dir: true,
                depth: 0,
                is_expanded,
                parent_path: None,
            });
            if is_expanded {
                collect_visible_children(
                    folder,
                    1,
                    &self.expanded_dirs,
                    &mut nodes,
                    Some(folder_str),
                );
            }
        }

        self.workspace_files = nodes;
        if self.selected_file_idx >= self.workspace_files.len() && !self.workspace_files.is_empty() {
            self.selected_file_idx = self.workspace_files.len() - 1;
        }
        self.refresh_workspace_tag_items();
    }

    pub fn refresh_workspace_tag_items(&mut self) {
        if self.workspace_folders.is_empty() {
            self.all_workspace_tag_items.clear();
            return;
        }

        let is_multi_root = self.workspace_folders.len() > 1;
        let mut items = Vec::new();

        for folder in &self.workspace_folders {
            let root_name = folder
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| folder.to_string_lossy().to_string());

            if is_multi_root {
                items.push(TagItem {
                    path: format!("{}/", root_name),
                    name: root_name.clone(),
                    is_dir: true,
                });
            }

            collect_recursive_tag_items(
                folder,
                folder,
                1,
                is_multi_root,
                &root_name,
                &mut items,
            );
        }

        self.all_workspace_tag_items = items;
    }
}

fn collect_recursive_tag_items(
    root: &Path,
    current: &Path,
    depth: usize,
    is_multi_root: bool,
    root_name: &str,
    out: &mut Vec<TagItem>,
) {
    if depth > 12 {
        return;
    }

    let entries = match std::fs::read_dir(current) {
        Ok(e) => e,
        Err(_) => return,
    };

    let mut dirs = Vec::new();
    let mut files = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if name.starts_with('.')
            || name == "target"
            || name == "node_modules"
            || name == ".git"
            || name == ".cargo"
        {
            continue;
        }

        if path.is_dir() {
            dirs.push((name, path));
        } else {
            files.push((name, path));
        }
    }

    dirs.sort_by(|a, b| a.0.cmp(&b.0));
    files.sort_by(|a, b| a.0.cmp(&b.0));

    for (name, path) in dirs {
        let rel_path = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let clean_path = if is_multi_root {
            format!("{}/{}/", root_name, rel_path)
        } else {
            format!("{}/", rel_path)
        };

        out.push(TagItem {
            path: clean_path,
            name,
            is_dir: true,
        });

        collect_recursive_tag_items(root, &path, depth + 1, is_multi_root, root_name, out);
    }

    for (name, path) in files {
        let rel_path = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let clean_path = if is_multi_root {
            format!("{}/{}", root_name, rel_path)
        } else {
            rel_path
        };

        out.push(TagItem {
            path: clean_path,
            name,
            is_dir: false,
        });
    }
}

fn collect_visible_children(
    dir: &Path,
    depth: usize,
    expanded_dirs: &HashSet<String>,
    nodes: &mut Vec<WorkspaceFileNode>,
    parent_path: Option<String>,
) {
    if depth > 5 {
        return;
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(read_dir) => read_dir,
        Err(_) => return,
    };

    let mut dirs = Vec::new();
    let mut files = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if name.starts_with('.') || name == "target" || name == "node_modules" {
            continue;
        }

        if path.is_dir() {
            dirs.push((name, path));
        } else {
            files.push((name, path));
        }
    }

    dirs.sort_by(|a, b| a.0.cmp(&b.0));
    files.sort_by(|a, b| a.0.cmp(&b.0));

    for (name, path) in dirs {
        let path_str = path.to_string_lossy().to_string();
        let is_expanded = expanded_dirs.contains(&path_str);
        nodes.push(WorkspaceFileNode {
            path: path_str.clone(),
            name,
            is_dir: true,
            depth,
            is_expanded,
            parent_path: parent_path.clone(),
        });

        if is_expanded {
            collect_visible_children(&path, depth + 1, expanded_dirs, nodes, Some(path_str));
        }
    }

    for (name, path) in files {
        nodes.push(WorkspaceFileNode {
            path: path.to_string_lossy().to_string(),
            name,
            is_dir: false,
            depth,
            is_expanded: false,
            parent_path: parent_path.clone(),
        });
    }
}