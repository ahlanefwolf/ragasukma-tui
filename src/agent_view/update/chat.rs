use crate::agent_view::states::AgentWorkspaceState;
use crate::agent_view::types::{AgentFocusArea, AgentSidebarTab, ChatMessage, MessageRole};
use crate::app::state::current_time_str;
use crate::app::types::Action;
use crate::core_bridge::CoreBridgeCommand;
use tokio::sync::mpsc::UnboundedSender;

/// Chat panel: prompt submit, scrolling, focus/panels, file tree & selection, tag query, tool cards, reasoning toggle, sidebar tab.
pub(crate) fn handle_action(
    agent: &mut AgentWorkspaceState,
    action: Action,
    bridge_cmd_tx: Option<&UnboundedSender<CoreBridgeCommand>>,
) -> Option<Option<String>> {
    let status = match action {
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
            let label = if agent.reasoning_expanded {
                "expanded"
            } else {
                "collapsed"
            };
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
        Action::AgentSwitchSidebarTab => {
            agent.switch_sidebar_tab();
            if matches!(
                agent.focus_area,
                AgentFocusArea::Tasks | AgentFocusArea::Files | AgentFocusArea::McpTools
            ) {
                agent.focus_area = match agent.sidebar_tab {
                    AgentSidebarTab::Tasks => AgentFocusArea::Tasks,
                    AgentSidebarTab::Files => AgentFocusArea::Files,
                    AgentSidebarTab::Mcp => AgentFocusArea::McpTools,
                };
            }
            let tab_name = match agent.sidebar_tab {
                AgentSidebarTab::Tasks => "Tasks",
                AgentSidebarTab::Files => "Files",
                AgentSidebarTab::Mcp => "MCP Servers",
            };
            Some(format!(
                "Sidebar: {} (Focus: {:?})",
                tab_name, agent.focus_area
            ))
        }
        _ => return None,
    };

    Some(status)
}
