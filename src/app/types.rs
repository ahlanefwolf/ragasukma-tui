use crate::core_bridge::{CoreBridgeCommand, CoreBridgeEvent};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffLineType {
    Context,
    Added,
    Removed,
    Header,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    pub line_type: DiffLineType,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffApprovalState {
    pub tool_call_id: String,
    pub tool_name: String,
    pub file_path: String,
    pub diff_lines: Vec<DiffLine>,
    pub scroll_offset: usize,
}

impl DiffApprovalState {
    pub fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset = self.scroll_offset.saturating_sub(1);
        }
    }

    pub fn scroll_down(&mut self) {
        if self.scroll_offset + 1 < self.diff_lines.len() {
            self.scroll_offset = self.scroll_offset.saturating_add(1);
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConfirmDialogState {
    pub title: String,
    pub message: String,
    pub prompt: String,
    pub on_confirm_action: Box<Action>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Quit,
    NextTab,
    PrevTab,
    SwitchTab(usize),
    ToggleHelp,
    CloseModal,
    SetStatus(String),

    FocusNext,
    FocusPrev,
    Select,

    ConfirmOk,
    ConfirmCancel,
    ShowError(String),
    ConfirmDelete,
    CancelDelete,

    StartAntigravityOAuth,
    StartCodexOAuth,
    StartClaudeOAuth,

    AgentInsertChar(char),
    AgentBackspace,
    AgentClearInput,
    AgentClearChat,
    AgentSubmitPrompt,
    AgentFocusInput,
    AgentUnfocusInput,
    AgentFocusFiles,
    AgentFocusChat,
    AgentFocusNextPanel,
    AgentFocusPrevPanel,
    AgentSelectFileNext,
    AgentSelectFilePrev,
    AgentInsertSelectedFile,
    AgentToggleFileTreeExpand,
    AgentScanFiles,
    AgentScrollUp,
    AgentScrollDown,
    AgentToggleToolExpand,

    // Phase 1 Foundations
    AgentTagQueryNext,
    AgentTagQueryPrev,
    AgentApplyTag,
    AgentCloseTagQuery,

    // Phase 2 Chat Streaming & Reasoning
    AgentToggleReasoning,
    AgentScrollToTop,
    AgentScrollToBottom,

    // Phase 3 Tool Execution & Approval
    AgentApprovePending,
    AgentRejectPending,
    AgentCycleToolFilter,
    AgentToggleToolCard(usize),

    // Phase 4 Tasks & Sessions (1:1 with ai-voice AgentSidebar)
    AgentSwitchSidebarTab,
    AgentNewTask,
    AgentDeleteSelectedTask,
    AgentSwitchTask,
    AgentNextTask,
    AgentPrevTask,

    // Phase 5 Async Engine Integration & Turn Cancellation
    AgentCancelTurn,
    AgentRetryTurn,

    // Phase 6 Ergonomics & Responsive Adaptations
    AgentToggleSidebar,
    AgentFocusPrompt,
    AgentCycleModel,
    AgentOpenModelModal,
    AgentCloseModelModal,
    AgentSelectNextModalModel,
    AgentSelectPrevModalModel,
    AgentApplyModalModel,
    AgentModelModalChar(char),
    AgentModelModalBackspace,
    AgentRefreshModels,

    // Multi-Folder Workspace Support (1:1 with web agent)
    AgentOpenAddFolderModal,
    AgentCloseAddFolderModal,
    AgentAddFolderChar(char),
    AgentAddFolderBackspace,
    AgentSubmitAddFolder,
    AgentRemoveActiveFolder,
    AgentNextFolder,
    AgentPrevFolder,
    AgentBrowseFolderDialog,
    AgentOpenFolderInFileManager,

    // Phase 7 Projects (Database SQLite migrations/0004)
    AgentOpenProjectModal,
    AgentCloseProjectModal,
    AgentSelectNextProject,
    AgentSelectPrevProject,
    AgentSwitchProject,
    AgentOpenCreateProjectModal,
    AgentCloseCreateProjectModal,
    AgentCreateProjectChar(char),
    AgentCreateProjectBackspace,
    AgentSubmitCreateProject,

    AgentPasteClipboard,
    AgentCopyActive,
    AgentSelectAllInput,

    FormChar(char),
    FormBackspace,
    FormNextField,
    FormPrevField,
    FormSubmit,
    CancelForm,
    CustomLlmAdd,
    CustomLlmEdit,
    CustomLlmToggle,
    CustomLlmDelete,
    CustomLlmConfirmDelete,
    CustomLlmCancelDelete,
    CustomLlmRefresh,
    CustomLlmToggleFocus,

    ShowDiffPreview {
        file_path: String,
        tool_name: String,
        diff_lines: Vec<DiffLine>,
    },
    DiffScrollUp,
    DiffScrollDown,
    DiffApprove,
    DiffReject,

    BridgeEvent(CoreBridgeEvent),
    BridgeCommand(CoreBridgeCommand),

    ToggleAccountActive,
    SetAccountActive,
    RefreshQuota,
    QuotaScrollUp,
    QuotaScrollDown,
    ProviderToggleFocus,
}
