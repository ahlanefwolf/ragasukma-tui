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
    Files,
    Chat,
    Inspector,
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

#[derive(Debug, Clone)]
pub struct FileTagQuery {
    pub query: String,
    pub matched_files: Vec<String>,
    pub selected_idx: usize,
    pub tag_start_idx: usize,
}

#[derive(Debug, Clone)]
pub struct ContextFileItem {
    pub path: String,
    pub name: String,
    pub token_count: usize,
    pub is_pinned: bool,
    pub is_readonly: bool,
    pub is_modified: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct TokenBreakdown {
    pub system_tokens: usize,
    pub chat_history_tokens: usize,
    pub context_files_tokens: usize,
    pub completion_tokens: usize,
    pub max_context_limit: usize,
}

impl TokenBreakdown {
    #[must_use]
    pub fn total_used(&self) -> usize {
        self.system_tokens + self.chat_history_tokens + self.context_files_tokens + self.completion_tokens
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
