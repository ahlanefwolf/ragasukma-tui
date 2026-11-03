use ragasukma_core::llm::providers::custom::{
    load_accounts, save_accounts, validate_account, CustomLlmAccount,
};

use super::types::{CustomLlmTabState, FormField};

impl CustomLlmTabState {
    pub fn new() -> Self {
        let accounts = load_accounts();
        let models = Vec::new();

        Self {
            accounts,
            selected_idx: 0,
            models,
            is_fetching_models: false,
            is_form_open: false,
            editing_prefix: None,
            form_prefix: String::new(),
            form_name: String::new(),
            form_base_url: String::new(),
            form_api_key: String::new(),
            form_enabled: true,
            focused_field: FormField::Prefix,
            form_error: None,
            delete_confirm_prefix: None,
        }
    }

    #[must_use]
    pub fn selected_account(&self) -> Option<&CustomLlmAccount> {
        self.accounts.get(self.selected_idx)
    }

    pub fn select_next(&mut self) {
        if !self.accounts.is_empty() {
            self.selected_idx = (self.selected_idx + 1) % self.accounts.len();
        }
    }

    pub fn select_prev(&mut self) {
        if !self.accounts.is_empty() {
            if self.selected_idx == 0 {
                self.selected_idx = self.accounts.len() - 1;
            } else {
                self.selected_idx -= 1;
            }
        }
    }

    pub fn toggle_enabled(&mut self) -> Option<String> {
        if let Some(acc) = self.accounts.get_mut(self.selected_idx) {
            acc.enabled = !acc.enabled;
            let status = if acc.enabled { "Enabled" } else { "Disabled" };
            let msg = format!("{} [{}] is now {}", acc.name, acc.prefix, status);
            let _ = save_accounts(&self.accounts);
            Some(msg)
        } else {
            None
        }
    }

    pub fn open_add_form(&mut self) {
        self.editing_prefix = None;
        self.form_prefix.clear();
        self.form_name.clear();
        self.form_base_url = "https://".to_string();
        self.form_api_key.clear();
        self.form_enabled = true;
        self.focused_field = FormField::Prefix;
        self.form_error = None;
        self.is_form_open = true;
    }

    pub fn open_edit_form(&mut self) {
        let acc_opt = self.selected_account().cloned();
        if let Some(acc) = acc_opt {
            self.editing_prefix = Some(acc.prefix.clone());
            self.form_prefix = acc.prefix.clone();
            self.form_name = acc.name.clone();
            self.form_base_url = acc.base_url.clone();
            self.form_api_key = acc.api_key.clone();
            self.form_enabled = acc.enabled;
            self.focused_field = FormField::Name;
            self.form_error = None;
            self.is_form_open = true;
        }
    }

    pub fn refresh_models(&mut self) {
        let fetched = ragasukma_core::llm::providers::custom::fetch_models();
        if !fetched.is_empty() {
            self.models = fetched;
        }
    }

    pub fn close_form(&mut self) {
        self.is_form_open = false;
        self.form_error = None;
    }

    pub fn form_next_field(&mut self) {
        self.focused_field = self.focused_field.next();
    }

    pub fn form_prev_field(&mut self) {
        self.focused_field = self.focused_field.prev();
    }

    pub fn form_input(&mut self, c: char) {
        match self.focused_field {
            FormField::Prefix => {
                if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                    self.form_prefix.push(c.to_ascii_lowercase());
                }
            }
            FormField::Name => self.form_name.push(c),
            FormField::BaseUrl => self.form_base_url.push(c),
            FormField::ApiKey => self.form_api_key.push(c),
            FormField::Enabled => {
                if c == ' ' {
                    self.form_enabled = !self.form_enabled;
                }
            }
            FormField::SaveBtn | FormField::CancelBtn => {}
        }
    }

    pub fn form_backspace(&mut self) {
        match self.focused_field {
            FormField::Prefix => {
                self.form_prefix.pop();
            }
            FormField::Name => {
                self.form_name.pop();
            }
            FormField::BaseUrl => {
                self.form_base_url.pop();
            }
            FormField::ApiKey => {
                self.form_api_key.pop();
            }
            _ => {}
        }
    }

    pub fn form_toggle_enabled(&mut self) {
        self.form_enabled = !self.form_enabled;
    }

    pub fn form_submit(&mut self) -> Result<String, String> {
        let candidate = CustomLlmAccount {
            prefix: self.form_prefix.trim().to_string(),
            name: self.form_name.trim().to_string(),
            base_url: self.form_base_url.trim().to_string(),
            api_key: self.form_api_key.trim().to_string(),
            enabled: self.form_enabled,
        };

        if let Err(err) = validate_account(&candidate) {
            let msg = err.to_string();
            self.form_error = Some(msg.clone());
            return Err(msg);
        }

        let is_editing = self.editing_prefix.is_some();
        let original_prefix = self.editing_prefix.as_deref().unwrap_or("");

        if (!is_editing || original_prefix != candidate.prefix)
            && self.accounts.iter().any(|a| a.prefix == candidate.prefix)
        {
            let err = format!("Prefix '{}' is already in use by another provider", candidate.prefix);
            self.form_error = Some(err.clone());
            return Err(err);
        }

        if let Some(orig) = &self.editing_prefix {
            if let Some(idx) = self.accounts.iter().position(|a| &a.prefix == orig) {
                self.accounts[idx] = candidate.clone();
                self.selected_idx = idx;
            }
        } else {
            self.accounts.push(candidate.clone());
            self.selected_idx = self.accounts.len() - 1;
        }

        if let Err(err) = save_accounts(&self.accounts) {
            let msg = format!("Failed to save config: {err}");
            self.form_error = Some(msg.clone());
            return Err(msg);
        }

        let act = if is_editing { "Updated" } else { "Saved" };
        let name = candidate.name;
        self.is_form_open = false;
        self.form_error = None;
        Ok(format!("{act} custom provider '{name}' successfully"))
    }

    pub fn open_delete_confirm(&mut self) -> bool {
        if let Some(acc) = self.selected_account() {
            self.delete_confirm_prefix = Some(acc.prefix.clone());
            true
        } else {
            false
        }
    }

    pub fn confirm_delete(&mut self) -> Option<String> {
        if let Some(prefix) = self.delete_confirm_prefix.take() {
            if let Some(pos) = self.accounts.iter().position(|a| a.prefix == prefix) {
                let removed = self.accounts.remove(pos);
                let _ = save_accounts(&self.accounts);
                if self.selected_idx >= self.accounts.len() && !self.accounts.is_empty() {
                    self.selected_idx = self.accounts.len() - 1;
                }
                Some(format!("Deleted custom provider '{}'", removed.name))
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn cancel_delete(&mut self) {
        self.delete_confirm_prefix = None;
    }

    #[must_use]
    pub fn models_for_prefix(&self, prefix: &str) -> Vec<&(String, String)> {
        self.models.iter().filter(|(_, p)| p == prefix).collect()
    }
}
