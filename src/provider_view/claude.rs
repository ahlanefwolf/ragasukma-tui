use ragasukma_core::llm::providers::claude::types::ClaudeQuotaResponse;
use ragasukma_core::llm::providers::ClaudeProvider;
use super::types::{AccountInfo, ModelQuotaItem, ProviderTabState};

pub fn load_claude_tab() -> ProviderTabState {
    std::thread::spawn(|| {
        let mut state = ProviderTabState::init_claude();
        let provider = ClaudeProvider::new();
        let raw_accounts = provider.list_account_infos();

        state.accounts = raw_accounts
            .into_iter()
            .map(|a| AccountInfo {
                id: a.id.clone(),
                email: a.email.unwrap_or(a.id),
                plan: "Claude Pro/Team".to_string(),
                is_active: a.is_active,
                status: if a.is_active { "Active".to_string() } else { "Inactive".to_string() },
                quotas: Vec::new(),
            })
            .collect();

        state
    })
    .join()
    .unwrap_or_else(|_| ProviderTabState::init_claude())
}

pub fn map_claude_quotas(resp: ClaudeQuotaResponse) -> Vec<ModelQuotaItem> {
    let mut items = Vec::new();

    if let Some(session) = resp.session_quota {
        let percentage = if session.total > 0.0 {
            ((session.remaining / session.total) * 100.0).clamp(0.0, 100.0) as u16
        } else {
            100
        };
        items.push(ModelQuotaItem {
            name: "Session Quota Window".to_string(),
            percentage,
            detail: format!("{:.0} / {:.0} remaining", session.remaining, session.total),
            reset_in: session.reset_at.unwrap_or_else(|| "Active Window".to_string()),
        });
    }

    if let Some(weekly) = resp.weekly_quota {
        let percentage = if weekly.total > 0.0 {
            ((weekly.remaining / weekly.total) * 100.0).clamp(0.0, 100.0) as u16
        } else {
            100
        };
        items.push(ModelQuotaItem {
            name: "Weekly Quota Window".to_string(),
            percentage,
            detail: format!("{:.0} / {:.0} remaining", weekly.remaining, weekly.total),
            reset_in: weekly.reset_at.unwrap_or_else(|| "Weekly".to_string()),
        });
    }

    items
}