//! Agent-view action dispatch, split per domain:
//! - `input`     – text editing, cursor, paste/clear
//! - `chat`      – scroll, reasoning expand, tool cards, submit prompt
//! - `sessions`  – task/session lifecycle & focus/panel switching
//! - `models`    – model modal & selection
//! - `mcp`       – MCP server management
//! - `workspace` – folders, project modal & management

use crate::agent_view::states::AgentWorkspaceState;
use crate::app::types::Action;
use crate::core_bridge::CoreBridgeCommand;
use tokio::sync::mpsc::UnboundedSender;

mod chat;
mod input;
mod mcp;
mod models;
mod sessions;
mod workspace;

/// Tries each per-domain handler in order; the first one that recognizes
/// the action wins. Handlers return `None` when the action is not theirs,
/// and `Some(status)` when handled.
pub fn handle_agent_action(
    agent: &mut AgentWorkspaceState,
    action: Action,
    bridge_cmd_tx: Option<&UnboundedSender<CoreBridgeCommand>>,
) -> Option<Option<String>> {
    input::handle_action(agent, action.clone(), bridge_cmd_tx)
        .or_else(|| chat::handle_action(agent, action.clone(), bridge_cmd_tx))
        .or_else(|| sessions::handle_action(agent, action.clone(), bridge_cmd_tx))
        .or_else(|| models::handle_action(agent, action.clone(), bridge_cmd_tx))
        .or_else(|| mcp::handle_action(agent, action.clone()))
        .or_else(|| workspace::handle_action(agent, action, bridge_cmd_tx))
}
