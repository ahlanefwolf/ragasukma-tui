use crate::agent_view::states::AgentWorkspaceState;
use crate::app::types::Action;

/// MCP server management: navigate, toggle, refresh.
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
        Action::AgentToggleMcpServer => 'toggle: {
            let Some(project_id) = agent.active_project_id.clone() else {
                break 'toggle Some("No active project to toggle MCP servers".to_string());
            };
            if agent.mcp_servers.is_empty() {
                break 'toggle Some("No MCP servers available".to_string());
            }
            let idx = agent.selected_mcp_idx.min(agent.mcp_servers.len() - 1);
            let server_id = agent.mcp_servers[idx].id.clone();
            let server_name = agent.mcp_servers[idx].name.clone();
            let mut active = agent.mcp_active_ids.clone();
            let now_active = if let Some(pos) = active.iter().position(|id| id == &server_id) {
                active.remove(pos);
                false
            } else {
                active.push(server_id);
                true
            };
            match crate::mcp_server_view::service::set_project_active_servers(&project_id, &active)
            {
                Ok(()) => {
                    agent.mcp_active_ids = active;
                    Some(format!(
                        "MCP server '{}' {}",
                        server_name,
                        if now_active { "enabled" } else { "disabled" }
                    ))
                }
                Err(e) => Some(format!("Failed to update MCP server: {e}")),
            }
        }
        Action::AgentRefreshMcpServers => {
            let Some(project_id) = agent.active_project_id.clone() else {
                return Some(Some("No active project to load MCP servers".to_string()));
            };
            match crate::mcp_server_view::service::load_for_project(Some(&project_id)) {
                Ok((servers, active_ids)) => {
                    let count = servers.len();
                    agent.mcp_servers = servers;
                    agent.mcp_active_ids = active_ids;
                    if agent.selected_mcp_idx >= agent.mcp_servers.len() {
                        agent.selected_mcp_idx = agent.mcp_servers.len().saturating_sub(1);
                    }
                    Some(format!("Loaded {} MCP server(s) for project", count))
                }
                Err(e) => Some(format!("Failed to load MCP servers: {e}")),
            }
        }
        _ => return None,
    };

    Some(status)
}
