use ragasukma_core::agent::models::{McpServer, McpServerTool};
use crate::ui::TextInput;

pub trait McpServerExt {
    fn config_value(&self) -> serde_json::Value;
}

impl McpServerExt for McpServer {
    fn config_value(&self) -> serde_json::Value {
        self.config_value()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpFormField {
    Name,
    Config,
    Enabled,
    SaveBtn,
    CancelBtn,
}

impl McpFormField {
    #[must_use]
    pub fn next(self) -> Self {
        match self {
            Self::Name => Self::Config,
            Self::Config => Self::Enabled,
            Self::Enabled => Self::SaveBtn,
            Self::SaveBtn => Self::CancelBtn,
            Self::CancelBtn => Self::Name,
        }
    }

    #[must_use]
    pub fn prev(self) -> Self {
        match self {
            Self::Name => Self::CancelBtn,
            Self::Config => Self::Name,
            Self::Enabled => Self::Config,
            Self::SaveBtn => Self::Enabled,
            Self::CancelBtn => Self::SaveBtn,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpPane {
    Servers,
    Tools,
}

#[derive(Debug, Clone)]
pub struct McpServerTabState {
    pub servers: Vec<McpServer>,
    pub selected_idx: usize,
    pub active_pane: McpPane,

    // Form modal state
    pub is_form_open: bool,
    pub editing_id: Option<String>,
    pub form_name: TextInput,
    pub form_json: TextInput,
    pub form_enabled: bool,
    pub focused_field: McpFormField,
    pub form_error: Option<String>,

    // Tools list & detail state
    pub is_tools_modal_open: bool,
    pub server_tools: Vec<McpServerTool>,
    pub tools_selected_idx: usize,
    pub is_tool_detail_open: bool,
    pub tool_detail_scroll: usize,

    // Sync modal state & model runner
    pub is_sync_modal_open: bool,
    pub available_models: Vec<String>,
    pub selected_model_idx: usize,
    pub sync_search_input: TextInput,
    pub is_syncing: bool,
    pub sync_status_message: Option<String>,

    // Delete confirm modal
    pub delete_confirm_id: Option<String>,
}
