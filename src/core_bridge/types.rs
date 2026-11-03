use ragasukma_core::agent::models::Message;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedMessageDto {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: Option<String>,
    pub thought: Option<String>,
    pub tool_calls: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectDto {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

impl From<Message> for PersistedMessageDto {
    fn from(m: Message) -> Self {
        Self {
            id: m.id,
            session_id: m.session_id,
            role: m.role,
            content: m.content,
            thought: m.thought,
            tool_calls: m.tool_calls,
            created_at: m.created_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreBridgeEvent {
    Connected {
        project_id: String,
        session_id: String,
    },
    ProjectsLoaded {
        projects: Vec<ProjectDto>,
        active_id: String,
        active_name: String,
    },
    ProjectCreated {
        project: ProjectDto,
    },
    ProjectSwitched {
        project_id: String,
        project_name: String,
    },
    ProjectDeleted {
        id: String,
    },
    SessionsLoaded {
        sessions: Vec<(String, String)>,
        active_id: String,
    },
    SessionCreated {
        id: String,
        title: String,
    },
    SessionDeleted {
        id: String,
    },
    HistoryLoaded {
        session_id: String,
        messages: Vec<PersistedMessageDto>,
    },
    StatusChange(String),
    Delta(String),
    ThoughtDelta(String),
    StepInfo {
        iteration: usize,
        action: String,
        details: Option<String>,
    },
    ToolCall {
        name: String,
        args: String,
    },
    TurnCompleted(String),
    TurnCancelled {
        session_id: String,
    },
    TurnFailed {
        session_id: String,
        error: String,
    },
    QuotaUpdated {
        provider: String,
        input_tokens: usize,
        output_tokens: usize,
        total_tokens: usize,
        requests: usize,
    },
    AccountQuotaUpdated {
        provider: String,
        account_id: String,
        quotas: Vec<crate::provider_view::ModelQuotaItem>,
    },
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreBridgeCommand {
    ExecuteTurn {
        prompt: String,
        model_override: Option<String>,
    },
    RefreshQuota {
        provider: String,
    },
    RefreshAccountQuota {
        provider: String,
        account_id: String,
    },
    CreateProject {
        name: String,
        description: Option<String>,
    },
    SwitchProject {
        project_id: String,
    },
    DeleteProject {
        project_id: String,
    },
    CreateSession {
        title: String,
    },
    SwitchSession {
        session_id: String,
    },
    DeleteSession {
        session_id: String,
    },
    LoadHistory {
        session_id: String,
    },
    CancelTurn,
    RetryTurn,
}
