use crate::agent_view::states::AgentWorkspaceState;
use crate::agent_view::types::AgentFocusArea;
use crate::app::types::Action;
use crate::core_bridge::CoreBridgeCommand;
use tokio::sync::mpsc::UnboundedSender;

/// Folders & project management: add/remove/browse folders, scan, project modal create/delete.
pub(crate) fn handle_action(
    agent: &mut AgentWorkspaceState,
    action: Action,
    bridge_cmd_tx: Option<&UnboundedSender<CoreBridgeCommand>>,
) -> Option<Option<String>> {
    let status = match action {
        Action::AgentOpenAddFolderModal => {
            agent.open_add_folder_modal();
            Some("Add folder to workspace (Enter path or press Tab to browse)".to_string())
        }
        Action::AgentCloseAddFolderModal => {
            agent.close_add_folder_modal();
            Some("Cancelled adding folder".to_string())
        }
        Action::AgentAddFolderChar(c) => {
            agent.add_folder_modal.input_path.push(c);
            None
        }
        Action::AgentAddFolderBackspace => {
            agent.add_folder_modal.input_path.pop();
            None
        }
        Action::AgentSubmitAddFolder => match agent.submit_add_folder() {
            Ok(()) => {
                let count = agent.workspace_folders.len();
                if let Some(tx) = bridge_cmd_tx {
                    let folders: Vec<String> = agent
                        .workspace_folders
                        .iter()
                        .map(|f| f.to_string_lossy().to_string())
                        .collect();

                    if let Some(active_pid) = &agent.active_project_id {
                        let _ = tx.send(CoreBridgeCommand::UpdateProjectFolders {
                            project_id: active_pid.clone(),
                            folders,
                        });
                    } else {
                        let first_folder_name = agent
                            .workspace_folders
                            .first()
                            .and_then(|p| p.file_name())
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| "Workspace".to_string());

                        let root_path_json = serde_json::to_string(&folders).ok();
                        let _ = tx.send(CoreBridgeCommand::CreateProject {
                            name: first_folder_name,
                            description: Some("Workspace project".to_string()),
                            root_path: root_path_json,
                        });
                    }
                }
                Some(format!(
                    "Folder attached successfully (Total: {} folders)",
                    count
                ))
            }
            Err(err) => Some(format!("Failed to attach folder: {}", err)),
        },
        Action::AgentRemoveActiveFolder => {
            let idx = agent.active_folder_idx;
            match agent.remove_workspace_folder(idx) {
                Ok(()) => {
                    if let (Some(active_pid), Some(tx)) = (&agent.active_project_id, bridge_cmd_tx)
                    {
                        let folders: Vec<String> = agent
                            .workspace_folders
                            .iter()
                            .map(|f| f.to_string_lossy().to_string())
                            .collect();
                        let _ = tx.send(CoreBridgeCommand::UpdateProjectFolders {
                            project_id: active_pid.clone(),
                            folders,
                        });
                    }
                    Some("Folder detached from workspace".to_string())
                }
                Err(err) => Some(format!("Failed to detach folder: {}", err)),
            }
        }
        Action::AgentNextFolder => {
            agent.select_next_folder();
            agent
                .workspace_folders
                .get(agent.active_folder_idx)
                .map(|f| format!("Active folder: {}", f.display()))
        }
        Action::AgentPrevFolder => {
            agent.select_prev_folder();
            agent
                .workspace_folders
                .get(agent.active_folder_idx)
                .map(|f| format!("Active folder: {}", f.display()))
        }
        Action::AgentBrowseFolderDialog => {
            let initial = if !agent.add_folder_modal.input_path.trim().is_empty() {
                Some(agent.add_folder_modal.input_path.as_str())
            } else {
                agent
                    .workspace_folders
                    .get(agent.active_folder_idx)
                    .and_then(|p| p.to_str())
            };

            match ragasukma_core::agent::tools::pick_folder_dialog(initial) {
                Ok(Some(selected_path)) => {
                    let result = if agent.add_folder_modal.is_open {
                        agent.add_folder_modal.input_path = selected_path;
                        agent.submit_add_folder()
                    } else {
                        agent.add_workspace_folder(std::path::PathBuf::from(selected_path))
                    };

                    match result {
                        Ok(()) => {
                            if let Some(tx) = bridge_cmd_tx {
                                let folders: Vec<String> = agent
                                    .workspace_folders
                                    .iter()
                                    .map(|f| f.to_string_lossy().to_string())
                                    .collect();

                                if let Some(active_pid) = &agent.active_project_id {
                                    let _ = tx.send(CoreBridgeCommand::UpdateProjectFolders {
                                        project_id: active_pid.clone(),
                                        folders,
                                    });
                                } else {
                                    let first_folder_name = agent
                                        .workspace_folders
                                        .first()
                                        .and_then(|p| p.file_name())
                                        .map(|n| n.to_string_lossy().to_string())
                                        .unwrap_or_else(|| "Workspace".to_string());

                                    let root_path_json = serde_json::to_string(&folders).ok();
                                    let _ = tx.send(CoreBridgeCommand::CreateProject {
                                        name: first_folder_name,
                                        description: Some("Workspace project".to_string()),
                                        root_path: root_path_json,
                                    });
                                }
                            }
                            Some("Folder attached from file manager dialog".to_string())
                        }
                        Err(err) => Some(format!("Failed to attach folder: {}", err)),
                    }
                }
                Ok(None) => Some("Folder selection cancelled".to_string()),
                Err(err) => Some(format!("File manager dialog error: {}", err)),
            }
        }
        Action::AgentOpenFolderInFileManager => {
            let target_path = if agent.focus_area == AgentFocusArea::Files {
                if let Some(file_node) = agent.workspace_files.get(agent.selected_file_idx) {
                    let p = std::path::Path::new(&file_node.path);
                    if p.exists() {
                        if p.is_dir() {
                            Some(p.to_path_buf())
                        } else {
                            p.parent()
                                .map(|parent| parent.to_path_buf())
                                .or_else(|| Some(p.to_path_buf()))
                        }
                    } else {
                        agent
                            .workspace_folders
                            .get(agent.active_folder_idx)
                            .cloned()
                    }
                } else {
                    agent
                        .workspace_folders
                        .get(agent.active_folder_idx)
                        .cloned()
                }
            } else {
                agent
                    .workspace_folders
                    .get(agent.active_folder_idx)
                    .cloned()
            };

            if let Some(folder) = target_path {
                match ragasukma_core::agent::tools::open_path_in_file_manager(
                    &folder.to_string_lossy(),
                ) {
                    Ok(()) => Some(format!("Opened file manager for {}", folder.display())),
                    Err(err) => Some(format!("Failed to open file manager: {}", err)),
                }
            } else {
                None
            }
        }
        Action::AgentOpenProjectModal => {
            agent.project_modal.is_open = true;
            Some(
                "Project switcher opened. Select a project or press 'n' to create new.".to_string(),
            )
        }
        Action::AgentCloseProjectModal => {
            agent.project_modal.is_open = false;
            None
        }
        Action::AgentSelectNextProject => {
            if !agent.projects.is_empty() {
                agent.project_modal.selected_idx =
                    (agent.project_modal.selected_idx + 1) % agent.projects.len();
            }
            None
        }
        Action::AgentSelectPrevProject => {
            if !agent.projects.is_empty() {
                if agent.project_modal.selected_idx == 0 {
                    agent.project_modal.selected_idx = agent.projects.len() - 1;
                } else {
                    agent.project_modal.selected_idx -= 1;
                }
            }
            None
        }
        Action::AgentSwitchProject => {
            if let Some(target) = agent.projects.get(agent.project_modal.selected_idx) {
                let target_id = target.id.clone();
                let target_name = target.name.clone();
                agent.project_modal.is_open = false;
                agent.is_running = false;
                agent.messages.clear();
                agent.active_session_id = None;
                if let Some(tx) = bridge_cmd_tx {
                    let _ = tx.send(CoreBridgeCommand::SwitchProject {
                        project_id: target_id,
                    });
                    Some(format!("Switched to project: {}", target_name))
                } else {
                    None
                }
            } else {
                None
            }
        }
        Action::AgentDeleteProject => {
            if let Some(target) = agent.projects.get(agent.project_modal.selected_idx) {
                let target_id = target.id.clone();
                let target_name = target.name.clone();

                if let Some(tx) = bridge_cmd_tx {
                    let _ = tx.send(CoreBridgeCommand::DeleteProject {
                        project_id: target_id.clone(),
                    });
                }

                // If active project is deleted, reset workspace state
                if agent.active_project_id.as_deref() == Some(&target_id) {
                    agent.active_project_id = None;
                    agent.active_project_name.clear();
                    agent.workspace_folders.clear();
                    agent.active_folder_idx = 0;
                    agent.scan_workspace_files();
                    agent.sessions.clear();
                    agent.active_session_id = None;
                    agent.clear_chat();
                }

                agent.projects.remove(agent.project_modal.selected_idx);
                if agent.project_modal.selected_idx >= agent.projects.len()
                    && !agent.projects.is_empty()
                {
                    agent.project_modal.selected_idx = agent.projects.len() - 1;
                }

                Some(format!("Project '{}' deleted successfully", target_name))
            } else {
                None
            }
        }
        Action::AgentOpenCreateProjectModal => {
            agent.project_modal.is_open = false;
            agent.create_project_modal.is_open = true;
            agent.create_project_modal.name_input.clear();
            Some("Create New Project".to_string())
        }
        Action::AgentCloseCreateProjectModal => {
            agent.create_project_modal.is_open = false;
            None
        }
        Action::AgentCreateProjectChar(c) => {
            agent.create_project_modal.name_input.push(c);
            None
        }
        Action::AgentCreateProjectBackspace => {
            agent.create_project_modal.name_input.pop();
            None
        }
        Action::AgentSubmitCreateProject => {
            let name = agent.create_project_modal.name_input.trim().to_string();
            let res = if !name.is_empty() {
                if let Some(tx) = bridge_cmd_tx {
                    let _ = tx.send(CoreBridgeCommand::CreateProject {
                        name: name.clone(),
                        description: None,
                        root_path: None,
                    });
                }
                Some(format!("Project '{}' created", name))
            } else {
                None
            };
            agent.create_project_modal.is_open = false;
            res
        }
        _ => return None,
    };

    Some(status)
}
