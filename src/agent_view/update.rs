use crate::agent_view::state::AgentWorkspaceState;
use crate::agent_view::types::{AgentFocusArea, AgentSidebarTab, ChatMessage, MessageRole};
use crate::app::state::current_time_str;
use crate::app::types::Action;
use crate::core_bridge::CoreBridgeCommand;
use tokio::sync::mpsc::UnboundedSender;

pub fn handle_agent_action(
    agent: &mut AgentWorkspaceState,
    action: Action,
    bridge_cmd_tx: Option<&UnboundedSender<CoreBridgeCommand>>,
) -> Option<Option<String>> {
    let status = match action {
        Action::AgentInsertChar(c) => {
            agent.insert_char(c);
            agent.check_and_update_tag_query();
            None
        }
        Action::AgentBackspace => {
            agent.backspace();
            agent.check_and_update_tag_query();
            None
        }
        Action::AgentClearInput => {
            agent.clear_input();
            agent.close_tag_query();
            None
        }
        Action::AgentClearChat => {
            agent.clear_chat();
            None
        }
        Action::AgentSubmitPrompt => {
            agent.close_tag_query();
            if let Some(prompt) = agent.submit_prompt() {
                if let Some(tx) = bridge_cmd_tx {
                    let _ = tx.send(CoreBridgeCommand::ExecuteTurn {
                        prompt: prompt.clone(),
                        model_override: None,
                    });
                }
                agent.start_turn();
                agent.push_message(ChatMessage {
                    role: MessageRole::User,
                    sender_name: "You".to_string(),
                    content: prompt,
                    timestamp: current_time_str(),
                    tool_calls: Vec::new(),
                    reasoning: None,
                });
            }
            None
        }
        Action::AgentFocusInput => {
            agent.focus_input();
            Some(format!("Focus: {:?}", agent.focus_area))
        }
        Action::AgentUnfocusInput => {
            agent.unfocus_input();
            Some(format!("Focus: {:?}", agent.focus_area))
        }
        Action::AgentFocusFiles => {
            agent.focus_area = AgentFocusArea::Files;
            Some("Focus: Files".to_string())
        }
        Action::AgentFocusChat => {
            agent.focus_area = AgentFocusArea::Chat;
            Some("Focus: Chat".to_string())
        }
        Action::AgentFocusNextPanel => {
            agent.cycle_focus_next();
            Some(format!("Focus: {:?}", agent.focus_area))
        }
        Action::AgentFocusPrevPanel => {
            agent.cycle_focus_prev();
            Some(format!("Focus: {:?}", agent.focus_area))
        }
        Action::AgentSelectFileNext => {
            agent.select_file_next();
            None
        }
        Action::AgentSelectFilePrev => {
            agent.select_file_prev();
            None
        }
        Action::AgentInsertSelectedFile => {
            agent.insert_selected_file_to_input();
            None
        }
        Action::AgentToggleFileTreeExpand => {
            agent.toggle_file_tree_expand();
            None
        }
        Action::AgentScanFiles => {
            agent.scan_workspace_files();
            None
        }
        Action::AgentScrollUp => {
            agent.scroll_up();
            None
        }
        Action::AgentScrollDown => {
            agent.scroll_down();
            None
        }
        Action::AgentToggleToolExpand => {
            agent.toggle_tool_expand();
            None
        }
        Action::AgentTagQueryNext => {
            agent.select_tag_query_next();
            None
        }
        Action::AgentTagQueryPrev => {
            agent.select_tag_query_prev();
            None
        }
        Action::AgentApplyTag => {
            agent.apply_selected_tag();
            None
        }
        Action::AgentCloseTagQuery => {
            agent.close_tag_query();
            None
        }
        Action::AgentToggleReasoning => {
            agent.toggle_reasoning();
            let label = if agent.reasoning_expanded { "expanded" } else { "collapsed" };
            Some(format!("Reasoning {}", label))
        }
        Action::AgentScrollToTop => {
            agent.scroll_to_top();
            Some("Scrolled to top".to_string())
        }
        Action::AgentScrollToBottom => {
            agent.scroll_to_bottom();
            Some("Scrolled to bottom".to_string())
        }
        Action::AgentApprovePending => {
            if let Some(req) = agent.approve_pending() {
                Some(format!("Execution approved: {}", req.tool_name))
            } else {
                None
            }
        }
        Action::AgentRejectPending => {
            if let Some(req) = agent.reject_pending() {
                Some(format!("Execution rejected: {}", req.tool_name))
            } else {
                None
            }
        }
        Action::AgentCycleToolFilter => {
            agent.cycle_tool_filter();
            Some(format!("Tool Filter: {:?}", agent.tool_filter))
        }
        Action::AgentToggleToolCard(idx) => {
            agent.toggle_tool_card(idx);
            None
        }
        Action::AgentSwitchSidebarTab => {
            agent.switch_sidebar_tab();
            if matches!(agent.focus_area, AgentFocusArea::Tasks | AgentFocusArea::Files) {
                agent.focus_area = match agent.sidebar_tab {
                    AgentSidebarTab::Tasks => AgentFocusArea::Tasks,
                    AgentSidebarTab::Files => AgentFocusArea::Files,
                };
            }
            let tab_name = match agent.sidebar_tab {
                AgentSidebarTab::Tasks => "Tasks",
                AgentSidebarTab::Files => "Files",
            };
            Some(format!("Sidebar: {} (Focus: {:?})", tab_name, agent.focus_area))
        }
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
            Some(format!("Refreshed models from core providers ({} available)", count))
        }
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
        Action::AgentSubmitAddFolder => {
            match agent.submit_add_folder() {
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
                    Some(format!("Folder attached successfully (Total: {} folders)", count))
                }
                Err(err) => {
                    Some(format!("Failed to attach folder: {}", err))
                }
            }
        }
        Action::AgentRemoveActiveFolder => {
            let idx = agent.active_folder_idx;
            match agent.remove_workspace_folder(idx) {
                Ok(()) => {
                    if let (Some(active_pid), Some(tx)) = (&agent.active_project_id, bridge_cmd_tx) {
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
                Err(err) => {
                    Some(format!("Failed to detach folder: {}", err))
                }
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
                Ok(None) => {
                    Some("Folder selection cancelled".to_string())
                }
                Err(err) => {
                    Some(format!("File manager dialog error: {}", err))
                }
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
                        agent.workspace_folders.get(agent.active_folder_idx).cloned()
                    }
                } else {
                    agent.workspace_folders.get(agent.active_folder_idx).cloned()
                }
            } else {
                agent.workspace_folders.get(agent.active_folder_idx).cloned()
            };

            if let Some(folder) = target_path {
                match ragasukma_core::agent::tools::open_path_in_file_manager(&folder.to_string_lossy()) {
                    Ok(()) => {
                        Some(format!("Opened file manager for {}", folder.display()))
                    }
                    Err(err) => {
                        Some(format!("Failed to open file manager: {}", err))
                    }
                }
            } else {
                None
            }
        }
        Action::AgentOpenProjectModal => {
            agent.project_modal.is_open = true;
            Some("Project switcher opened. Select a project or press 'n' to create new.".to_string())
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
                    Some(format!("Beralih ke project: {}", target_name))
                } else {
                    None
                }
            } else {
                None
            }
        }
        Action::AgentOpenCreateProjectModal => {
            agent.project_modal.is_open = false;
            agent.create_project_modal.is_open = true;
            agent.create_project_modal.name_input.clear();
            Some("Buat Project Baru".to_string())
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
                    let initial_folders: Vec<String> = if let Some(f) = agent.workspace_folders.get(agent.active_folder_idx) {
                        vec![f.to_string_lossy().to_string()]
                    } else if let Ok(cwd) = std::env::current_dir() {
                        vec![cwd.to_string_lossy().to_string()]
                    } else {
                        Vec::new()
                    };
                    let root_path = if initial_folders.is_empty() {
                        None
                    } else {
                        serde_json::to_string(&initial_folders).ok()
                    };

                    let _ = tx.send(CoreBridgeCommand::CreateProject {
                        name: name.clone(),
                        description: None,
                        root_path,
                    });
                }
                Some(format!("Project '{}' dibuat", name))
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