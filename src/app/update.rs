use crate::agent_view::types::AgentFocusArea;
use crate::app::state::App;
use crate::app::types::{Action, DiffApprovalState};

impl App {
    pub fn update(&mut self, action: Action) {
        // 1. Delegate to Agent View module
        if let Some(status_opt) = crate::agent_view::update::handle_agent_action(
            &mut self.agent,
            action.clone(),
            self.bridge_cmd_tx.as_ref(),
        ) {
            if let Some(status) = status_opt {
                self.set_status(status);
            }
            return;
        }

        // 2. Delegate to Custom LLM View module
        if let Some(status_opt) =
            crate::custom_llm_view::update::handle_custom_llm_action(&mut self.custom_llm, &action)
        {
            if let Some(status) = status_opt {
                self.set_status(status);
            }
            return;
        }

        // 3. Delegate to Provider View module
        if let Some(status_opt) = crate::provider_view::update::handle_provider_action(
            self.active_tab,
            &mut self.antigravity,
            &mut self.codex,
            &mut self.claude,
            &action,
            self.bridge_cmd_tx.as_ref(),
        ) {
            if let Some(status) = status_opt {
                self.set_status(status);
            }
            if matches!(action, Action::RefreshQuota) {
                self.force_refresh_selected_account_quota();
            }
            return;
        }

        // 4. Tangani Action Root / Global App Level
        match action {
            Action::Quit => {
                self.should_quit = true;
            }
            Action::NextTab => {
                self.active_tab = (self.active_tab + 1) % 5;
                if self.active_tab >= 1 && self.active_tab <= 3 {
                    self.request_selected_account_quota();
                }
            }
            Action::PrevTab => {
                self.active_tab = if self.active_tab == 0 {
                    4
                } else {
                    self.active_tab - 1
                };
                if self.active_tab >= 1 && self.active_tab <= 3 {
                    self.request_selected_account_quota();
                }
            }
            Action::SwitchTab(tab) => {
                if tab < 5 {
                    self.active_tab = tab;
                    if (1..=3).contains(&tab) {
                        self.request_selected_account_quota();
                    }
                }
            }
            Action::CloseModal => {
                self.error_modal = None;
            }
            Action::SetStatus(msg) => {
                self.set_status(msg);
            }
            Action::FocusNext => match self.active_tab {
                0 => self.agent.cycle_focus_next(),
                1 => {
                    self.antigravity.next_account();
                    self.request_selected_account_quota();
                }
                2 => {
                    self.codex.next_account();
                    self.request_selected_account_quota();
                }
                3 => {
                    self.claude.next_account();
                    self.request_selected_account_quota();
                }
                4 => self.custom_llm.select_next(),
                _ => {}
            },
            Action::FocusPrev => match self.active_tab {
                0 => self.agent.cycle_focus_prev(),
                1 => {
                    self.antigravity.prev_account();
                    self.request_selected_account_quota();
                }
                2 => {
                    self.codex.prev_account();
                    self.request_selected_account_quota();
                }
                3 => {
                    self.claude.prev_account();
                    self.request_selected_account_quota();
                }
                4 => self.custom_llm.select_prev(),
                _ => {}
            },
            Action::Select => match self.active_tab {
                4 => {
                    self.custom_llm.toggle_enabled();
                }
                0 if self.agent.focus_area == AgentFocusArea::Files => {
                    self.agent.insert_selected_file_to_input();
                }
                _ => {}
            },
            Action::ConfirmOk => {
                if let Some(dialog) = self.confirm_dialog.take() {
                    self.update(*dialog.on_confirm_action);
                }
            }
            Action::ConfirmCancel | Action::CancelDelete => {
                self.confirm_dialog = None;
            }
            Action::ShowError(err) => {
                tracing::error!("Application error modal: {err}");
                self.error_modal = Some(err);
            }
            Action::StartAntigravityOAuth => {
                let port = 51121;
                let state = format!(
                    "ragasukma_{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs()
                );
                let redirect_uri = format!("http://localhost:{}/oauth/callback", port);
                let auth_url =
                    ragasukma_core::llm::providers::antigravity::oauth::build_auth_url(
                        &redirect_uri,
                        &state,
                    );

                if let Some(event_tx) = self.event_tx.clone() {
                    crate::oauth_listener::spawn_oauth_listener(
                        crate::oauth_listener::OAuthProviderKind::Antigravity,
                        port,
                        state,
                        event_tx,
                    );
                }

                crate::oauth_listener::open_browser_oauth(&auth_url);
                self.set_status(format!("OAuth Antigravity dibuka di browser (port {port})"));
            }
            Action::StartCodexOAuth => {
                let port = 1455;
                let (code_verifier, code_challenge) =
                    ragasukma_core::llm::providers::codex::oauth::generate_pkce();
                let state = format!(
                    "ragasukma_codex_{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs()
                );
                ragasukma_core::llm::providers::codex::oauth::store_oauth_state(
                    &state,
                    &code_verifier,
                );
                let redirect_uri =
                    ragasukma_core::llm::providers::codex::constants::CODEX_REDIRECT_URI;
                let auth_url = ragasukma_core::llm::providers::codex::oauth::build_auth_url(
                    redirect_uri,
                    &state,
                    &code_challenge,
                );

                if let Some(event_tx) = self.event_tx.clone() {
                    crate::oauth_listener::spawn_oauth_listener(
                        crate::oauth_listener::OAuthProviderKind::Codex,
                        port,
                        state,
                        event_tx,
                    );
                }

                crate::oauth_listener::open_browser_oauth(&auth_url);
                self.set_status(format!("OAuth Codex dibuka di browser (port {port})"));
            }
            Action::StartClaudeOAuth => {
                let port = 54321;
                let redirect_uri = format!("http://localhost:{}/oauth/callback", port);
                let (code_verifier, code_challenge) =
                    ragasukma_core::llm::providers::claude::oauth::generate_pkce();
                let state = format!(
                    "claude_oauth_{}_{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                    std::process::id()
                );
                ragasukma_core::llm::providers::claude::oauth::store_oauth_state(
                    &state,
                    &code_verifier,
                );
                let auth_url = ragasukma_core::llm::providers::claude::oauth::build_auth_url(
                    &redirect_uri,
                    &state,
                    &code_challenge,
                );

                if let Some(event_tx) = self.event_tx.clone() {
                    crate::oauth_listener::spawn_oauth_listener(
                        crate::oauth_listener::OAuthProviderKind::Claude,
                        port,
                        state,
                        event_tx,
                    );
                }

                crate::oauth_listener::open_browser_oauth(&auth_url);
                self.set_status(format!("OAuth Claude dibuka di browser (port {port})"));
            }
            Action::ShowDiffPreview {
                file_path,
                tool_name,
                diff_lines,
            } => {
                self.diff_modal = Some(DiffApprovalState {
                    tool_call_id: "diff_preview".to_string(),
                    tool_name,
                    file_path,
                    diff_lines,
                    scroll_offset: 0,
                });
            }
            Action::DiffScrollUp => {
                if let Some(ref mut d) = self.diff_modal {
                    d.scroll_up();
                }
            }
            Action::DiffScrollDown => {
                if let Some(ref mut d) = self.diff_modal {
                    d.scroll_down();
                }
            }
            Action::DiffApprove => {
                self.diff_modal = None;
                self.set_status("Patch/Diff Disetujui".to_string());
            }
            Action::DiffReject => {
                self.diff_modal = None;
                self.set_status("Patch/Diff Ditolak".to_string());
            }
            Action::BridgeEvent(ev) => {
                self.handle_bridge_event(ev);
            }
            Action::BridgeCommand(cmd) => {
                if let Some(tx) = &self.bridge_cmd_tx {
                    if let Err(e) = tx.send(cmd) {
                        tracing::error!("Failed to dispatch bridge command: {e}");
                    }
                } else {
                    tracing::warn!("Bridge command sender is unavailable");
                }
            }
            _ => {}
        }
    }
}