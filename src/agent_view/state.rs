use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::types::{
    AddFolderModalState, AgentFocusArea, ApprovalRequest, ChatMessage, ContextFileItem,
    FileTagQuery, MessageRole, TokenBreakdown, ToolFilter, WorkspaceFileNode,
    WorkspaceFolderItem,
};
use crate::core_bridge::PersistedMessageDto;

#[derive(Debug, Clone)]
pub struct AgentWorkspaceState {
    pub messages: Vec<ChatMessage>,
    pub input_buffer: String,
    pub cursor_pos: usize,
    pub scroll_offset: usize,
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

    // Phase 3: Tool Execution & Filter
    pub tool_filter: ToolFilter,
    pub pending_approval: Option<ApprovalRequest>,

    // Phase 4: Context Management & Tokens
    pub context_files: Vec<ContextFileItem>,
    pub selected_context_file_idx: usize,
    pub total_prompt_tokens: usize,
    pub total_completion_tokens: usize,
    pub max_context_limit: usize,

    // Phase 6: Responsive Layout & Inspection
    pub is_sidebar_visible: bool,
    pub is_inspector_visible: bool,
}

impl Default for AgentWorkspaceState {
    fn default() -> Self {
        let initial_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self {
            messages: Vec::new(),
            input_buffer: String::new(),
            cursor_pos: 0,
            scroll_offset: 0,
            is_input_focused: false,
            focus_area: AgentFocusArea::Chat,
            is_running: false,
            active_turn_id: None,
            active_model: "claude-3-5-sonnet-20241022".to_string(),
            available_models: vec![
                "claude-3-5-sonnet-20241022".to_string(),
                "gpt-4o".to_string(),
                "gemini-1.5-pro".to_string(),
                "deepseek-chat".to_string(),
            ],
            expanded_tools: HashSet::new(),
            expanded_tool_idx: None,
            reasoning_expanded: true,
            auto_scroll: true,
            spinner_tick: 0,
            workspace_folders: vec![initial_root],
            active_folder_idx: 0,
            add_folder_modal: AddFolderModalState::default(),
            workspace_files: Vec::new(),
            selected_file_idx: 0,
            expanded_dirs: HashSet::new(),
            tag_query: None,
            tool_filter: ToolFilter::All,
            pending_approval: None,
            context_files: Vec::new(),
            selected_context_file_idx: 0,
            total_prompt_tokens: 0,
            total_completion_tokens: 0,
            max_context_limit: 128_000,
            is_sidebar_visible: true,
            is_inspector_visible: true,
        }
    }
}

impl AgentWorkspaceState {
    pub fn sample() -> Self {
        Self {
            messages: vec![ChatMessage {
                role: MessageRole::System,
                sender_name: "System".to_string(),
                content: "Ragasukma Agent ready. Tech Gold & Dark Slate theme active.".to_string(),
                timestamp: "09:00".to_string(),
                tool_calls: Vec::new(),
                reasoning: None,
            }],
            ..Default::default()
        }
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
    }

    pub fn fail_turn(&mut self, _err: String) {
        self.is_running = false;
        self.active_turn_id = None;
    }

    pub fn submit_prompt(&mut self) -> Option<String> {
        let trimmed = self.input_buffer.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            self.input_buffer.clear();
            self.cursor_pos = 0;
            self.tag_query = None;
            Some(trimmed)
        }
    }

    pub fn focus_input(&mut self) {
        self.is_input_focused = true;
        self.focus_area = AgentFocusArea::Chat;
    }

    pub fn unfocus_input(&mut self) {
        self.is_input_focused = false;
    }

    pub fn cycle_focus_next(&mut self) {
        self.focus_area = match self.focus_area {
            AgentFocusArea::Files => AgentFocusArea::Chat,
            AgentFocusArea::Chat => {
                if self.is_inspector_visible {
                    AgentFocusArea::Inspector
                } else if self.is_sidebar_visible {
                    AgentFocusArea::Files
                } else {
                    AgentFocusArea::Chat
                }
            }
            AgentFocusArea::Inspector => {
                if self.is_sidebar_visible {
                    AgentFocusArea::Files
                } else {
                    AgentFocusArea::Chat
                }
            }
        };
    }

    pub fn cycle_focus_prev(&mut self) {
        self.focus_area = match self.focus_area {
            AgentFocusArea::Files => {
                if self.is_inspector_visible {
                    AgentFocusArea::Inspector
                } else {
                    AgentFocusArea::Chat
                }
            }
            AgentFocusArea::Chat => {
                if self.is_sidebar_visible {
                    AgentFocusArea::Files
                } else if self.is_inspector_visible {
                    AgentFocusArea::Inspector
                } else {
                    AgentFocusArea::Chat
                }
            }
            AgentFocusArea::Inspector => AgentFocusArea::Chat,
        };
    }

    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(3);
        self.auto_scroll = false;
    }

    pub fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(3);
        if self.scroll_offset == 0 {
            self.auto_scroll = true;
        }
    }

    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = 9999;
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

    pub fn start_turn(&mut self) {
        self.is_running = true;
    }

    pub fn cancel_turn(&mut self) {
        self.is_running = false;
        self.active_turn_id = None;
    }

    pub fn tick_spinner(&mut self) {
        self.spinner_tick = (self.spinner_tick + 1) % 10;
    }

    pub fn check_and_update_tag_query(&mut self) {
        if let Some(at_pos) = self.input_buffer[..self.cursor_pos].rfind('@') {
            let query = self.input_buffer[at_pos + 1..self.cursor_pos].to_string();
            let matches: Vec<String> = self
                .workspace_files
                .iter()
                .filter(|f| !f.is_dir && f.name.to_lowercase().contains(&query.to_lowercase()))
                .map(|f| f.path.clone())
                .take(8)
                .collect();

            self.tag_query = Some(FileTagQuery {
                query,
                matched_files: matches,
                selected_idx: 0,
                tag_start_idx: at_pos,
            });
        } else {
            self.tag_query = None;
        }
    }

    pub fn close_tag_query(&mut self) {
        self.tag_query = None;
    }

    pub fn select_tag_query_next(&mut self) {
        if let Some(tq) = &mut self.tag_query {
            let len = tq.matched_files.len();
            if len > 0 {
                tq.selected_idx = (tq.selected_idx + 1) % len;
            }
        }
    }

    pub fn select_tag_query_prev(&mut self) {
        if let Some(tq) = &mut self.tag_query {
            let len = tq.matched_files.len();
            if len > 0 {
                if tq.selected_idx == 0 {
                    tq.selected_idx = len - 1;
                } else {
                    tq.selected_idx -= 1;
                }
            }
        }
    }

    pub fn apply_selected_tag(&mut self) {
        let (prefix, insertion, suffix) = match self.tag_query.take() {
            Some(tq) => match tq.matched_files.get(tq.selected_idx) {
                Some(path) => {
                    let prefix = self.input_buffer[..tq.tag_start_idx].to_string();
                    let suffix = self.input_buffer[self.cursor_pos..].to_string();
                    let insertion = format!("@{} ", path);
                    (prefix, insertion, suffix)
                }
                None => return,
            },
            None => return,
        };
        let new_cursor = prefix.len() + insertion.len();
        self.input_buffer = format!("{}{}{}", prefix, insertion, suffix);
        self.cursor_pos = new_cursor;
    }

    pub fn toggle_pin_file(&mut self, path: &str) {
        if let Some(cf) = self.context_files.iter_mut().find(|f| f.path == path) {
            cf.is_pinned = !cf.is_pinned;
        } else {
            self.context_files.push(ContextFileItem {
                path: path.to_string(),
                name: Path::new(path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.to_string()),
                token_count: 50,
                is_pinned: true,
                is_readonly: false,
                is_modified: false,
            });
        }
    }

    pub fn toggle_pin_selected_file(&mut self) {
        if let Some(cf) = self.context_files.get_mut(self.selected_context_file_idx) {
            cf.is_pinned = !cf.is_pinned;
        }
    }

    pub fn remove_context_file(&mut self, idx: usize) {
        if idx < self.context_files.len() {
            self.context_files.remove(idx);
            if self.selected_context_file_idx >= self.context_files.len() && !self.context_files.is_empty() {
                self.selected_context_file_idx = self.context_files.len() - 1;
            }
        }
    }

    pub fn next_context_file(&mut self) {
        if !self.context_files.is_empty() {
            self.selected_context_file_idx = (self.selected_context_file_idx + 1) % self.context_files.len();
        }
    }

    pub fn prev_context_file(&mut self) {
        if !self.context_files.is_empty() {
            if self.selected_context_file_idx == 0 {
                self.selected_context_file_idx = self.context_files.len() - 1;
            } else {
                self.selected_context_file_idx -= 1;
            }
        }
    }

    pub fn calculate_token_breakdown(&self) -> TokenBreakdown {
        let chat_tokens = self.messages.iter().map(|m| m.content.len() / 4).sum();
        let file_tokens = self.context_files.iter().map(|f| f.token_count).sum();
        TokenBreakdown {
            system_tokens: 1500,
            chat_history_tokens: chat_tokens,
            context_files_tokens: file_tokens,
            completion_tokens: self.total_completion_tokens,
            max_context_limit: self.max_context_limit,
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
                if let Some(home) = std::env::var_os("HOME") {
                    PathBuf::from(home).join(rest)
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
            self.workspace_folders.remove(idx);
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
            let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            self.workspace_folders.push(current_dir);
        }

        let mut nodes = Vec::new();
        if self.workspace_folders.len() == 1 {
            let single_dir = &self.workspace_folders[0];
            collect_visible_children(single_dir, 0, &self.expanded_dirs, &mut nodes, None);
        } else {
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
        }

        self.workspace_files = nodes;
        if self.selected_file_idx >= self.workspace_files.len() && !self.workspace_files.is_empty() {
            self.selected_file_idx = self.workspace_files.len() - 1;
        }
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
