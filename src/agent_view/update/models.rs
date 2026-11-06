use crate::agent_view::states::AgentWorkspaceState;
use crate::app::types::Action;
use crate::core_bridge::CoreBridgeCommand;
use tokio::sync::mpsc::UnboundedSender;

/// Model selection: cycle, modal open/close, modal char/backspace, apply, refresh.
pub(crate) fn handle_action(
    agent: &mut AgentWorkspaceState,
    action: Action,
    bridge_cmd_tx: Option<&UnboundedSender<CoreBridgeCommand>>,
) -> Option<Option<String>> {
    let status = match action {
        Action::AgentFocusPrompt => {
            agent.focus_input();
            Some("Prompt focused".to_string())
        }
        Action::AgentCycleModel => {
            agent.cycle_model();
            let model_name = agent.active_model.clone();
            if let Some(s) = agent.sessions.get_mut(agent.selected_session_idx) {
                s.model = Some(model_name.clone());
            }
            if let Some(ref sid) = agent.active_session_id
                && let Some(tx) = bridge_cmd_tx
            {
                let _ = tx.send(CoreBridgeCommand::UpdateSessionModel {
                    session_id: sid.clone(),
                    model: model_name.clone(),
                });
            }
            Some(format!("Selected model: {}", model_name))
        }
        Action::AgentOpenModelModal => {
            agent.open_model_modal();
            Some("Select Model: Type to filter, [↑/↓] or [Tab] to navigate, [Enter] to select, [Esc] to cancel".to_string())
        }
        Action::AgentCloseModelModal => {
            agent.close_model_modal();
            Some("Model selector closed".to_string())
        }
        Action::AgentSelectNextModalModel => {
            agent.select_next_modal_model();
            None
        }
        Action::AgentSelectPrevModalModel => {
            agent.select_prev_modal_model();
            None
        }
        Action::AgentModelModalChar(c) => {
            agent.model_modal_push_char(c);
            None
        }
        Action::AgentModelModalBackspace => {
            agent.model_modal_backspace();
            None
        }
        Action::AgentApplyModalModel => {
            agent.apply_modal_model();
            let model_name = agent.active_model.clone();
            if let Some(s) = agent.sessions.get_mut(agent.selected_session_idx) {
                s.model = Some(model_name.clone());
            }
            if let Some(ref sid) = agent.active_session_id
                && let Some(tx) = bridge_cmd_tx
            {
                let _ = tx.send(CoreBridgeCommand::UpdateSessionModel {
                    session_id: sid.clone(),
                    model: model_name.clone(),
                });
            }
            Some(format!("Active model set to: {}", model_name))
        }
        Action::AgentRefreshModels => {
            let count = agent.refresh_available_models();
            Some(format!(
                "Refreshed models from core providers ({} available)",
                count
            ))
        }
        _ => return None,
    };

    Some(status)
}
