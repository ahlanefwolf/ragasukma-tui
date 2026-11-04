use ragasukma_core::agent::models::Message;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectDto {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub root_path: Option<String>,
}

impl From<ragasukma_core::agent::models::Project> for ProjectDto {
    fn from(p: ragasukma_core::agent::models::Project) -> Self {
        Self {
            id: p.id,
            name: p.name,
            description: Some(p.description),
            root_path: p.root_path,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionDto {
    pub id: String,
    pub title: String,
    pub status: String,
    pub model: Option<String>,
    pub total_tokens_input: i64,
    pub total_tokens_output: i64,
    pub updated_at: i64,
}

impl From<ragasukma_core::agent::models::Session> for SessionDto {
    fn from(s: ragasukma_core::agent::models::Session) -> Self {
        Self {
            id: s.id,
            title: s.title,
            status: s.status,
            model: s.model,
            total_tokens_input: s.total_tokens_input.unwrap_or(0),
            total_tokens_output: s.total_tokens_output.unwrap_or(0),
            updated_at: s.updated_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedMessageDto {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: Option<String>,
    pub thought: Option<String>,
    pub tool_calls: Option<String>,
    pub token_usage_input: Option<i64>,
    pub token_usage_output: Option<i64>,
    pub created_at: i64,
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
            token_usage_input: m.token_usage_input,
            token_usage_output: m.token_usage_output,
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
        root_path: Option<String>,
    },
    ProjectDeleted {
        id: String,
    },
    SessionsLoaded {
        sessions: Vec<SessionDto>,
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
        root_path: Option<String>,
    },
    UpdateProjectFolders {
        project_id: String,
        folders: Vec<String>,
    },
    SwitchProject {
        project_id: String,
    },
    DeleteProject {
        project_id: String,
    },
    CreateSession {
        title: String,
        model: Option<String>,
    },
    SwitchSession {
        session_id: String,
    },
    UpdateSessionModel {
        session_id: String,
        model: String,
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