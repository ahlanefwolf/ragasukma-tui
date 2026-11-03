use ragasukma_core::llm::providers::custom::CustomLlmAccount;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormField {
    Prefix,
    Name,
    BaseUrl,
    ApiKey,
    Enabled,
    SaveBtn,
    CancelBtn,
}

impl FormField {
    pub fn next(self) -> Self {
        match self {
            Self::Prefix => Self::Name,
            Self::Name => Self::BaseUrl,
            Self::BaseUrl => Self::ApiKey,
            Self::ApiKey => Self::Enabled,
            Self::Enabled => Self::SaveBtn,
            Self::SaveBtn => Self::CancelBtn,
            Self::CancelBtn => Self::Prefix,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Prefix => Self::CancelBtn,
            Self::Name => Self::Prefix,
            Self::BaseUrl => Self::Name,
            Self::ApiKey => Self::BaseUrl,
            Self::Enabled => Self::ApiKey,
            Self::SaveBtn => Self::Enabled,
            Self::CancelBtn => Self::SaveBtn,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CustomLlmTabState {
    pub accounts: Vec<CustomLlmAccount>,
    pub selected_idx: usize,
    pub models: Vec<(String, String)>,
    pub is_fetching_models: bool,

    // Modal Form State (Add / Edit)
    pub is_form_open: bool,
    pub editing_prefix: Option<String>,
    pub form_prefix: String,
    pub form_name: String,
    pub form_base_url: String,
    pub form_api_key: String,
    pub form_enabled: bool,
    pub focused_field: FormField,
    pub form_error: Option<String>,

    // Delete Confirmation Modal State
    pub delete_confirm_prefix: Option<String>,
}

impl Default for CustomLlmTabState {
    fn default() -> Self {
        Self::new()
    }
}
