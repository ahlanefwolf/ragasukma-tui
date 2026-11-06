use super::helpers::collect_visible_children;
use super::*;

impl AgentWorkspaceState {
    pub fn cycle_tool_filter(&mut self) {
        self.tool_filter = match self.tool_filter {
            ToolFilter::All => ToolFilter::RunningOnly,
            ToolFilter::RunningOnly => ToolFilter::FailedOnly,
            ToolFilter::FailedOnly => ToolFilter::All,
        };
    }

    pub fn approve_pending(&mut self) -> Option<ApprovalRequest> {
        self.pending_approval.take()
    }

    pub fn reject_pending(&mut self) -> Option<ApprovalRequest> {
        self.pending_approval.take()
    }

    pub fn select_file_next(&mut self) {
        if !self.workspace_files.is_empty() {
            self.selected_file_idx = (self.selected_file_idx + 1) % self.workspace_files.len();
        }
    }

    pub fn select_file_prev(&mut self) {
        if !self.workspace_files.is_empty() {
            if self.selected_file_idx == 0 {
                self.selected_file_idx = self.workspace_files.len() - 1;
            } else {
                self.selected_file_idx -= 1;
            }
        }
    }

    pub fn toggle_file_tree_expand(&mut self) {
        if let Some(file) = self
            .workspace_files
            .get(self.selected_file_idx)
            .filter(|f| f.is_dir)
        {
            let path = file.path.clone();
            if self.expanded_dirs.contains(&path) {
                self.expanded_dirs.remove(&path);
            } else {
                self.expanded_dirs.insert(path);
            }
            self.scan_workspace_files();
        }
    }

    pub fn insert_selected_file_to_input(&mut self) {
        if let Some(file) = self.workspace_files.get(self.selected_file_idx) {
            let insertion = format!("@{} ", file.path);
            let prefix = &self.input_buffer[..self.cursor_pos];
            let suffix = &self.input_buffer[self.cursor_pos..];
            let new_cursor = prefix.len() + insertion.len();
            self.input_buffer = format!("{}{}{}", prefix, insertion, suffix);
            self.cursor_pos = new_cursor;
        }
    }

    pub fn get_workspace_folders_list(&self) -> Vec<WorkspaceFolderItem> {
        self.workspace_folders
            .iter()
            .map(|p| {
                let name = p
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| p.to_string_lossy().to_string());
                WorkspaceFolderItem {
                    path: p.clone(),
                    name,
                    is_valid: p.is_dir(),
                }
            })
            .collect()
    }

    pub fn add_workspace_folder(&mut self, path: PathBuf) -> Result<(), String> {
        let expanded = match path.strip_prefix("~") {
            Ok(rest) => {
                let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"));
                if let Some(home_path) = home {
                    PathBuf::from(home_path).join(rest)
                } else {
                    path.clone()
                }
            }
            Err(_) => path.clone(),
        };

        if !expanded.is_dir() {
            return Err(format!("Directory not found: {}", expanded.display()));
        }

        let canonical = match expanded.canonicalize() {
            Ok(c) => c,
            Err(_) => expanded,
        };

        if self.workspace_folders.iter().any(|f| {
            if let Ok(c) = f.canonicalize() {
                c == canonical
            } else {
                f == &canonical
            }
        }) {
            return Err("Folder is already in workspace".to_string());
        }

        let folder_str = canonical.to_string_lossy().to_string();
        self.expanded_dirs.insert(folder_str);
        self.workspace_folders.push(canonical);
        self.active_folder_idx = self.workspace_folders.len() - 1;
        self.scan_workspace_files();
        Ok(())
    }

    pub fn remove_workspace_folder(&mut self, idx: usize) -> Result<(), String> {
        if self.workspace_folders.len() <= 1 {
            return Err("Cannot remove the only workspace folder".to_string());
        }
        if idx < self.workspace_folders.len() {
            let removed = self.workspace_folders.remove(idx);
            self.expanded_dirs
                .remove(&removed.to_string_lossy().to_string());
            if self.active_folder_idx >= self.workspace_folders.len() {
                self.active_folder_idx = self.workspace_folders.len().saturating_sub(1);
            }
            self.scan_workspace_files();
            Ok(())
        } else {
            Err("Invalid folder index".to_string())
        }
    }

    pub fn select_next_folder(&mut self) {
        if !self.workspace_folders.is_empty() {
            self.active_folder_idx = (self.active_folder_idx + 1) % self.workspace_folders.len();
        }
    }

    pub fn select_prev_folder(&mut self) {
        if !self.workspace_folders.is_empty() {
            if self.active_folder_idx == 0 {
                self.active_folder_idx = self.workspace_folders.len() - 1;
            } else {
                self.active_folder_idx -= 1;
            }
        }
    }

    pub fn open_add_folder_modal(&mut self) {
        self.add_folder_modal.is_open = true;
        self.add_folder_modal.input_path.clear();
        self.add_folder_modal.validation_message = None;
    }

    pub fn close_add_folder_modal(&mut self) {
        self.add_folder_modal.is_open = false;
        self.add_folder_modal.input_path.clear();
        self.add_folder_modal.validation_message = None;
    }

    pub fn submit_add_folder(&mut self) -> Result<(), String> {
        let input = self.add_folder_modal.input_path.trim().to_string();
        if input.is_empty() {
            self.add_folder_modal.validation_message =
                Some((false, "Path cannot be empty".to_string()));
            return Err("Path cannot be empty".to_string());
        }
        let path = PathBuf::from(&input);
        match self.add_workspace_folder(path) {
            Ok(()) => {
                self.close_add_folder_modal();
                Ok(())
            }
            Err(err) => {
                self.add_folder_modal.validation_message = Some((false, err.clone()));
                Err(err)
            }
        }
    }

    pub fn scan_workspace_files(&mut self) {
        if self.workspace_folders.is_empty() {
            self.workspace_files.clear();
            self.selected_file_idx = 0;
            return;
        }

        let mut nodes = Vec::new();
        for folder in &self.workspace_folders {
            let folder_str = folder.to_string_lossy().to_string();
            let name = folder
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| folder_str.clone());
            let is_expanded = self.expanded_dirs.contains(&folder_str);
            nodes.push(WorkspaceFileNode {
                path: folder_str.clone(),
                name,
                is_dir: true,
                depth: 0,
                is_expanded,
                parent_path: None,
            });
            if is_expanded {
                collect_visible_children(
                    folder,
                    1,
                    &self.expanded_dirs,
                    &mut nodes,
                    Some(folder_str),
                );
            }
        }

        self.workspace_files = nodes;
        if self.selected_file_idx >= self.workspace_files.len() && !self.workspace_files.is_empty()
        {
            self.selected_file_idx = self.workspace_files.len() - 1;
        }
        self.refresh_workspace_tag_items();
    }
}
