#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelQuotaItem {
    pub name: String,
    pub percentage: u16,
    pub detail: String,
    pub reset_in: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountInfo {
    pub id: String,
    pub email: String,
    pub plan: String,
    pub is_active: bool,
    pub status: String,
    pub quotas: Vec<ModelQuotaItem>,
}

#[derive(Debug, Clone)]
pub struct ProviderTabState {
    pub provider_name: String,
    pub icon: String,
    pub accounts: Vec<AccountInfo>,
    pub selected_account_idx: usize,
    pub last_refreshed: Option<String>,
    pub is_refreshing: bool,
}
