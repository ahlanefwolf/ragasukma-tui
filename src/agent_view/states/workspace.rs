use super::helpers::collect_recursive_tag_items;
use super::*;

impl AgentWorkspaceState {
    pub fn refresh_workspace_tag_items(&mut self) {
        if self.workspace_folders.is_empty() {
            self.all_workspace_tag_items.clear();
            return;
        }

        let is_multi_root = self.workspace_folders.len() > 1;
        let mut items = Vec::new();

        for folder in &self.workspace_folders {
            let root_name = folder
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| folder.to_string_lossy().to_string());

            if is_multi_root {
                items.push(TagItem {
                    path: format!("{}/", root_name),
                    name: root_name.clone(),
                    is_dir: true,
                });
            }

            collect_recursive_tag_items(folder, folder, 1, is_multi_root, &root_name, &mut items);
        }

        self.all_workspace_tag_items = items;
    }

    /// Automatically reload and synchronize MCP servers and active IDs for the current project.
    pub fn reload_mcp_servers(&mut self) -> Result<usize, String> {
        let (servers, active_ids) =
            crate::mcp_server_view::service::load_for_project(self.active_project_id.as_deref())?;
        let count = servers.len();
        self.mcp_servers = servers;
        self.mcp_active_ids = active_ids;
        if self.selected_mcp_idx >= self.mcp_servers.len() {
            self.selected_mcp_idx = self.mcp_servers.len().saturating_sub(1);
        }
        Ok(count)
    }
}
