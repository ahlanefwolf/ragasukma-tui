use ragasukma_core::llm::providers::antigravity::types::AntigravityQuotaResponse;
use ragasukma_core::llm::providers::AntigravityProvider;
use super::types::{AccountInfo, ModelQuotaItem, ProviderTabState};

pub fn load_antigravity_tab() -> ProviderTabState {
    std::thread::spawn(|| {
        let mut state = ProviderTabState::init_antigravity();
        let provider = AntigravityProvider::new();
        let raw_accounts = provider.list_accounts();

        state.accounts = raw_accounts
            .into_iter()
            .map(|a| {
                let plan = a
                    .tier_id
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or_else(|| "Standard".to_string());

                AccountInfo {
                    id: a.id.clone(),
                    email: a.email.unwrap_or(a.id),
                    plan,
                    is_active: a.is_active,
                    status: if a.is_active { "Active".to_string() } else { "Inactive".to_string() },
                    quotas: Vec::new(),
                }
            })
            .collect();

        state
    })
    .join()
    .unwrap_or_else(|_| ProviderTabState::init_antigravity())
}

pub fn map_antigravity_quotas(resp: AntigravityQuotaResponse) -> Vec<ModelQuotaItem> {
    let mut items = Vec::new();
    let mut all_models = resp.models;
    all_models.extend(resp.weekly);

    for m in all_models {
        let percentage = (m.remaining_percentage.clamp(0.0, 100.0).round()) as u16;
        let detail = format!("{:.1}% remaining", m.remaining_percentage);
        let reset_in = m.reset_time.unwrap_or_else(|| "Active Window".to_string());

        items.push(ModelQuotaItem {
            name: m.display_name,
            percentage,
            detail,
            reset_in,
        });
    }

    items
}
