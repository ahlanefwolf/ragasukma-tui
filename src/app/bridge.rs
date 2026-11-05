use crate::agent_view::types::{ToolCallInfo, ToolExecutionStatus};
use crate::app::state::App;
use crate::core_bridge::{CoreBridgeCommand, CoreBridgeEvent};

impl App {
    pub fn handle_bridge_event(&mut self, ev: CoreBridgeEvent) {
        match ev {
            CoreBridgeEvent::Connected { session_id, .. } => {
                self.set_status(format!("Agent ready (session {})", session_id));
            }
            CoreBridgeEvent::StatusChange { session_id, status } => {
                if let Some(ref sid) = session_id
                    && status == "running"
                    && let Some(s) = self.agent.sessions.iter_mut().find(|s| &s.id == sid)
                {
                    s.is_running = true;
                }
                if session_id.as_ref() == self.agent.active_session_id.as_ref() || session_id.is_none() {
                    self.set_status(status);
                }
            }
            CoreBridgeEvent::Delta { session_id, delta } => {
                if let Some(s) = self.agent.sessions.iter_mut().find(|s| s.id == session_id) {
                    s.is_running = true;
                }
                if Some(&session_id) == self.agent.active_session_id.as_ref() {
                    self.agent.is_running = true;
                    self.agent.append_chunk(&delta);
                }
            }
            CoreBridgeEvent::ThoughtDelta { session_id, thought } => {
                if let Some(s) = self.agent.sessions.iter_mut().find(|s| s.id == session_id) {
                    s.is_running = true;
                }
                if Some(&session_id) == self.agent.active_session_id.as_ref() {
                    self.agent.is_running = true;
                    self.agent.append_thought(&thought);
                }
            }
            CoreBridgeEvent::StepInfo { session_id, action, details, .. } => {
                let det_str = details.unwrap_or_default();
                tracing::debug!("Agent step: session={session_id}, action='{action}', details='{det_str}'");
                if Some(&session_id) == self.agent.active_session_id.as_ref() {
                    self.set_status(format!("[Step] {} - {}", action, det_str));
                }
            }
            CoreBridgeEvent::ToolCall { session_id, name, args } => {
                tracing::info!("Tool call received: session={session_id}, name='{name}', args='{args}'");
                if Some(&session_id) == self.agent.active_session_id.as_ref()
                    && let Some(last) = self.agent.messages.last_mut() {
                        last.tool_calls.push(ToolCallInfo {
                            id: format!("call_{}", last.tool_calls.len() + 1),
                            name,
                            args_summary: args,
                            output_summary: None,
                            status: ToolExecutionStatus::Success,
                            duration_ms: None,
                            stdout: None,
                            stderr: None,
                            exit_code: None,
                            diff_preview: None,
                        });
                    }
            }
            CoreBridgeEvent::TurnCompleted { session_id, summary: msg } => {
                tracing::info!("Turn completed for {session_id}: {msg}");
                let is_active = Some(&session_id) == self.agent.active_session_id.as_ref();
                if let Some(s) = self.agent.sessions.iter_mut().find(|s| s.id == session_id) {
                    s.is_running = false;
                }
                if is_active {
                    if !msg.trim().is_empty() {
                        let mut found = false;
                        if let Some(last) = self
                            .agent
                            .messages
                            .last_mut()
                            .filter(|m| m.role == crate::agent_view::types::MessageRole::Assistant && m.content.is_empty())
                        {
                            last.content = msg.clone();
                            found = true;
                        }
                        if !found {
                            self.agent.push_message(crate::agent_view::types::ChatMessage {
                                role: crate::agent_view::types::MessageRole::Assistant,
                                sender_name: "Agent".to_string(),
                                content: msg,
                                timestamp: "now".to_string(),
                                tool_calls: Vec::new(),
                                reasoning: None,
                            });
                        }
                    }
                    self.agent.finish_turn();
                    self.set_status("Turn completed".to_string());
                    if let Some(tx) = self.bridge_cmd_tx.as_ref() {
                        let _ = tx.send(crate::core_bridge::types::CoreBridgeCommand::LoadHistory {
                            session_id,
                        });
                    }
                } else {
                    let s_title = self
                        .agent
                        .sessions
                        .iter()
                        .find(|s| s.id == session_id)
                        .map(|s| s.title.clone())
                        .unwrap_or_else(|| "Task".to_string());
                    self.set_status(format!("Task in '{}' completed", s_title));
                }
            }
            CoreBridgeEvent::TurnCancelled { session_id } => {
                tracing::warn!("Turn cancelled for session: {session_id}");
                if let Some(s) = self.agent.sessions.iter_mut().find(|s| s.id == session_id) {
                    s.is_running = false;
                }
                if Some(&session_id) == self.agent.active_session_id.as_ref() {
                    self.agent.cancel_turn();
                }
                self.set_status(format!("Turn cancelled for session: {}", session_id));
            }
            CoreBridgeEvent::TurnFailed { error, session_id } => {
                tracing::error!("Turn failed for session {session_id}: {error}");
                if let Some(s) = self.agent.sessions.iter_mut().find(|s| s.id == session_id) {
                    s.is_running = false;
                }
                if Some(&session_id) == self.agent.active_session_id.as_ref() {
                    self.agent.push_message(crate::agent_view::types::ChatMessage {
                        role: crate::agent_view::types::MessageRole::Assistant,
                        sender_name: "System Error".to_string(),
                        content: format!("⚠️ Error: {}", error),
                        timestamp: "now".to_string(),
                        tool_calls: Vec::new(),
                        reasoning: None,
                    });
                    self.agent.fail_turn(error.clone());
                }
                self.set_status(format!("Turn failed: {}", error));
            }
            CoreBridgeEvent::QuotaUpdated {
                provider,
                total_tokens,
                ..
            } => {
                self.set_status(format!("Quota updated: {} total: {}", provider, total_tokens));
            }
            CoreBridgeEvent::AccountQuotaUpdated {
                provider,
                account_id,
                quotas,
            } => {
                let p = match provider.to_lowercase().as_str() {
                    "antigravity" => &mut self.antigravity,
                    "codex" => &mut self.codex,
                    "claude" => &mut self.claude,
                    _ => return,
                };
                if let Some(acc) = p.accounts.iter_mut().find(|a| a.id == account_id) {
                    acc.quotas = quotas;
                }
            }
            CoreBridgeEvent::ProjectsLoaded {
                projects,
                active_id,
                active_name,
            } => {
                let initial_root = projects
                    .iter()
                    .find(|p| p.id == active_id)
                    .and_then(|p| p.root_path.clone());

                self.agent.projects = projects
                    .into_iter()
                    .filter(|p| {
                        let lower_name = p.name.to_lowercase();
                        let lower_id = p.id.to_lowercase();
                        lower_id != "default"
                            && !lower_name.contains("workspace default")
                            && !lower_name.contains("default workspace")
                    })
                    .map(|p| crate::agent_view::types::ProjectItem {
                        id: p.id,
                        name: p.name,
                        description: p.description,
                        root_path: p.root_path,
                    })
                    .collect();
                if !active_id.is_empty() {
                    self.agent.active_project_id = Some(active_id);
                    self.agent.active_project_name = active_name;
                } else {
                    self.agent.active_project_id = None;
                    self.agent.active_project_name = "No Project".to_string();
                }

                let parsed = ragasukma_core::agent::executor::helpers::parse_project_folders(initial_root.as_deref());
                let folders: Vec<std::path::PathBuf> = parsed
                    .into_iter()
                    .map(std::path::PathBuf::from)
                    .filter(|p| p.exists())
                    .collect();

                if !folders.is_empty() {
                    self.agent.workspace_folders = folders;
                    self.agent.active_folder_idx = 0;
                    self.agent.scan_workspace_files();
                } else if !self.agent.workspace_folders.is_empty()
                    && let (Some(pid), Some(tx)) = (&self.agent.active_project_id, &self.bridge_cmd_tx)
                {
                    let current_paths: Vec<String> = self
                        .agent
                        .workspace_folders
                        .iter()
                        .map(|p| p.to_string_lossy().to_string())
                        .collect();
                    let _ = tx.send(CoreBridgeCommand::UpdateProjectFolders {
                        project_id: pid.clone(),
                        folders: current_paths,
                    });
                }
            }
            CoreBridgeEvent::ProjectCreated { project } => {
                let parsed = ragasukma_core::agent::executor::helpers::parse_project_folders(project.root_path.as_deref());
                let folders: Vec<std::path::PathBuf> = parsed
                    .into_iter()
                    .map(std::path::PathBuf::from)
                    .filter(|p| p.exists())
                    .collect();

                self.agent.active_project_id = Some(project.id.clone());
                self.agent.active_project_name = project.name.clone();
                if !folders.is_empty() {
                    self.agent.workspace_folders = folders;
                    self.agent.active_folder_idx = 0;
                    self.agent.scan_workspace_files();
                }

                let item = crate::agent_view::types::ProjectItem {
                    id: project.id.clone(),
                    name: project.name.clone(),
                    description: project.description.clone(),
                    root_path: project.root_path,
                };
                self.agent.projects.push(item);
            }
            CoreBridgeEvent::ProjectSwitched {
                project_id,
                project_name,
                root_path,
            } => {
                self.agent.active_project_id = Some(project_id);
                self.agent.active_project_name = project_name;

                let parsed = ragasukma_core::agent::executor::helpers::parse_project_folders(root_path.as_deref());
                let folders: Vec<std::path::PathBuf> = parsed
                    .into_iter()
                    .map(std::path::PathBuf::from)
                    .filter(|p| p.exists())
                    .collect();

                self.agent.workspace_folders = folders;
                self.agent.active_folder_idx = 0;
                self.agent.scan_workspace_files();
            }
            CoreBridgeEvent::ProjectDeleted { id } => {
                self.agent.projects.retain(|p| p.id != id);
            }
            CoreBridgeEvent::SessionsLoaded {
                sessions,
                active_id,
            } => {
                self.agent.sessions = sessions
                    .into_iter()
                    .map(|s| {
                        let is_running = s.status == "running";
                        crate::agent_view::types::TaskSessionItem {
                            id: s.id,
                            title: s.title,
                            status: s.status,
                            model: s.model,
                            total_tokens_input: s.total_tokens_input,
                            total_tokens_output: s.total_tokens_output,
                            updated_at: s.updated_at,
                            is_running,
                        }
                    })
                    .collect();
                self.agent.active_session_id = Some(active_id.clone());
                if let Some(pos) = self.agent.sessions.iter().position(|s| s.id == active_id) {
                    self.agent.selected_session_idx = pos;
                    if let Some(ref m) = self.agent.sessions[pos].model {
                        self.agent.active_model = m.clone();
                    }
                }
            }
            CoreBridgeEvent::Error(err) => {
                tracing::error!("Core bridge error received: {err}");
                self.error_modal = Some(err);
            }
            CoreBridgeEvent::HistoryLoaded { session_id, messages } => {
                self.agent.active_session_id = Some(session_id.clone());
                let sum_in: i64 = messages.iter().map(|m| m.token_usage_input.unwrap_or(0)).sum();
                let sum_out: i64 = messages.iter().map(|m| m.token_usage_output.unwrap_or(0)).sum();
                if let Some(pos) = self.agent.sessions.iter().position(|s| s.id == session_id) {
                    self.agent.selected_session_idx = pos;
                    if sum_in > 0 || sum_out > 0 {
                        self.agent.sessions[pos].total_tokens_input = sum_in;
                        self.agent.sessions[pos].total_tokens_output = sum_out;
                    }
                    if let Some(ref m) = self.agent.sessions[pos].model {
                        self.agent.active_model = m.clone();
                    }
                }
                self.agent.set_messages_from_persisted(messages);
                self.set_status(format!("Loaded conversation history for session: {}", session_id));
            }
            CoreBridgeEvent::SessionCreated { id, title } => {
                let session_item = crate::agent_view::types::TaskSessionItem {
                    id: id.clone(),
                    title: title.clone(),
                    status: "ready".to_string(),
                    model: Some(self.agent.active_model.clone()),
                    total_tokens_input: 0,
                    total_tokens_output: 0,
                    updated_at: 0,
                    is_running: false,
                };
                self.agent.sessions.insert(0, session_item);
                self.agent.selected_session_idx = 0;
                self.agent.active_session_id = Some(id);
                self.agent.messages.clear();
                self.set_status(format!("Created new task session: {}", title));
            }
            CoreBridgeEvent::SessionDeleted { id } => {
                self.agent.sessions.retain(|s| s.id != id);
                if self.agent.selected_session_idx >= self.agent.sessions.len() && !self.agent.sessions.is_empty() {
                    self.agent.selected_session_idx = self.agent.sessions.len() - 1;
                }
                self.set_status("Deleted task session".to_string());
            }
        }
    }

    pub fn request_selected_account_quota(&mut self) {
        self.request_selected_account_quota_internal(false);
    }

    pub fn force_refresh_selected_account_quota(&mut self) {
        self.request_selected_account_quota_internal(true);
    }

    fn request_selected_account_quota_internal(&mut self, force: bool) {
        let (provider, account_id) = match self.active_tab {
            1 => ("antigravity", self.antigravity.selected_account().map(|a| a.id.clone())),
            2 => ("codex", self.codex.selected_account().map(|a| a.id.clone())),
            3 => ("claude", self.claude.selected_account().map(|a| a.id.clone())),
            _ => return,
        };

        if let (Some(tx), Some(acc_id)) = (&self.bridge_cmd_tx, account_id) {
            let cache_key = (provider.to_string(), acc_id.clone());
            let now = std::time::Instant::now();

            if !force
                && self
                    .quota_last_fetched
                    .get(&cache_key)
                    .is_some_and(|last| now.duration_since(*last).as_secs() < 60)
            {
                // Data kuota masih segar dalam batas TTL (60 detik), abaikan fetch redundant
                return;
            }

            self.quota_last_fetched.insert(cache_key, now);
            let _ = tx.send(CoreBridgeCommand::RefreshAccountQuota {
                provider: provider.to_string(),
                account_id: acc_id,
            });
        }
    }

    pub fn request_all_active_quotas(&self) {
        if let Some(tx) = &self.bridge_cmd_tx {
            if let Some(acc) = self.antigravity.selected_account() {
                let _ = tx.send(CoreBridgeCommand::RefreshAccountQuota {
                    provider: "antigravity".to_string(),
                    account_id: acc.id.clone(),
                });
            }
            if let Some(acc) = self.codex.selected_account() {
                let _ = tx.send(CoreBridgeCommand::RefreshAccountQuota {
                    provider: "codex".to_string(),
                    account_id: acc.id.clone(),
                });
            }
            if let Some(acc) = self.claude.selected_account() {
                let _ = tx.send(CoreBridgeCommand::RefreshAccountQuota {
                    provider: "claude".to_string(),
                    account_id: acc.id.clone(),
                });
            }
        }
    }
}
