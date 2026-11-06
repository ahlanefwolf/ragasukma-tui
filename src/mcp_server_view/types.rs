use ragasukma_core::agent::models::McpServer;

pub trait McpServerExt {
    fn config_value(&self) -> serde_json::Value;
}

impl McpServerExt for McpServer {
    fn config_value(&self) -> serde_json::Value {
        serde_json::from_str(&self.config).unwrap_or(serde_json::Value::Null)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpFormField {
    Name,
    JsonConfig,
    Enabled,
    SaveBtn,
    CancelBtn,
}

impl McpFormField {
    #[must_use]
    pub fn next(self) -> Self {
        match self {
            Self::Name => Self::JsonConfig,
            Self::JsonConfig => Self::Enabled,
            Self::Enabled => Self::SaveBtn,
            Self::SaveBtn => Self::CancelBtn,
            Self::CancelBtn => Self::Name,
        }
    }

    #[must_use]
    pub fn prev(self) -> Self {
        match self {
            Self::Name => Self::CancelBtn,
            Self::JsonConfig => Self::Name,
            Self::Enabled => Self::JsonConfig,
            Self::SaveBtn => Self::Enabled,
            Self::CancelBtn => Self::SaveBtn,
        }
    }
}

#[derive(Debug, Clone)]
pub struct McpServerTabState {
    pub servers: Vec<McpServer>,
    pub selected_idx: usize,

    // Form modal state
    pub is_form_open: bool,
    pub editing_id: Option<String>,
    pub form_name: String,
    pub form_json: String,
    pub form_enabled: bool,
    pub focused_field: McpFormField,
    pub form_error: Option<String>,
    pub name_cursor: usize,
    pub json_cursor: usize,
    pub json_scroll: usize,

    // Delete confirm modal
    pub delete_confirm_id: Option<String>,
}
