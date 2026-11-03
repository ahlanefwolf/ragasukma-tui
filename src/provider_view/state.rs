use super::types::{AccountInfo, ModelQuotaItem, ProviderTabState};

impl ProviderTabState {
    pub fn load_antigravity() -> Self {
        let real_accounts = crate::assets::load_antigravity_accounts();
        if !real_accounts.is_empty() {
            let accounts = real_accounts
                .into_iter()
                .map(|acc| {
                    let status = if !acc.is_active {
                        "Disabled".to_string()
                    } else if acc.rate_limited_until > 0 {
                        "Rate Limited".to_string()
                    } else {
                        "Active".to_string()
                    };

                    let tier_name = acc.tier_id.clone().unwrap_or_else(|| "free-tier".to_string());
                    let proj_name = acc.project_id.clone().unwrap_or_else(|| "default".to_string());

                    let quotas = vec![
                        ModelQuotaItem {
                            name: "gemini-2.5-pro".to_string(),
                            percentage: 90,
                            detail: format!("Tier: {}", tier_name),
                            reset_in: "Active session".to_string(),
                        },
                        ModelQuotaItem {
                            name: "gemini-2.5-flash".to_string(),
                            percentage: 98,
                            detail: format!("Project: {}", proj_name),
                            reset_in: "Ready".to_string(),
                        },
                        ModelQuotaItem {
                            name: "claude-3-7-sonnet".to_string(),
                            percentage: 85,
                            detail: "OAuth Connected".to_string(),
                            reset_in: "Ready".to_string(),
                        },
                    ];

                    AccountInfo {
                        id: acc.id,
                        email: acc.email.unwrap_or_else(|| "active@google.com".to_string()),
                        plan: tier_name.to_uppercase(),
                        is_active: acc.is_active,
                        status,
                        quotas,
                    }
                })
                .collect();

            Self {
                provider_name: "Antigravity".to_string(),
                icon: "🪐".to_string(),
                accounts,
                selected_account_idx: 0,
                is_refreshing: false,
                last_refreshed: Some("Loaded from assets/config".to_string()),
            }
        } else {
            Self::sample_antigravity()
        }
    }

    pub fn load_codex() -> Self {
        let real_accounts = crate::assets::load_codex_accounts();
        if !real_accounts.is_empty() {
            let accounts = real_accounts
                .into_iter()
                .map(|acc| AccountInfo {
                    id: acc.id,
                    email: acc.email.unwrap_or_else(|| "codex@openai.com".to_string()),
                    plan: "PAY-AS-YOU-GO".to_string(),
                    is_active: acc.is_active,
                    status: if acc.is_active { "Active".to_string() } else { "Inactive".to_string() },
                    quotas: vec![
                        ModelQuotaItem {
                            name: "gpt-4o".to_string(),
                            percentage: 80,
                            detail: "Standard Quota".to_string(),
                            reset_in: "Rolling window".to_string(),
                        },
                    ],
                })
                .collect();

            Self {
                provider_name: "Codex".to_string(),
                icon: "🔮".to_string(),
                accounts,
                selected_account_idx: 0,
                is_refreshing: false,
                last_refreshed: Some("Loaded from assets/config".to_string()),
            }
        } else {
            Self::sample_codex()
        }
    }

    pub fn load_claude() -> Self {
        let real_accounts = crate::assets::load_claude_accounts();
        if !real_accounts.is_empty() {
            let accounts = real_accounts
                .into_iter()
                .map(|acc| AccountInfo {
                    id: acc.id,
                    email: acc.email.unwrap_or_else(|| "claude@anthropic.com".to_string()),
                    plan: "PRO".to_string(),
                    is_active: acc.is_active,
                    status: if acc.is_active { "Active".to_string() } else { "Inactive".to_string() },
                    quotas: vec![
                        ModelQuotaItem {
                            name: "claude-3-7-sonnet".to_string(),
                            percentage: 75,
                            detail: "Standard Quota".to_string(),
                            reset_in: "5h window".to_string(),
                        },
                    ],
                })
                .collect();

            Self {
                provider_name: "Claude".to_string(),
                icon: "🧠".to_string(),
                accounts,
                selected_account_idx: 0,
                is_refreshing: false,
                last_refreshed: Some("Loaded from assets/config".to_string()),
            }
        } else {
            Self::sample_claude()
        }
    }

    pub fn sample_antigravity() -> Self {
        Self {
            provider_name: "Antigravity".to_string(),
            icon: "🪐".to_string(),
            accounts: vec![
                AccountInfo {
                    id: "ag-1".to_string(),
                    email: "developer.primary@gmail.com".to_string(),
                    plan: "ULTIMATE".to_string(),
                    is_active: true,
                    status: "Healthy".to_string(),
                    quotas: vec![
                        ModelQuotaItem {
                            name: "gemini-2.5-pro".to_string(),
                            percentage: 82,
                            detail: "4,100 / 5,000 reqs".to_string(),
                            reset_in: "3h 42m".to_string(),
                        },
                        ModelQuotaItem {
                            name: "gemini-2.5-flash".to_string(),
                            percentage: 95,
                            detail: "19,000 / 20,000 reqs".to_string(),
                            reset_in: "1h 15m".to_string(),
                        },
                        ModelQuotaItem {
                            name: "claude-3-5-sonnet".to_string(),
                            percentage: 35,
                            detail: "350 / 1,000 reqs".to_string(),
                            reset_in: "6h 20m".to_string(),
                        },
                        ModelQuotaItem {
                            name: "gemini-2.0-flash-exp".to_string(),
                            percentage: 60,
                            detail: "600 / 1,000 reqs".to_string(),
                            reset_in: "45m".to_string(),
                        },
                    ],
                },
                AccountInfo {
                    id: "ag-2".to_string(),
                    email: "backup.work@company.io".to_string(),
                    plan: "PRO".to_string(),
                    is_active: false,
                    status: "Standby".to_string(),
                    quotas: vec![
                        ModelQuotaItem {
                            name: "gemini-2.5-pro".to_string(),
                            percentage: 100,
                            detail: "2,000 / 2,000 reqs".to_string(),
                            reset_in: "24h".to_string(),
                        },
                        ModelQuotaItem {
                            name: "gemini-2.5-flash".to_string(),
                            percentage: 88,
                            detail: "8,800 / 10,000 reqs".to_string(),
                            reset_in: "5h 10m".to_string(),
                        },
                    ],
                },
            ],
            selected_account_idx: 0,
            is_refreshing: false,
            last_refreshed: Some("Just now".to_string()),
        }
    }

    pub fn sample_codex() -> Self {
        Self {
            provider_name: "Codex".to_string(),
            icon: "🔮".to_string(),
            accounts: vec![
                AccountInfo {
                    id: "codex-1".to_string(),
                    email: "team@openai-enterprise.com".to_string(),
                    plan: "TEAM TIER 4".to_string(),
                    is_active: true,
                    status: "Healthy".to_string(),
                    quotas: vec![
                        ModelQuotaItem {
                            name: "gpt-4o".to_string(),
                            percentage: 76,
                            detail: "760 / 1,000 reqs (3h window)".to_string(),
                            reset_in: "1h 25m".to_string(),
                        },
                        ModelQuotaItem {
                            name: "o3-mini-high".to_string(),
                            percentage: 42,
                            detail: "420 / 1,000 reqs (Weekly)".to_string(),
                            reset_in: "2d 4h".to_string(),
                        },
                        ModelQuotaItem {
                            name: "o1-preview".to_string(),
                            percentage: 15,
                            detail: "15 / 100 reqs (Weekly)".to_string(),
                            reset_in: "4d 18h".to_string(),
                        },
                    ],
                },
            ],
            selected_account_idx: 0,
            is_refreshing: false,
            last_refreshed: Some("1m ago".to_string()),
        }
    }

    pub fn sample_claude() -> Self {
        Self {
            provider_name: "Claude".to_string(),
            icon: "🧠".to_string(),
            accounts: vec![
                AccountInfo {
                    id: "claude-1".to_string(),
                    email: "anthropic.team@organization.org".to_string(),
                    plan: "CLAUDE PRO".to_string(),
                    is_active: true,
                    status: "Optimal".to_string(),
                    quotas: vec![
                        ModelQuotaItem {
                            name: "claude-3-7-sonnet".to_string(),
                            percentage: 64,
                            detail: "32 / 50 msgs (5h window)".to_string(),
                            reset_in: "2h 45m".to_string(),
                        },
                        ModelQuotaItem {
                            name: "claude-3-5-haiku".to_string(),
                            percentage: 92,
                            detail: "460 / 500 msgs (Daily)".to_string(),
                            reset_in: "11h 10m".to_string(),
                        },
                    ],
                },
            ],
            selected_account_idx: 0,
            is_refreshing: false,
            last_refreshed: Some("5m ago".to_string()),
        }
    }

    pub fn next_account(&mut self) {
        if !self.accounts.is_empty() {
            self.selected_account_idx = (self.selected_account_idx + 1) % self.accounts.len();
        }
    }

    pub fn prev_account(&mut self) {
        if !self.accounts.is_empty() {
            if self.selected_account_idx == 0 {
                self.selected_account_idx = self.accounts.len().saturating_sub(1);
            } else {
                self.selected_account_idx -= 1;
            }
        }
    }

    pub fn set_active_account(&mut self) {
        let sel = self.selected_account_idx;
        for (i, acc) in self.accounts.iter_mut().enumerate() {
            acc.is_active = i == sel;
        }
    }

    pub fn active_account(&self) -> Option<&AccountInfo> {
        self.accounts.iter().find(|a| a.is_active)
    }

    pub fn selected_account(&self) -> Option<&AccountInfo> {
        self.accounts.get(self.selected_account_idx)
    }

    pub fn refresh(&mut self) {
        self.is_refreshing = false;
        self.last_refreshed = Some("Just now".to_string());
    }

    pub fn update_quota_summary(
        &mut self,
        input_tokens: usize,
        output_tokens: usize,
        total_tokens: usize,
        requests: usize,
    ) {
        self.is_refreshing = false;
        self.last_refreshed = Some("Just now".to_string());
        if let Some(acc) = self.accounts.first_mut() {
            let live_usage_name = "Live Core Usage";
            let usage_detail = format!(
                "Reqs: {} | In: {} | Out: {} | Total: {}",
                requests, input_tokens, output_tokens, total_tokens
            );
            let pct = (((total_tokens % 100_000) * 100) / 100_000) as u16;
            if let Some(item) = acc.quotas.iter_mut().find(|q| q.name == live_usage_name) {
                item.detail = usage_detail;
                item.percentage = pct.min(100);
            } else {
                acc.quotas.insert(
                    0,
                    ModelQuotaItem {
                        name: live_usage_name.to_string(),
                        percentage: pct.min(100),
                        detail: usage_detail,
                        reset_in: "Realtime".to_string(),
                    },
                );
            }
        }
    }

    pub fn update_live_usage(&mut self, account_id: &str, percentage: u8, live_usage_name: &str, usage_detail: String) {
        if let Some(acc) = self.accounts.iter_mut().find(|a| a.id == account_id) {
            let pct = percentage as u16;
            if let Some(item) = acc.quotas.iter_mut().find(|q| q.name == live_usage_name) {
                item.detail = usage_detail;
                item.percentage = pct.min(100);
            } else {
                acc.quotas.insert(
                    0,
                    ModelQuotaItem {
                        name: live_usage_name.to_string(),
                        percentage: pct.min(100),
                        detail: usage_detail,
                        reset_in: "Realtime".to_string(),
                    },
                );
            }
        }
    }
}
