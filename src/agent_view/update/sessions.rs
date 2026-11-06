use crate::agent_view::states::AgentWorkspaceState;
use crate::app::types::Action;
use crate::core_bridge::CoreBridgeCommand;
use tokio::sync::mpsc::UnboundedSender;

/// Task/session lifecycle: new/delete/switch/next/prev task, cancel/retry turn, sidebar toggle & prompt focus.
pub(crate) fn handle_action(
    agent: &mut AgentWorkspaceState,
    action: Action,
    bridge_cmd_tx: Option<&UnboundedSender<CoreBridgeCommand>>,
) -> Option<Option<String>> {
    let status = match action {
        Action::AgentNewTask => {
            let default_title = format!("Task #{}", agent.sessions.len() + 1);
            let active_model = agent.active_model.clone();
            if let Some(tx) = bridge_cmd_tx {
                let _ = tx.send(CoreBridgeCommand::CreateSession {
                    title: default_title,
                    model: Some(active_model),
                });
            } else {
                agent.create_new_task(None);
            }
            Some("Creating new agent task session...".to_string())
        }
        Action::AgentDeleteSelectedTask => {
            if let Some(s) = agent.sessions.get(agent.selected_session_idx) {
                let sid = s.id.clone();
                if let Some(tx) = bridge_cmd_tx {
                    let _ = tx.send(CoreBridgeCommand::DeleteSession { session_id: sid });
                } else {
                    agent.delete_selected_task();
                }
                Some("Deleting task session...".to_string())
            } else {
                None
            }
        }
        Action::AgentSwitchTask => {
            if let Some(s) = agent.sessions.get(agent.selected_session_idx) {
                let sid = s.id.clone();
                if let Some(ref m) = s.model {
                    agent.active_model = m.clone();
                }
                agent.active_session_id = Some(sid.clone());
                agent.is_running = s.is_running;
                if let Some(tx) = bridge_cmd_tx {
                    let _ = tx.send(CoreBridgeCommand::SwitchSession { session_id: sid });
                }
                Some(format!("Switched to task: {}", s.title))
            } else {
                None
            }
        }
        Action::AgentNextTask => {
            agent.next_session();
            if let Some(s) = agent.sessions.get(agent.selected_session_idx) {
                let sid = s.id.clone();
                if let Some(ref m) = s.model {
                    agent.active_model = m.clone();
                }
                agent.active_session_id = Some(sid.clone());
                agent.is_running = s.is_running;
                if let Some(tx) = bridge_cmd_tx {
                    let _ = tx.send(CoreBridgeCommand::SwitchSession { session_id: sid });
                }
                Some(format!("Selected task: {}", s.title))
            } else {
                None
            }
        }
        Action::AgentPrevTask => {
            agent.prev_session();
            if let Some(s) = agent.sessions.get(agent.selected_session_idx) {
                let sid = s.id.clone();
                if let Some(ref m) = s.model {
                    agent.active_model = m.clone();
                }
                agent.active_session_id = Some(sid.clone());
                agent.is_running = s.is_running;
                if let Some(tx) = bridge_cmd_tx {
                    let _ = tx.send(CoreBridgeCommand::SwitchSession { session_id: sid });
                }
                Some(format!("Selected task: {}", s.title))
            } else {
                None
            }
        }
        Action::AgentCancelTurn => {
            if let Some(tx) = bridge_cmd_tx {
                let _ = tx.send(CoreBridgeCommand::CancelTurn {
                    session_id: agent.active_session_id.clone(),
                });
            }
            agent.cancel_turn();
            Some("Cancelling turn...".to_string())
        }
        Action::AgentRetryTurn => {
            if let Some(tx) = bridge_cmd_tx {
                let _ = tx.send(CoreBridgeCommand::RetryTurn);
            }
            agent.start_turn();
            Some("Retrying turn...".to_string())
        }
        Action::AgentToggleSidebar => {
            agent.toggle_sidebar();
            let status_msg = if agent.is_sidebar_visible {
                "Sidebar opened"
            } else {
                "Sidebar hidden"
            };
            Some(status_msg.to_string())
        }
        _ => return None,
    };

    Some(status)
}
