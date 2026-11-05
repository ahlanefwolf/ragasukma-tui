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
    pub session_start_instant: std::time::Instant,

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
            input_scroll_line: 0,
            input_wrap_width: Cell::new(80),
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
            session_start_instant: std::time::Instant::now(),
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

pub const MAX_CHAT_MESSAGES: usize = 300;

impl AgentWorkspaceState {
    pub fn push_message(&mut self, message: ChatMessage) {
        self.messages.push(message);
        self.enforce_message_limit();
    }

    pub fn set_messages(&mut self, messages: Vec<ChatMessage>) {
        self.messages = messages;
        self.enforce_message_limit();
    }

    fn enforce_message_limit(&mut self) {
        if self.messages.len() > MAX_CHAT_MESSAGES {
            let overflow = self.messages.len() - MAX_CHAT_MESSAGES;
            self.messages.drain(0..overflow);
            self.expanded_tools.clear();
            self.expanded_tool_idx = None;
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
        self.enforce_message_limit();
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

    pub fn ensure_valid_cursor(&mut self) {
        if self.cursor_pos > self.input_buffer.len() {
            self.cursor_pos = self.input_buffer.len();
        }
        while !self.input_buffer.is_char_boundary(self.cursor_pos) && self.cursor_pos > 0 {
            self.cursor_pos -= 1;
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputVisualLine {
    pub start_byte: usize,
    pub end_byte: usize,
    pub is_paragraph_start: bool,
}

impl AgentWorkspaceState {
    pub fn compute_visual_lines(&self, wrap_width: usize) -> Vec<InputVisualLine> {
        let wrap = wrap_width.max(5);
        if self.input_buffer.is_empty() {
            return vec![InputVisualLine {
                start_byte: 0,
                end_byte: 0,
                is_paragraph_start: true,
            }];
        }

        let mut vlines = Vec::new();
        let mut phys_start = 0;
        let total_len = self.input_buffer.len();

        while phys_start <= total_len {
            let phys_end = match self.input_buffer[phys_start..].find('\n') {
                Some(idx) => phys_start + idx,
                None => total_len,
            };

            let phys_str = &self.input_buffer[phys_start..phys_end];
            let char_count = phys_str.chars().count();

            if char_count == 0 {
                vlines.push(InputVisualLine {
                    start_byte: phys_start,
                    end_byte: phys_end,
                    is_paragraph_start: true,
                });
            } else {
                let mut seg_start_byte = phys_start;
                let mut is_first = true;
                let mut current_seg_chars = 0;

                for (b_offset, _) in phys_str.char_indices() {
                    if current_seg_chars == wrap {
                        let seg_end_byte = phys_start + b_offset;
                        vlines.push(InputVisualLine {
                            start_byte: seg_start_byte,
                            end_byte: seg_end_byte,
                            is_paragraph_start: is_first,
                        });
                        is_first = false;
                        seg_start_byte = seg_end_byte;
                        current_seg_chars = 0;
                    }
                    current_seg_chars += 1;
                }

                // Push last segment of this physical line
                vlines.push(InputVisualLine {
                    start_byte: seg_start_byte,
                    end_byte: phys_end,
                    is_paragraph_start: is_first,
                });

                // If this physical line filled the width completely and cursor is at end,
                // wrap automatically to a new visual line below
                if char_count > 0 && char_count.is_multiple_of(wrap) && self.cursor_pos == phys_end {
                    vlines.push(InputVisualLine {
                        start_byte: phys_end,
                        end_byte: phys_end,
                        is_paragraph_start: false,
                    });
                }
            }

            if phys_end == total_len {
                break;
            }
            phys_start = phys_end + 1; // skip '\n'
        }

        if vlines.is_empty() {
            vlines.push(InputVisualLine {
                start_byte: 0,
                end_byte: 0,
                is_paragraph_start: true,
            });
        }

        vlines
    }

    pub fn get_cursor_visual_position(&self, wrap_width: usize) -> (usize, usize) {
        let vlines = self.compute_visual_lines(wrap_width);
        if vlines.is_empty() {
            return (0, 0);
        }
        let safe_pos = self.cursor_pos.min(self.input_buffer.len());

        for (idx, vl) in vlines.iter().enumerate() {
            // When cursor is at the right edge boundary of a wrapped line,
            // automatically prefer the start of the next line below (wrap downwards)
            if safe_pos == vl.end_byte
                && idx + 1 < vlines.len()
                && vlines[idx + 1].start_byte == vl.end_byte
            {
                continue;
            }

            if safe_pos >= vl.start_byte && safe_pos <= vl.end_byte {
                let col = self.input_buffer[vl.start_byte..safe_pos].chars().count();
                return (idx, col);
            }
        }

        let last_idx = vlines.len() - 1;
        let last_vl = &vlines[last_idx];
        let col = self.input_buffer[last_vl.start_byte..safe_pos.max(last_vl.start_byte)]
            .chars()
            .count();
        (last_idx, col)
    }

    pub fn get_cursor_row_col(&self) -> (usize, usize) {
        let wrap = self.input_wrap_width.get().max(5);
        self.get_cursor_visual_position(wrap)
    }

    pub fn adjust_input_scroll(&mut self) {
        let (row, _) = self.get_cursor_row_col();
        if row < self.input_scroll_line {
            self.input_scroll_line = row;
        } else if row >= self.input_scroll_line + 3 {
            self.input_scroll_line = row - 3 + 1;
        }
    }

    pub fn cursor_left(&mut self) {
        self.ensure_valid_cursor();
        if self.cursor_pos > 0 {
            let prev_len = self.input_buffer[..self.cursor_pos]
                .chars()
                .last()
                .map(|c| c.len_utf8())
                .unwrap_or(1);
            self.cursor_pos = self.cursor_pos.saturating_sub(prev_len);
            self.ensure_valid_cursor();
            self.adjust_input_scroll();
            self.check_and_update_tag_query();
        }
    }

    pub fn cursor_right(&mut self) {
        self.ensure_valid_cursor();
        if self.cursor_pos < self.input_buffer.len() {
            let next_len = self.input_buffer[self.cursor_pos..]
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(1);
            self.cursor_pos = (self.cursor_pos + next_len).min(self.input_buffer.len());
            self.ensure_valid_cursor();
            self.adjust_input_scroll();
            self.check_and_update_tag_query();
        }
    }

    pub fn cursor_home(&mut self) {
        self.ensure_valid_cursor();
        let wrap = self.input_wrap_width.get().max(5);
        let vlines = self.compute_visual_lines(wrap);
        let (row, _) = self.get_cursor_visual_position(wrap);
        if let Some(vl) = vlines.get(row) {
            self.cursor_pos = vl.start_byte;
        }
        self.ensure_valid_cursor();
        self.adjust_input_scroll();
        self.check_and_update_tag_query();
    }

    pub fn cursor_end(&mut self) {
        self.ensure_valid_cursor();
        let wrap = self.input_wrap_width.get().max(5);
        let vlines = self.compute_visual_lines(wrap);
        let (row, _) = self.get_cursor_visual_position(wrap);
        if let Some(vl) = vlines.get(row) {
            self.cursor_pos = vl.end_byte;
        }
        self.ensure_valid_cursor();
        self.adjust_input_scroll();
        self.check_and_update_tag_query();
    }

    pub fn cursor_up(&mut self) {
        self.ensure_valid_cursor();
        let wrap = self.input_wrap_width.get().max(5);
        let vlines = self.compute_visual_lines(wrap);
        let (row, col) = self.get_cursor_visual_position(wrap);
        if row == 0 {
            self.cursor_home();
            return;
        }

        if let Some(prev_vl) = vlines.get(row - 1) {
            let prev_str = &self.input_buffer[prev_vl.start_byte..prev_vl.end_byte];
            let prev_char_count = prev_str.chars().count();
            let target_col = col.min(prev_char_count);
            let col_byte_offset: usize = prev_str
                .chars()
                .take(target_col)
                .map(|c| c.len_utf8())
                .sum();
            self.cursor_pos = prev_vl.start_byte + col_byte_offset;
            self.ensure_valid_cursor();
            self.adjust_input_scroll();
            self.check_and_update_tag_query();
        }
    }

    pub fn cursor_down(&mut self) {
        self.ensure_valid_cursor();
        let wrap = self.input_wrap_width.get().max(5);
        let vlines = self.compute_visual_lines(wrap);
        let (row, col) = self.get_cursor_visual_position(wrap);
        if row + 1 >= vlines.len() {
            self.cursor_end();
            return;
        }

        if let Some(next_vl) = vlines.get(row + 1) {
            let next_str = &self.input_buffer[next_vl.start_byte..next_vl.end_byte];
            let next_char_count = next_str.chars().count();
            let target_col = col.min(next_char_count);
            let col_byte_offset: usize = next_str
                .chars()
                .take(target_col)
                .map(|c| c.len_utf8())
                .sum();
            self.cursor_pos = next_vl.start_byte + col_byte_offset;
            self.ensure_valid_cursor();
            self.adjust_input_scroll();
            self.check_and_update_tag_query();
        }
    }

    pub fn insert_newline(&mut self) {
        self.insert_char('\n');
        self.adjust_input_scroll();
    }

    pub fn insert_char(&mut self, c: char) {
        self.ensure_valid_cursor();
        let prefix = &self.input_buffer[..self.cursor_pos];
        let suffix = &self.input_buffer[self.cursor_pos..];
        self.input_buffer = format!("{}{}{}", prefix, c, suffix);
        self.cursor_pos += c.len_utf8();
        self.adjust_input_scroll();
        self.check_and_update_tag_query();
    }

    pub fn sanitize_pasted_text(text: &str) -> String {
        let text = text.replace('\r', "");
        let raw_lines: Vec<&str> = text.split('\n').collect();
        if raw_lines.len() <= 1 {
            return text.replace('\t', "    ");
        }

        let mut lines = Vec::new();
        let mut i = 0;
        while i < raw_lines.len() {
            let line = raw_lines[i];
            let trimmed_end = line.trim_end();
            if let Some(stripped) = trimmed_end.strip_suffix('\\') {
                // Shell line continuation: merge with next line
                let mut merged = stripped.trim_end().to_string();
                i += 1;
                while i < raw_lines.len() {
                    let next_line = raw_lines[i].trim();
                    if let Some(next_stripped) = next_line.strip_suffix('\\') {
                        merged.push(' ');
                        merged.push_str(next_stripped.trim_end());
                        i += 1;
                    } else {
                        if !next_line.is_empty() {
                            merged.push(' ');
                            merged.push_str(next_line);
                        }
                        break;
                    }
                }
                lines.push(merged);
            } else {
                lines.push(line.replace('\t', "    "));
            }
            i += 1;
        }

        lines.join("\n")
    }

    pub fn paste(&mut self, text: &str) {
        self.ensure_valid_cursor();
        let sanitized = Self::sanitize_pasted_text(text);
        let prefix = &self.input_buffer[..self.cursor_pos];
        let suffix = &self.input_buffer[self.cursor_pos..];
        self.input_buffer = format!("{}{}{}", prefix, sanitized, suffix);
        self.cursor_pos += sanitized.len();
        self.adjust_input_scroll();
        self.check_and_update_tag_query();
    }

    pub fn backspace(&mut self) {
        self.ensure_valid_cursor();
        if self.cursor_pos > 0 {
            let prev_len = self.input_buffer[..self.cursor_pos]
                .chars()
                .last()
                .map(|c| c.len_utf8())
                .unwrap_or(0);
            if prev_len > 0 && prev_len <= self.cursor_pos {
                let start = self.cursor_pos - prev_len;
                let end = self.cursor_pos;
                if self.input_buffer.is_char_boundary(start) && self.input_buffer.is_char_boundary(end) {
                    self.input_buffer.replace_range(start..end, "");
                    self.cursor_pos = start;
                }
            }
            self.adjust_input_scroll();
            self.check_and_update_tag_query();
        }
    }

    pub fn delete_forward(&mut self) {
        self.ensure_valid_cursor();
        if self.cursor_pos < self.input_buffer.len() {
            let next_len = self.input_buffer[self.cursor_pos..]
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(0);
            if next_len > 0 {
                let start = self.cursor_pos;
                let end = self.cursor_pos + next_len;
                if self.input_buffer.is_char_boundary(start) && self.input_buffer.is_char_boundary(end) {
                    self.input_buffer.replace_range(start..end, "");
                }
            }
            self.adjust_input_scroll();
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
        self.enforce_message_limit();
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
        self.ensure_valid_cursor();
        if let Some(at_pos) = self.input_buffer[..self.cursor_pos].rfind('@') {
            let slice_between = &self.input_buffer[at_pos + 1..self.cursor_pos];
            // If user typed space, tab, newline after '@', or query is unreasonably long (e.g. pasted code/email)
            if slice_between.contains(' ')
                || slice_between.contains('\t')
                || slice_between.contains('\n')
                || slice_between.len() > 64
            {
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
            self.session_start_instant = std::time::Instant::now();
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
            self.session_start_instant = std::time::Instant::now();
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
            total_requests: 0,
        };
        self.sessions.push(new_task);
        self.selected_session_idx = self.sessions.len() - 1;
        self.active_session_id = Some(id);
        self.session_start_instant = std::time::Instant::now();
    }

    pub fn delete_selected_task(&mut self) {
        if !self.sessions.is_empty() && self.selected_session_idx < self.sessions.len() {
            self.sessions.remove(self.selected_session_idx);
            if self.selected_session_idx >= self.sessions.len() && !self.sessions.is_empty() {
                self.selected_session_idx = self.sessions.len() - 1;
            }
            self.active_session_id = self.sessions.get(self.selected_session_idx).map(|s| s.id.clone());
            self.session_start_instant = std::time::Instant::now();
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
                    // Tool output is fed back as input context to the next model call
                    msg_in_tokens += tok;
                }
                _ => {}
            }
        }

        // Each tool call executed by the agent constitutes a full API round-trip cycle to the model.
        let total_tool_calls: usize = self.messages.iter().map(|m| m.tool_calls.len()).sum();

        // Calculate total LLM interaction turns (requests to model):
        // Each assistant response and tool execution represents a completed LLM API call cycle.
        // In addition, any ongoing turn (is_running) counts as an active live request (+1).
        // Determine current session either by active_session_id or selected_session_idx
        let cur_session = self
            .active_session_id
            .as_ref()
            .and_then(|aid| self.sessions.iter().find(|s| &s.id == aid))
            .or_else(|| self.sessions.get(self.selected_session_idx));

        let is_actively_running = self.is_running
            || cur_session.map(|s| s.is_running).unwrap_or(false);
        let active_request_increment = if is_actively_running { 1 } else { 0 };

        let completed_requests = assistant_turns + total_tool_calls;
        let (stored_in, stored_out, stored_requests) = if let Some(s) = cur_session {
            (s.total_tokens_input as usize, s.total_tokens_output as usize, s.total_requests)
        } else {
            (0, 0, 0)
        };

        let base_requests = completed_requests.max(stored_requests);
        let task_requests = if base_requests > 0 {
            base_requests + active_request_increment
        } else {
            user_prompts.max(active_request_increment)
        };

        // If actively running, ensure real-time stream token accumulation is included
        let live_stream_tokens = if is_actively_running {
            self.messages
                .last()
                .filter(|m| m.role == MessageRole::Assistant)
                .map(|m| {
                    let content_tok = m.content.len() / 4;
                    let thought_tok = m.reasoning.as_ref().map(|r| r.len() / 4).unwrap_or(0);
                    (content_tok + thought_tok).max(1)
                })
                .unwrap_or(1)
        } else {
            0
        };

        let task_input_tokens = stored_in.max(msg_in_tokens);
        let task_output_tokens = stored_out.max(msg_out_tokens) + live_stream_tokens;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_limit_eviction() {
        let mut state = AgentWorkspaceState::default();
        for i in 0..(MAX_CHAT_MESSAGES + 50) {
            state.push_message(ChatMessage {
                role: MessageRole::User,
                sender_name: "User".to_string(),
                content: format!("Message {}", i),
                timestamp: "now".to_string(),
                tool_calls: Vec::new(),
                reasoning: None,
            });
        }
        assert_eq!(state.messages.len(), MAX_CHAT_MESSAGES);
        assert_eq!(state.messages[0].content, "Message 50");
        assert_eq!(state.messages[MAX_CHAT_MESSAGES - 1].content, format!("Message {}", MAX_CHAT_MESSAGES + 49));
    }

    #[test]
    fn test_bracketed_paste_multiline() {
        let mut state = AgentWorkspaceState::default();
        state.insert_char('a');
        let pasted_text = "line 1\r\nline 2\nline 3";
        state.paste(pasted_text);
        assert_eq!(state.input_buffer, "aline 1\nline 2\nline 3");
        assert_eq!(state.cursor_pos, "aline 1\nline 2\nline 3".len());

        // Backspace should delete the last character safely
        state.backspace();
        assert_eq!(state.input_buffer, "aline 1\nline 2\nline ");
        assert_eq!(state.cursor_pos, "aline 1\nline 2\nline ".len());
    }

    #[test]
    fn test_paste_multiline_bash_command_and_backspace() {
        let mut state = AgentWorkspaceState::default();
        let cmd = "cargo clippy --bin ragasukma -- \\\n  -W dead_code \\\n  -W unused \\\n  -W clippy::redundant_clone \\\n  -W clippy::await_holding_lock \\\n  -W clippy::mutex_atomic \\\n  -W clippy::rc_buffer \\\n  -W clippy::panic \\\n  -W clippy::unwrap_used";
        state.paste(cmd);
        let expected = "cargo clippy --bin ragasukma -- -W dead_code -W unused -W clippy::redundant_clone -W clippy::await_holding_lock -W clippy::mutex_atomic -W clippy::rc_buffer -W clippy::panic -W clippy::unwrap_used";
        assert_eq!(state.input_buffer, expected);
        assert_eq!(state.cursor_pos, expected.len());

        // Backspacing 'd' from unwrap_used
        state.backspace();
        assert_eq!(
            state.input_buffer,
            "cargo clippy --bin ragasukma -- -W dead_code -W unused -W clippy::redundant_clone -W clippy::await_holding_lock -W clippy::mutex_atomic -W clippy::rc_buffer -W clippy::panic -W clippy::unwrap_use"
        );
        assert_eq!(state.cursor_pos, expected.len() - 1);
    }

    #[test]
    fn test_paste_with_at_symbol_no_freeze() {
        let mut state = AgentWorkspaceState::default();
        state.paste("const fn = @decorator\nfunction test() {}");
        assert!(state.tag_query.is_none());
        state.backspace();
        assert_eq!(state.input_buffer, "const fn = @decorator\nfunction test() {");
    }

    #[test]
    fn test_2d_cursor_navigation_and_scroll() {
        let mut state = AgentWorkspaceState::default();
        // Insert 5 lines
        state.paste("line 1\nline 2\nline 3\nline 4\nline 5");
        assert_eq!(state.get_cursor_row_col(), (4, 6)); // Line 4, col 6 ("line 5")
        // Scroll line must be 2 (showing lines 2, 3, 4 in 3-line window)
        assert_eq!(state.input_scroll_line, 2);

        // Move cursor UP
        state.cursor_up();
        assert_eq!(state.get_cursor_row_col(), (3, 6));

        state.cursor_up();
        assert_eq!(state.get_cursor_row_col(), (2, 6));

        state.cursor_up();
        assert_eq!(state.get_cursor_row_col(), (1, 6));
        // Viewport should scroll up to show line 1
        assert_eq!(state.input_scroll_line, 1);

        state.cursor_up();
        assert_eq!(state.get_cursor_row_col(), (0, 6));
        assert_eq!(state.input_scroll_line, 0);

        // Home and End
        state.cursor_home();
        assert_eq!(state.get_cursor_row_col(), (0, 0));

        state.cursor_end();
        assert_eq!(state.get_cursor_row_col(), (0, 6));

        // Move cursor DOWN
        state.cursor_down();
        assert_eq!(state.get_cursor_row_col(), (1, 6));

        // Insert newline
        state.insert_newline();
        assert_eq!(state.get_cursor_row_col(), (2, 0));
    }

    #[test]
    fn test_auto_wrap_to_newline_on_right_edge() {
        let mut state = AgentWorkspaceState::default();
        // Set wrap width to 10
        state.input_wrap_width.set(10);

        // Type 10 characters "0123456789"
        for c in "0123456789".chars() {
            state.insert_char(c);
        }
        // At 10 chars, it hits the right edge boundary and automatically wraps downwards to next line (line 1, col 0)
        assert_eq!(state.get_cursor_row_col(), (1, 0));

        // Type the 11th character
        state.insert_char('A');
        assert_eq!(state.get_cursor_row_col(), (1, 1));
        assert_eq!(state.input_buffer, "0123456789A");

        // Cursor UP moves back to first line
        state.cursor_up();
        assert_eq!(state.get_cursor_row_col(), (0, 1));

        // Cursor DOWN moves back to second line
        state.cursor_down();
        assert_eq!(state.get_cursor_row_col(), (1, 1));
    }

    #[test]
    fn test_tag_query_navigation_and_selection() {
        let mut state = AgentWorkspaceState::default();
        state.is_input_focused = true;
        state.all_workspace_tag_items = vec![
            TagItem {
                name: "file1.rs".into(),
                path: "src/file1.rs".into(),
                is_dir: false,
            },
            TagItem {
                name: "file2.rs".into(),
                path: "src/file2.rs".into(),
                is_dir: false,
            },
        ];

        // Trigger tag query
        state.input_buffer = "check @file".to_string();
        state.cursor_pos = state.input_buffer.len();
        state.check_and_update_tag_query();

        assert!(state.tag_query.is_some());
        let tq = state.tag_query.as_ref().unwrap();
        assert_eq!(tq.query, "file");
        assert_eq!(tq.matched_items.len(), 2);
        assert_eq!(tq.selected_idx, 0);

        // Next selection
        state.select_tag_query_next();
        assert_eq!(state.tag_query.as_ref().unwrap().selected_idx, 1);

        // Prev selection
        state.select_tag_query_prev();
        assert_eq!(state.tag_query.as_ref().unwrap().selected_idx, 0);

        // Apply selected tag
        state.apply_selected_tag();
        assert!(state.tag_query.is_none());
        assert_eq!(state.input_buffer, "check @src/file1.rs ");
    }
}