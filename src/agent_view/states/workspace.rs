use super::helpers::collect_recursive_tag_items;
use super::*;

fn block_on<F>(future: F) -> Result<F::Output, String>
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        match handle.runtime_flavor() {
            tokio::runtime::RuntimeFlavor::MultiThread => {
                Ok(tokio::task::block_in_place(|| handle.block_on(future)))
            }
            _ => std::thread::scope(|s| {
                s.spawn(|| {
                    tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .map_err(|e| e.to_string())
                        .map(|rt| rt.block_on(future))
                })
                .join()
                .map_err(|_| "thread joined with error".to_string())?
            }),
        }
    } else {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| e.to_string())
            .map(|rt| rt.block_on(future))
    }
}

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

    /// Automatically reload and synchronize MCP servers and active IDs for the current active session.
    pub fn reload_mcp_servers(&mut self) -> Result<usize, String> {
        let session_id = self.active_session_id.clone();
        let (servers, active_ids) = block_on(async move {
            let servers =
                ragasukma_core::repository::agent::projects_and_sessions::list_mcp_servers()
                    .await
                    .map_err(|e| e.to_string())?;
            let active_ids = if let Some(sid) = session_id {
                ragasukma_core::repository::agent::projects_and_sessions::get_session_active_mcp_servers(&sid)
                    .await
                    .map(|list| list.into_iter().map(|(srv, _)| srv.id).collect())
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
            Ok::<_, String>((servers, active_ids))
        })??;

        let count = servers.len();
        self.mcp_servers = servers;
        self.mcp_active_ids = active_ids;
        if self.selected_mcp_idx >= self.mcp_servers.len() {
            self.selected_mcp_idx = self.mcp_servers.len().saturating_sub(1);
        }
        Ok(count)
    }
}
