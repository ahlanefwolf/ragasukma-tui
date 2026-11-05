use ragasukma_core::llm::providers::codex::types::{CodexModelInfo, CodexQuotaResponse};
use ragasukma_core::llm::providers::CodexProvider;
use super::types::{AccountInfo, ModelQuotaItem, ProviderTabState};

pub fn load_codex_tab() -> ProviderTabState {
    std::thread::spawn(|| {
        let mut state = ProviderTabState::init_codex();
        let provider = CodexProvider::new();
        let raw_accounts = provider.list_account_infos();

        state.accounts = raw_accounts
            .into_iter()
            .map(|a| {
                let plan = a
                    .plan_type
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or_else(|| "Standard".to_string());

                AccountInfo {
                    id: a.id.clone(),
                    email: a.email.unwrap_or(a.id),
                    plan,
                    is_active: a.is_active,
                    status: if a.is_active {
                        "Active".to_string()
                    } else {
                        "Inactive".to_string()
                    },
                    quotas: Vec::new(),
                }
            })
            .collect();

        state
    })
    .join()
    .unwrap_or_else(|_| ProviderTabState::init_codex())
}

pub fn map_codex_quotas(
    resp: CodexQuotaResponse,
    models: Vec<CodexModelInfo>,
) -> Vec<ModelQuotaItem> {
    let mut items = Vec::new();

    let mut session_pct = 100u16;
    let mut session_detail = "Active".to_string();
    let mut session_reset = "Shared Pool".to_string();

    if let Some(session) = resp.session_quota {
        session_pct = if session.total > 0.0 {
            ((session.remaining / session.total) * 100.0).clamp(0.0, 100.0) as u16
        } else {
            100
        };
        session_detail = format!("{:.0} / {:.0} remaining", session.remaining, session.total);
        if let Some(ref r) = session.reset_at {
            session_reset = r.clone();
        }
        items.push(ModelQuotaItem {
            name: "Session Quota Window".to_string(),
            percentage: session_pct,
            detail: session_detail.clone(),
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

    // Display models retrieved directly from account
    for model in models {
        let label = if !model.display_name.is_empty() {
            format!("{} ({})", model.display_name, model.model_id)
        } else {
            model.model_id
        };

        items.push(ModelQuotaItem {
            name: label,
            percentage: session_pct,
            detail: session_detail.clone(),
            reset_in: session_reset.clone(),
        });
    }

    items
}