#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolExecutionStatus {
    Running,
    Success,
    Failed,
}

#[derive(Debug, Clone)]
pub struct ToolCallInfo {
    pub id: String,
    pub name: String,
    pub args_summary: String,
    pub output_summary: Option<String>,
    pub status: ToolExecutionStatus,
    pub duration_ms: Option<u64>,
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
    Sessions,
    Files,
    Chat,
}

#[derive(Debug, Clone)]
pub struct ChatSession {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub messages: Vec<ChatMessage>,
}
