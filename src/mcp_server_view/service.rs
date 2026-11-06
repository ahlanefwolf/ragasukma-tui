use ragasukma_core::agent::models::McpServer;
use ragasukma_core::agent::repository::projects_and_sessions::{
    create_or_update_mcp_server, delete_mcp_server, get_project_mcp_server_ids, list_mcp_servers,
    set_project_mcp_servers,
};

/// Block on a future from a synchronous context.
fn block_on<F: std::future::Future>(fut: F) -> F::Output {
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        tokio::task::block_in_place(|| handle.block_on(fut))
    } else {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build tokio runtime")
            .block_on(fut)
    }
}

pub fn load_servers() -> Result<Vec<McpServer>, String> {
    block_on(list_mcp_servers()).map_err(|e| e.to_string())
}

pub fn save_server(id: &str, name: &str, config: &str, is_enabled: bool) -> Result<(), String> {
    block_on(create_or_update_mcp_server(id, name, config, is_enabled)).map_err(|e| e.to_string())
}

pub fn remove_server(id: &str) -> Result<(), String> {
    block_on(delete_mcp_server(id)).map_err(|e| e.to_string())
}

/// Load all MCP servers together with the ids that are active for a project.
/// When `project_id` is `None`, the active list is empty.
pub fn load_for_project(project_id: Option<&str>) -> Result<(Vec<McpServer>, Vec<String>), String> {
    let servers = block_on(list_mcp_servers()).map_err(|e| e.to_string())?;
    let active = match project_id {
        Some(pid) if !pid.is_empty() => {
            block_on(get_project_mcp_server_ids(pid)).map_err(|e| e.to_string())?
        }
        _ => Vec::new(),
    };
    Ok((servers, active))
}

/// Persist the set of active MCP server ids for a project.
pub fn set_project_active_servers(project_id: &str, ids: &[String]) -> Result<(), String> {
    block_on(set_project_mcp_servers(project_id, ids)).map_err(|e| e.to_string())
}

/// Generate a URL/ID-safe slug from a display name.
pub fn slugify(name: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for c in name.trim().to_ascii_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
            prev_dash = false;
        } else if !prev_dash && !out.is_empty() {
            out.push('-');
            prev_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "mcp-server".to_string()
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("Filesystem Server"), "filesystem-server");
        assert_eq!(slugify("  GitHub!!  "), "github");
        assert_eq!(slugify("Snake_Case.123"), "snake-case-123");
        assert_eq!(slugify("!!!"), "mcp-server");
        assert_eq!(slugify(""), "mcp-server");
    }
}
