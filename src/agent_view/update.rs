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
                agent.messages.push(ChatMessage {
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
            None
        }
        Action::AgentUnfocusInput => {
            agent.unfocus_input();
            None
        }
        Action::AgentFocusFiles => {
            agent.focus_area = AgentFocusArea::Files;
            None
        }
        Action::AgentFocusChat => {
            agent.focus_area = AgentFocusArea::Chat;
            None
        }
        Action::AgentFocusNextPanel => {
            agent.cycle_focus_next();
            None
        }
        Action::AgentFocusPrevPanel => {
            agent.cycle_focus_prev();
            None
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
            let label = if agent.reasoning_expanded { "ditampilkan" } else { "disembunyikan" };
            Some(format!("Reasoning {}", label))
        }
        Action::AgentScrollToTop => {
            agent.scroll_to_top();
            Some("Scroll ke paling atas".to_string())
        }
        Action::AgentScrollToBottom => {
            agent.scroll_to_bottom();
            Some("Scroll ke paling bawah".to_string())
        }
        Action::AgentApprovePending => {
            if let Some(req) = agent.approve_pending() {
                Some(format!("Eksekusi disetujui: {}", req.tool_name))
            } else {
                None
            }
        }
        Action::AgentRejectPending => {
            if let Some(req) = agent.reject_pending() {
                Some(format!("Eksekusi ditolak: {}", req.tool_name))
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
            let tab_name = match agent.sidebar_tab {
                AgentSidebarTab::Tasks => "Tasks",
                AgentSidebarTab::Files => "Files",
            };
            Some(format!("Sidebar tab switched to {}", tab_name))
        }
        Action::AgentNewTask => {
            let default_title = format!("Task #{}", agent.sessions.len() + 1);
            if let Some(tx) = bridge_cmd_tx {
                let _ = tx.send(CoreBridgeCommand::CreateSession {
                    title: default_title,
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
                agent.active_session_id = Some(sid.clone());
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
                agent.active_session_id = Some(sid.clone());
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
                agent.active_session_id = Some(sid.clone());
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
                let _ = tx.send(CoreBridgeCommand::CancelTurn);
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
            Some(format!("Selected model: {}", model_name))
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
                    if agent.add_folder_modal.is_open {
                        agent.add_folder_modal.input_path = selected_path;
                        let _ = agent.submit_add_folder();
                        Some("Folder attached from file manager dialog".to_string())
                    } else {
                        match agent.add_workspace_folder(std::path::PathBuf::from(selected_path)) {
                            Ok(()) => {
                                Some("Folder attached from file manager dialog".to_string())
                            }
                            Err(err) => {
                                Some(format!("Failed to attach folder: {}", err))
                            }
                        }
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
            if let Some(folder) = agent.workspace_folders.get(agent.active_folder_idx) {
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
            Some("Project Switcher dibuka. Pilih project atau tekan 'n' untuk buat baru.".to_string())
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
                    let _ = tx.send(CoreBridgeCommand::CreateProject {
                        name: name.clone(),
                        description: None,
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
