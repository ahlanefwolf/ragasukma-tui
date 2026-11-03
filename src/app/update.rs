use crate::agent_view::types::{AgentFocusArea, ChatMessage, MessageRole};
use crate::app::state::{current_time_str, App};
use crate::app::types::{Action, DiffApprovalState};
use crate::core_bridge::CoreBridgeCommand;

impl App {
    pub fn update(&mut self, action: Action) {
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
            Action::ToggleHelp => {
                self.show_help = !self.show_help;
            }
            Action::CloseModal => {
                self.show_help = false;
                self.confirm_dialog = None;
                self.diff_modal = None;
                self.error_modal = None;
            }
            Action::SetStatus(msg) => {
                self.set_status(msg);
            }
            Action::FocusNext => {
                match self.active_tab {
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
                }
            }
            Action::FocusPrev => {
                match self.active_tab {
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
                }
            }
            Action::Select => match self.active_tab {
                4 => {
                    self.custom_llm.toggle_enabled();
                }
                0 if self.agent.focus_area == AgentFocusArea::Files => {
                    self.agent.insert_selected_file_to_input();
                }
                _ => {}
            },
            Action::ToggleAccountActive => match self.active_tab {
                1 => {
                    if let Some(status) = self.antigravity.toggle_selected_account() {
                        let status_label = if status { "Aktif" } else { "Nonaktif" };
                        self.set_status(format!("Status akun Antigravity diubah: {status_label}"));
                    }
                }
                2 => {
                    if let Some(status) = self.codex.toggle_selected_account() {
                        let status_label = if status { "Aktif" } else { "Nonaktif" };
                        self.set_status(format!("Status akun Codex diubah: {status_label}"));
                    }
                }
                3 => {
                    if let Some(status) = self.claude.toggle_selected_account() {
                        let status_label = if status { "Aktif" } else { "Nonaktif" };
                        self.set_status(format!("Status akun Claude diubah: {status_label}"));
                    }
                }
                _ => {}
            },
            Action::SetAccountActive => match self.active_tab {
                1 if self.antigravity.set_selected_account_active().is_some() => {
                    self.set_status("Akun Antigravity disetel sebagai Default".to_string());
                }
                2 if self.codex.set_selected_account_active().is_some() => {
                    self.set_status("Akun Codex disetel sebagai Default".to_string());
                }
                3 if self.claude.set_selected_account_active().is_some() => {
                    self.set_status("Akun Claude disetel sebagai Default".to_string());
                }
                _ => {}
            },
            Action::RefreshQuota => {
                self.set_status("Menyegarkan kuota model provider...".to_string());
                self.request_selected_account_quota();
                if let Some(tx) = &self.bridge_cmd_tx {
                    let prov = match self.active_tab {
                        1 => "antigravity",
                        2 => "codex",
                        3 => "claude",
                        _ => "",
                    };
                    if !prov.is_empty() {
                        let _ = tx.send(CoreBridgeCommand::RefreshQuota {
                            provider: prov.to_string(),
                        });
                    }
                }
            }
            Action::ConfirmOk => {
                if let Some(dialog) = self.confirm_dialog.take() {
                    self.update(*dialog.on_confirm_action);
                }
            }
            Action::ConfirmCancel => {
                self.confirm_dialog = None;
            }
            Action::ShowError(err) => {
                tracing::error!("Application error modal: {err}");
                self.error_modal = Some(err);
            }
            Action::ConfirmDelete => match self.active_tab {
                4 => {
                    self.custom_llm.confirm_delete();
                }
                1 => {
                    if let Some(removed_name) = self.antigravity.remove_selected_account() {
                        self.set_status(format!(
                            "Akun Antigravity '{}' berhasil dihapus",
                            removed_name
                        ));
                    }
                }
                2 => {
                    if let Some(removed_name) = self.codex.remove_selected_account() {
                        self.set_status(format!("Akun Codex '{}' berhasil dihapus", removed_name));
                    }
                }
                3 => {
                    if let Some(removed_name) = self.claude.remove_selected_account() {
                        self.set_status(format!("Akun Claude '{}' berhasil dihapus", removed_name));
                    }
                }
                _ => {}
            },
            Action::CancelDelete => {
                self.confirm_dialog = None;
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

            Action::AgentInsertChar(c) => {
                self.agent.insert_char(c);
                self.agent.check_and_update_tag_query();
            }
            Action::AgentBackspace => {
                self.agent.backspace();
                self.agent.check_and_update_tag_query();
            }
            Action::AgentClearInput => {
                self.agent.clear_input();
                self.agent.close_tag_query();
            }
            Action::AgentClearChat => self.agent.clear_chat(),
            Action::AgentSubmitPrompt => {
                self.agent.close_tag_query();
                if let Some(prompt) = self.agent.submit_prompt() {
                    if let Some(tx) = &self.bridge_cmd_tx {
                        let _ = tx.send(CoreBridgeCommand::ExecuteTurn {
                            prompt: prompt.clone(),
                            model_override: None,
                        });
                    }
                    self.agent.messages.push(ChatMessage {
                        role: MessageRole::User,
                        sender_name: "You".to_string(),
                        content: prompt,
                        timestamp: current_time_str(),
                        tool_calls: Vec::new(),
                        reasoning: None,
                    });
                }
            }
            Action::AgentFocusInput => self.agent.focus_input(),
            Action::AgentUnfocusInput => self.agent.unfocus_input(),
            Action::AgentFocusFiles => self.agent.focus_area = AgentFocusArea::Files,
            Action::AgentFocusChat => self.agent.focus_area = AgentFocusArea::Chat,
            Action::AgentFocusNextPanel => self.agent.cycle_focus_next(),
            Action::AgentFocusPrevPanel => self.agent.cycle_focus_prev(),
            Action::AgentSelectFileNext => self.agent.select_file_next(),
            Action::AgentSelectFilePrev => self.agent.select_file_prev(),
            Action::AgentInsertSelectedFile => self.agent.insert_selected_file_to_input(),
            Action::AgentToggleFileTreeExpand => self.agent.toggle_file_tree_expand(),
            Action::AgentScanFiles => self.agent.scan_workspace_files(),
            Action::AgentScrollUp => self.agent.scroll_up(),
            Action::AgentScrollDown => self.agent.scroll_down(),
            Action::AgentToggleToolExpand => self.agent.toggle_tool_expand(),

            // Phase 1 Foundations
            Action::AgentTagQueryNext => self.agent.select_tag_query_next(),
            Action::AgentTagQueryPrev => self.agent.select_tag_query_prev(),
            Action::AgentApplyTag => {
                self.agent.apply_selected_tag();
            }
            Action::AgentCloseTagQuery => self.agent.close_tag_query(),

            // Phase 2 Chat Streaming & Reasoning
            Action::AgentToggleReasoning => {
                self.agent.toggle_reasoning();
                let status = if self.agent.reasoning_expanded {
                    "Reasoning: Ditampilkan"
                } else {
                    "Reasoning: Disembunyikan (collapsed)"
                };
                self.set_status(status.to_string());
            }
            Action::AgentScrollToTop => {
                self.agent.scroll_to_top();
                self.set_status("Scroll ke atas [Lock auto-scroll aktif]".to_string());
            }
            Action::AgentScrollToBottom => {
                self.agent.scroll_to_bottom();
                self.set_status("Scroll ke bawah [Auto-scroll aktif]".to_string());
            }

            // Phase 3 Tool Execution & Approval
            Action::AgentApprovePending => {
                if let Some(req) = self.agent.approve_pending() {
                    self.set_status(format!("Eksekusi disetujui: {}", req.tool_name));
                }
            }
            Action::AgentRejectPending => {
                if let Some(req) = self.agent.reject_pending() {
                    self.set_status(format!("Eksekusi ditolak: {}", req.tool_name));
                }
            }
            Action::AgentCycleToolFilter => {
                self.agent.cycle_tool_filter();
                self.set_status(format!("Tool Filter: {:?}", self.agent.tool_filter));
            }
            Action::AgentToggleToolCard(idx) => {
                self.agent.toggle_tool_card(idx);
            }

            // Phase 4 Multi-File Context Manager & Token Gauge
            Action::AgentTogglePinSelectedFile => {
                self.agent.toggle_pin_selected_file();
                self.set_status("Toggled context file pin".to_string());
            }
            Action::AgentRemoveSelectedContextFile => {
                let idx = self.agent.selected_context_file_idx;
                self.agent.remove_context_file(idx);
                self.set_status("Removed context file from prompt".to_string());
            }
            Action::AgentNextContextFile => {
                self.agent.next_context_file();
            }
            Action::AgentPrevContextFile => {
                self.agent.prev_context_file();
            }
            Action::AgentCancelTurn => {
                if let Some(tx) = &self.bridge_cmd_tx {
                    let _ = tx.send(CoreBridgeCommand::CancelTurn);
                }
                self.agent.cancel_turn();
                self.set_status("Cancelling turn...".to_string());
            }
            Action::AgentRetryTurn => {
                if let Some(tx) = &self.bridge_cmd_tx {
                    let _ = tx.send(CoreBridgeCommand::RetryTurn);
                }
                self.agent.start_turn();
                self.set_status("Retrying turn...".to_string());
            }
            Action::AgentToggleSidebar => {
                self.agent.toggle_sidebar();
                let status_msg = if self.agent.is_sidebar_visible {
                    "Sidebar opened"
                } else {
                    "Sidebar hidden"
                };
                self.set_status(status_msg.to_string());
            }
            Action::AgentFocusPrompt => {
                self.agent.focus_input();
                self.set_status("Prompt focused".to_string());
            }
            Action::AgentCycleModel => {
                self.agent.cycle_model();
                let model_name = self.agent.active_model.clone();
                self.set_status(format!("Selected model: {}", model_name));
            }

            Action::FormChar(c) => self.custom_llm.form_input(c),
            Action::FormBackspace => self.custom_llm.form_backspace(),
            Action::FormNextField => self.custom_llm.form_next_field(),
            Action::FormPrevField => self.custom_llm.form_prev_field(),
            Action::FormSubmit => {
                match self.custom_llm.form_submit() {
                    Ok(msg) => {
                        tracing::info!("Custom LLM endpoint saved: {msg}");
                        self.set_status(msg);
                    }
                    Err(err) => {
                        tracing::warn!("Custom LLM form submit error: {err}");
                        self.set_status(format!("Gagal: {err}"));
                    }
                }
            }
            Action::CancelForm => self.custom_llm.close_form(),
            Action::CustomLlmAdd => {
                self.custom_llm.open_add_form();
                self.set_status("Tambah Custom LLM Endpoint".to_string());
            }
            Action::CustomLlmEdit => {
                self.custom_llm.open_edit_form();
                self.set_status("Edit Custom LLM Endpoint".to_string());
            }
            Action::CustomLlmToggle => {
                if let Some(msg) = self.custom_llm.toggle_enabled() {
                    tracing::info!("{msg}");
                    self.set_status(msg);
                }
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
        }
    }
}