use crate::agent_view::types::{ToolCallInfo, ToolExecutionStatus};
use crate::app::state::App;
use crate::core_bridge::{CoreBridgeCommand, CoreBridgeEvent};

impl App {
    pub fn handle_bridge_event(&mut self, ev: CoreBridgeEvent) {
        match ev {
            CoreBridgeEvent::Connected { session_id, .. } => {
                self.set_status(format!("Agent siap (session {})", session_id));
            }
            CoreBridgeEvent::StatusChange(msg) => {
                self.set_status(msg);
            }
            CoreBridgeEvent::Delta(delta) => {
                self.agent.append_chunk(&delta);
            }
            CoreBridgeEvent::ThoughtDelta(thought) => {
                self.agent.append_thought(&thought);
            }
            CoreBridgeEvent::StepInfo { action, details, .. } => {
                let det_str = details.unwrap_or_default();
                tracing::debug!("Agent step: action='{action}', details='{det_str}'");
                self.set_status(format!("[Step] {} - {}", action, det_str));
            }
            CoreBridgeEvent::ToolCall { name, args } => {
                tracing::info!("Tool call received: name='{name}', args='{args}'");
                if let Some(last) = self.agent.messages.last_mut() {
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
            CoreBridgeEvent::TurnCompleted(msg) => {
                tracing::info!("Turn completed: {msg}");
                self.agent.finish_turn();
                self.set_status(format!("Turn completed: {}", msg));
            }
            CoreBridgeEvent::TurnCancelled { session_id } => {
                tracing::warn!("Turn cancelled for session: {session_id}");
                self.agent.cancel_turn();
                self.set_status(format!("Turn cancelled for session: {}", session_id));
            }
            CoreBridgeEvent::TurnFailed { error, session_id } => {
                tracing::error!("Turn failed for session {session_id}: {error}");
                self.agent.fail_turn(error.clone());
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
            CoreBridgeEvent::Error(err) => {
                tracing::error!("Core bridge error received: {err}");
                self.error_modal = Some(err);
            }
            _ => {}
        }
    }

    pub fn request_selected_account_quota(&self) {
        let (provider, account_id) = match self.active_tab {
            1 => ("antigravity", self.antigravity.selected_account().map(|a| a.id.clone())),
            2 => ("codex", self.codex.selected_account().map(|a| a.id.clone())),
            3 => ("claude", self.claude.selected_account().map(|a| a.id.clone())),
            _ => return,
        };

        if let (Some(tx), Some(acc_id)) = (&self.bridge_cmd_tx, account_id) {
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
