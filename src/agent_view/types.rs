#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolExecutionStatus {
    PendingApproval,
    Running,
    Success,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolFilter {
    All,
    RunningOnly,
    FailedOnly,
}

#[derive(Debug, Clone)]
pub struct ApprovalRequest {
    pub tool_id: String,
    pub tool_name: String,
    pub details: String,
    pub target_path: Option<String>,
    pub command: Option<String>,
    pub diff_content: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ToolCallInfo {
    pub id: String,
    pub name: String,
    pub args_summary: String,
    pub output_summary: Option<String>,
    pub status: ToolExecutionStatus,
    pub duration_ms: Option<u64>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub exit_code: Option<i32>,
    pub diff_preview: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub sender_name: String,
    pub content: String,
    pub timestamp: String,
    pub tool_calls: Vec<ToolCallInfo>,
    pub reasoning: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentFocusArea {
    Tasks,
    Files,
    Folders,
    Chat,
    Input,
    ModelTelemetry,
    McpTools,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentSidebarTab {
    Tasks,
    Files,
}

#[derive(Debug, Clone)]
pub struct TaskSessionItem {
    pub id: String,
    pub title: String,
    pub status: String,
    pub model: Option<String>,
    pub total_tokens_input: i64,
    pub total_tokens_output: i64,
    pub updated_at: i64,
    pub is_running: bool,
}

#[derive(Debug, Clone)]
pub struct WorkspaceFileNode {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
    pub depth: usize,
    pub is_expanded: bool,
    pub parent_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagItem {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
}

#[derive(Debug, Clone)]
pub struct FileTagQuery {
    pub query: String,
    pub matched_items: Vec<TagItem>,
    pub all_matches: Vec<TagItem>,
    pub selected_idx: usize,
    pub tag_start_idx: usize,
    pub batch_size: usize,
}

impl FileTagQuery {
    pub fn new(query: String, all_matches: Vec<TagItem>, tag_start_idx: usize) -> Self {
        let batch_size = 20;
        let initial_count = all_matches.len().min(batch_size);
        let matched_items = all_matches[..initial_count].to_vec();
        Self {
            query,
            matched_items,
            all_matches,
            selected_idx: 0,
            tag_start_idx,
            batch_size,
        }
    }

    pub fn has_more(&self) -> bool {
        self.matched_items.len() < self.all_matches.len()
    }

    pub fn load_next_batch(&mut self) -> bool {
        if !self.has_more() {
            return false;
        }
        let current_len = self.matched_items.len();
        let next_len = (current_len + self.batch_size).min(self.all_matches.len());
        self.matched_items.extend_from_slice(&self.all_matches[current_len..next_len]);
        true
    }
}

#[derive(Debug, Clone)]
pub struct TokenBreakdown {
    pub system_tokens: usize,
    pub chat_history_tokens: usize,
    pub session_tokens: usize,
    pub completion_tokens: usize,
    pub max_context_limit: usize,
    pub task_input_tokens: usize,
    pub task_output_tokens: usize,
    pub task_requests: usize,
}

impl TokenBreakdown {
    pub fn total_task_tokens(&self) -> usize {
        self.task_input_tokens + self.task_output_tokens
    }

    pub fn total_used(&self) -> usize {
        self.system_tokens + self.chat_history_tokens + self.session_tokens + self.completion_tokens
    }

    #[must_use]
    pub fn usage_ratio(&self) -> f64 {
        if self.max_context_limit == 0 {
            0.0
        } else {
            (self.total_used() as f64 / self.max_context_limit as f64).min(1.0)
        }
    }

    #[must_use]
    pub fn remaining_tokens(&self) -> usize {
        self.max_context_limit.saturating_sub(self.total_used())
    }

    #[must_use]
    pub fn is_danger(&self) -> bool {
        self.usage_ratio() >= 0.85
    }

    #[must_use]
    pub fn is_warning(&self) -> bool {
        self.usage_ratio() >= 0.60 && !self.is_danger()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceFolderItem {
    pub path: std::path::PathBuf,
    pub name: String,
    pub is_valid: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AddFolderModalState {
    pub is_open: bool,
    pub input_path: String,
    pub validation_message: Option<(bool, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectItem {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub root_path: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModelSelectorModalState {
    pub is_open: bool,
    pub selected_idx: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectSwitcherModalState {
    pub is_open: bool,
    pub selected_idx: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CreateProjectModalState {
    pub is_open: bool,
    pub name_input: String,
    pub description_input: String,
    pub cursor_pos: usize,
    pub active_field: usize, // 0: Name, 1: Description
    pub error_message: Option<String>,
}