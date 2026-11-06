use super::types::{AccountInfo, ModelQuotaItem, ProviderFocusArea, ProviderTabState};
use super::{antigravity, claude, codex};
use ragasukma_core::llm::providers::antigravity::provider::AntigravityProvider;
use ragasukma_core::llm::providers::claude::ClaudeProvider;
use ragasukma_core::llm::providers::codex::CodexProvider;

impl ProviderTabState {
    pub fn init_antigravity() -> Self {
        Self {
            provider_name: "Antigravity".to_string(),
            icon: String::new(),
            accounts: Vec::new(),
            selected_account_idx: 0,
            focus_area: ProviderFocusArea::Accounts,
            quota_scroll_offset: 0,
            last_refreshed: None,
            is_refreshing: false,
        }
    }

    pub fn init_codex() -> Self {
        Self {
            provider_name: "Codex".to_string(),
            icon: String::new(),
            accounts: Vec::new(),
            selected_account_idx: 0,
            focus_area: ProviderFocusArea::Accounts,
            quota_scroll_offset: 0,
            last_refreshed: None,
            is_refreshing: false,
        }
    }

    pub fn init_claude() -> Self {
        Self {
            provider_name: "Claude".to_string(),
            icon: String::new(),
            accounts: Vec::new(),
            selected_account_idx: 0,
            focus_area: ProviderFocusArea::Accounts,
            quota_scroll_offset: 0,
            last_refreshed: None,
            is_refreshing: false,
        }
    }

    pub fn reload_accounts(&mut self) {
        let updated = match self.provider_name.as_str() {
            name if name.contains("Antigravity") => antigravity::load_antigravity_tab(),
            name if name.contains("Codex") => codex::load_codex_tab(),
            name if name.contains("Claude") => claude::load_claude_tab(),
            _ => return,
        };

        let prev_idx = self.selected_account_idx;
        self.accounts = updated.accounts;
        if self.accounts.is_empty() {
            self.selected_account_idx = 0;
        } else if prev_idx >= self.accounts.len() {
            self.selected_account_idx = self.accounts.len() - 1;
        } else {
            self.selected_account_idx = prev_idx;
        }
    }

    pub fn selected_account(&self) -> Option<&AccountInfo> {
        self.accounts.get(self.selected_account_idx)
    }

    pub fn selected_account_mut(&mut self) -> Option<&mut AccountInfo> {
        self.accounts.get_mut(self.selected_account_idx)
    }

    pub fn next_account(&mut self) {
        if !self.accounts.is_empty() {
            self.selected_account_idx = (self.selected_account_idx + 1) % self.accounts.len();
            self.quota_scroll_offset = 0;
        }
    }

    pub fn prev_account(&mut self) {
        if !self.accounts.is_empty() {
            if self.selected_account_idx == 0 {
                self.selected_account_idx = self.accounts.len() - 1;
            } else {
                self.selected_account_idx -= 1;
            }
            self.quota_scroll_offset = 0;
        }
    }

    pub fn scroll_quota_down(&mut self, max_visible: usize) {
        let quota_count = self.selected_account().map_or(0, |a| a.quotas.len());
        if quota_count > max_visible {
            let max_scroll = quota_count.saturating_sub(max_visible);
            if self.quota_scroll_offset < max_scroll {
                self.quota_scroll_offset = (self.quota_scroll_offset + 1).min(max_scroll);
            }
        }
    }

    pub fn scroll_quota_up(&mut self) {
        if self.quota_scroll_offset > 0 {
            self.quota_scroll_offset = self.quota_scroll_offset.saturating_sub(1);
        }
    }

    pub fn toggle_focus(&mut self) {
        self.focus_area = match self.focus_area {
            ProviderFocusArea::Accounts => ProviderFocusArea::Quotas,
            ProviderFocusArea::Quotas => ProviderFocusArea::Accounts,
        };
    }

    pub fn set_focus(&mut self, area: ProviderFocusArea) {
        self.focus_area = area;
    }

    pub fn reset_quota_scroll(&mut self) {
        self.quota_scroll_offset = 0;
    }

    pub fn update_account_quotas(&mut self, account_id: &str, quotas: Vec<ModelQuotaItem>) {
        if let Some(acc) = self.accounts.iter_mut().find(|a| a.id == account_id) {
            acc.quotas = quotas;
        }
    }

    pub fn toggle_selected_account(&mut self) -> Option<bool> {
        if self.accounts.is_empty() {
            return None;
        }
        let acc_id = self.accounts.get(self.selected_account_idx)?.id.clone();

        if self.provider_name.contains("Antigravity") {
            let mut provider = AntigravityProvider::new();
            if let Ok(new_status) = provider.toggle_account_by_id(&acc_id) {
                if let Some(a) = self.accounts.get_mut(self.selected_account_idx) {
                    a.is_active = new_status;
                    a.status = if new_status {
                        "active".to_string()
                    } else {
                        "inactive".to_string()
                    };
                }
                return Some(new_status);
            }
        } else if self.provider_name.contains("Codex") {
            let mut provider = CodexProvider::new();
            if let Ok(new_status) = provider.toggle_account_by_id(&acc_id) {
                if let Some(a) = self.accounts.get_mut(self.selected_account_idx) {
                    a.is_active = new_status;
                    a.status = if new_status {
                        "active".to_string()
                    } else {
                        "inactive".to_string()
                    };
                }
                return Some(new_status);
            }
        } else if self.provider_name.contains("Claude") {
            let mut provider = ClaudeProvider::new();
            if let Ok(new_status) = provider.toggle_account_by_id(&acc_id) {
                if let Some(a) = self.accounts.get_mut(self.selected_account_idx) {
                    a.is_active = new_status;
                    a.status = if new_status {
                        "active".to_string()
                    } else {
                        "inactive".to_string()
                    };
                }
                return Some(new_status);
            }
        }
        None
    }

    pub fn set_selected_account_active(&mut self) -> Option<()> {
        if self.accounts.is_empty() {
            return None;
        }
        let acc_id = self.accounts.get(self.selected_account_idx)?.id.clone();

        if self.provider_name.contains("Antigravity") {
            let mut provider = AntigravityProvider::new();
            let mut found = false;
            for a in &mut provider.accounts {
                if a.id == acc_id {
                    a.is_active = true;
                    found = true;
                }
            }
            if found && provider.save_accounts().is_ok() {
                for (i, a) in self.accounts.iter_mut().enumerate() {
                    let is_target = i == self.selected_account_idx;
                    a.is_active = is_target;
                    a.status = if is_target {
                        "active".to_string()
                    } else {
                        "inactive".to_string()
                    };
                }
                return Some(());
            }
        } else if self.provider_name.contains("Codex") {
            let mut provider = CodexProvider::new();
            let mut found = false;
            for a in &mut provider.accounts {
                if a.id == acc_id {
                    a.is_active = true;
                    found = true;
                }
            }
            if found && provider.save_accounts().is_ok() {
                for (i, a) in self.accounts.iter_mut().enumerate() {
                    let is_target = i == self.selected_account_idx;
                    a.is_active = is_target;
                    a.status = if is_target {
                        "active".to_string()
                    } else {
                        "inactive".to_string()
                    };
                }
                return Some(());
            }
        } else if self.provider_name.contains("Claude") {
            let mut provider = ClaudeProvider::new();
            let mut found = false;
            for a in &mut provider.accounts {
                if a.id == acc_id {
                    a.is_active = true;
                    found = true;
                }
            }
            if found && provider.save_accounts().is_ok() {
                for (i, a) in self.accounts.iter_mut().enumerate() {
                    let is_target = i == self.selected_account_idx;
                    a.is_active = is_target;
                    a.status = if is_target {
                        "active".to_string()
                    } else {
                        "inactive".to_string()
                    };
                }
                return Some(());
            }
        }
        None
    }

    pub fn remove_selected_account(&mut self) -> Option<String> {
        if self.accounts.is_empty() {
            return None;
        }
        let acc = self.accounts.get(self.selected_account_idx)?;
        let acc_id = acc.id.clone();
        let acc_email = acc.email.clone();
        let display_name = if !acc_email.is_empty() {
            acc_email
        } else {
            acc_id.clone()
        };

        let removed = if self.provider_name.contains("Antigravity") {
            AntigravityProvider::new().remove_account(&acc_id)
        } else if self.provider_name.contains("Codex") {
            CodexProvider::new().remove_account(&acc_id)
        } else if self.provider_name.contains("Claude") {
            ClaudeProvider::new().remove_account(&acc_id)
        } else {
            false
        };

        if removed {
            self.accounts.remove(self.selected_account_idx);
            if self.selected_account_idx >= self.accounts.len() && !self.accounts.is_empty() {
                self.selected_account_idx = self.accounts.len() - 1;
            }
            Some(display_name)
        } else {
            None
        }
    }
}
