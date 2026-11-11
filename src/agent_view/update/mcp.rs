use crate::agent_view::states::AgentWorkspaceState;
use crate::app::types::Action;

/// MCP server management: navigate, refresh. (Auto on/off per active session)
pub(crate) fn handle_action(
    agent: &mut AgentWorkspaceState,
    action: Action,
) -> Option<Option<String>> {
    let status = match action {
        Action::AgentNextMcpServer => {
            if !agent.mcp_servers.is_empty() {
                agent.selected_mcp_idx =
                    (agent.selected_mcp_idx + 1).min(agent.mcp_servers.len() - 1);
            }
            None
        }
        Action::AgentPrevMcpServer => {
            agent.selected_mcp_idx = agent.selected_mcp_idx.saturating_sub(1);
            None
        }
        Action::AgentToggleMcpServer => {
            Some("MCP servers are automatically enabled/disabled by the active session".to_string())
        }
        Action::AgentRefreshMcpServers => match agent.reload_mcp_servers() {
            Ok(count) => Some(format!("Loaded {} MCP server(s) for active session", count)),
            Err(e) => Some(format!("Failed to load MCP servers: {e}")),
        },
        _ => return None,
    };

    Some(status)
}
