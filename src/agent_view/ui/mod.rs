pub mod chat;
pub mod helpers;
pub mod inspector;
pub mod modals;
pub mod sidebar;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    Frame,
};

pub use modals::render_diff_preview_modal;

pub fn get_help_lines(state: &AgentWorkspaceState) -> (String, &'static str, Vec<Line<'static>>) {
    let focus_str = match state.focus_area {
        AgentFocusArea::Folders => "Attached Folders",
        AgentFocusArea::Tasks => "Tasks Sidebar",
        AgentFocusArea::Files => "Workspace Files Sidebar",
        AgentFocusArea::Chat => "Chat Messages History",
        AgentFocusArea::Input => "Prompt Input Field",
        AgentFocusArea::Inspector => "Right Inspector / Tool Activity",
    };

    let lines = vec![
        Line::from(vec![
            Span::styled("  Tab / Shift+Tab  ", Theme::key_badge()),
            Span::raw(" Cycle focus between panels (Folders -> Tasks/Files -> Chat -> Input -> Inspector)"),
        ]),
        Line::from(vec![
            Span::styled("  a                ", Theme::key_badge()),
            Span::raw(" Add directory / folder to workspace (in Folders)"),
        ]),
        Line::from(vec![
            Span::styled("  d                ", Theme::key_badge()),
            Span::raw(" Delete selected task / Remove active folder"),
        ]),
        Line::from(vec![
            Span::styled("  s                ", Theme::key_badge()),
            Span::raw(" Switch sidebar tab between Tasks and Files"),
        ]),
        Line::from(vec![
            Span::styled("  n                ", Theme::key_badge()),
            Span::raw(" Create new task (in Tasks)"),
        ]),
        Line::from(vec![
            Span::styled("  Space            ", Theme::key_badge()),
            Span::raw(" Insert selected file path to prompt (in Files)"),
        ]),
        Line::from(vec![
            Span::styled("  i                ", Theme::key_badge()),
            Span::raw(" Focus prompt input field (from Chat)"),
        ]),
        Line::from(vec![
            Span::styled("  c                ", Theme::key_badge()),
            Span::raw(" Clear chat messages (in Chat)"),
        ]),
        Line::from(vec![
            Span::styled("  m / t / r        ", Theme::key_badge()),
            Span::raw(" Inspector: Cycle Model / Filter Tools / Toggle Reasoning"),
        ]),
        Line::from(vec![
            Span::styled("  b                ", Theme::key_badge()),
            Span::raw(" Toggle sidebar visibility"),
        ]),
        Line::from(vec![
            Span::styled("  P                ", Theme::key_badge()),
            Span::raw(" Open Project Manager Switcher modal"),
        ]),
        Line::from(vec![
            Span::styled("  Enter            ", Theme::key_badge()),
            Span::raw(" Confirm / Submit Prompt / Switch Task / Expand Folder"),
        ]),
        Line::from(vec![
            Span::styled("  Esc              ", Theme::key_badge()),
            Span::raw(" Unfocus prompt input / Close active modal"),
        ]),
        Line::from(vec![
            Span::styled("  F1 / ?           ", Theme::key_badge()),
            Span::raw(" Open this Help dialog"),
        ]),
    ];

    (
        "Agent Workspace (Tab [1])".to_string(),
        focus_str,
        lines,
    )
}

use crate::agent_view::state::AgentWorkspaceState;
use crate::agent_view::types::AgentFocusArea;
use crate::theme::Theme;

pub fn render_agent_tab(frame: &mut Frame, area: Rect, state: &AgentWorkspaceState) {
    let mut constraints = Vec::new();
    if state.is_sidebar_visible {
        constraints.push(Constraint::Percentage(22));
    }
    constraints.push(Constraint::Min(40));
    if state.is_inspector_visible {
        constraints.push(Constraint::Percentage(26));
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area);

    let mut current_idx = 0;

    // Left Sidebar
    if state.is_sidebar_visible {
        sidebar::render_left_sidebar(frame, chunks[current_idx], state);
        current_idx += 1;
    }

    // Center Chat Panel
    chat::render_center_chat(frame, chunks[current_idx], state);
    current_idx += 1;

    // Right Inspector Panel
    if state.is_inspector_visible {
        inspector::render_right_inspector(frame, chunks[current_idx], state);
    }

    // Floating Modals
    if state.project_modal.is_open {
        modals::render_project_switcher_modal(frame, area, state);
    }
    if state.create_project_modal.is_open {
        modals::render_create_project_modal(frame, area, &state.create_project_modal);
    }
    if state.add_folder_modal.is_open {
        modals::render_add_folder_modal(frame, area, &state.add_folder_modal);
    }
    if let Some(ref tq) = state.tag_query {
        modals::render_tag_query_popup(frame, area, tq);
    }
}

pub fn get_footer_key_hints(agent: &AgentWorkspaceState, width: u16) -> Vec<Span<'static>> {
    let mut hints = Vec::new();

    // Contextual hints based on currently focused card
    match agent.focus_area {
        AgentFocusArea::Tasks => {
            hints.push(Span::styled(" [Tab] ", Theme::key_badge()));
            hints.push(Span::raw("Next Card "));
            hints.push(Span::styled(" [Enter] ", Theme::key_badge()));
            hints.push(Span::raw("Switch Task "));
            hints.push(Span::styled(" [n] ", Theme::key_badge()));
            hints.push(Span::raw("New Task "));
            hints.push(Span::styled(" [d] ", Theme::key_badge()));
            hints.push(Span::raw("Delete Task "));
            hints.push(Span::styled(" [↑/↓] ", Theme::key_badge()));
            hints.push(Span::raw("Select "));
            hints.push(Span::styled(" [s] ", Theme::key_badge()));
            hints.push(Span::raw("Switch to Files "));
        }
        AgentFocusArea::Files => {
            hints.push(Span::styled(" [Tab] ", Theme::key_badge()));
            hints.push(Span::raw("Next Card "));
            hints.push(Span::styled(" [Enter] ", Theme::key_badge()));
            hints.push(Span::raw("Expand/Collapse "));
            hints.push(Span::styled(" [Space] ", Theme::key_badge()));
            hints.push(Span::raw("Tag File "));
            hints.push(Span::styled(" [o] ", Theme::key_badge()));
            hints.push(Span::raw("Open Folder "));
            hints.push(Span::styled(" [↑/↓] ", Theme::key_badge()));
            hints.push(Span::raw("Select "));
            hints.push(Span::styled(" [s] ", Theme::key_badge()));
            hints.push(Span::raw("Switch to Tasks "));
        }
        AgentFocusArea::Folders => {
            hints.push(Span::styled(" [Tab] ", Theme::key_badge()));
            hints.push(Span::raw("Next Card "));
            hints.push(Span::styled(" [a] ", Theme::key_badge()));
            hints.push(Span::raw("Add Folder "));
            hints.push(Span::styled(" [d] ", Theme::key_badge()));
            hints.push(Span::raw("Remove Folder "));
            hints.push(Span::styled(" [o] ", Theme::key_badge()));
            hints.push(Span::raw("Open Folder "));
            hints.push(Span::styled(" [↑/↓] ", Theme::key_badge()));
            hints.push(Span::raw("Select "));
            hints.push(Span::styled(" [Enter] ", Theme::key_badge()));
            hints.push(Span::raw("View Files "));
        }
        AgentFocusArea::Chat => {
            hints.push(Span::styled(" [Tab] ", Theme::key_badge()));
            hints.push(Span::raw("Next Card "));
            hints.push(Span::styled(" [i] ", Theme::key_badge()));
            hints.push(Span::raw("Write Input "));
            hints.push(Span::styled(" [↑/↓] ", Theme::key_badge()));
            hints.push(Span::raw("Scroll Chat "));
            hints.push(Span::styled(" [x] ", Theme::key_badge()));
            hints.push(Span::raw("Toggle Tools "));
            hints.push(Span::styled(" [c] ", Theme::key_badge()));
            hints.push(Span::raw("Clear Chat "));
        }
        AgentFocusArea::Input => {
            hints.push(Span::styled(" [Enter] ", Theme::key_badge()));
            hints.push(Span::raw("Send Prompt "));
            hints.push(Span::styled(" [Esc] ", Theme::key_badge()));
            hints.push(Span::raw("Unfocus "));
            hints.push(Span::styled(" [@] ", Theme::key_badge()));
            hints.push(Span::raw("Tag File "));
            hints.push(Span::styled(" [Ctrl+U] ", Theme::key_badge()));
            hints.push(Span::raw("Clear Line "));
        }
        AgentFocusArea::Inspector => {
            hints.push(Span::styled(" [Tab] ", Theme::key_badge()));
            hints.push(Span::raw("Next Card "));
            hints.push(Span::styled(" [m] ", Theme::key_badge()));
            hints.push(Span::raw("Model "));
            hints.push(Span::styled(" [t] ", Theme::key_badge()));
            hints.push(Span::raw("Filter Tools "));
            hints.push(Span::styled(" [r] ", Theme::key_badge()));
            hints.push(Span::raw("Reasoning "));
            hints.push(Span::styled(" [x] ", Theme::key_badge()));
            hints.push(Span::raw("Toggle Expand "));
            hints.push(Span::styled(" [↑/↓] ", Theme::key_badge()));
            hints.push(Span::raw("Scroll "));
        }
    }

    if width > 110 {
        hints.push(Span::raw(" | "));
        hints.push(Span::styled(" [B] ", Theme::key_badge()));
        hints.push(Span::raw("Toggle Sidebar "));
        hints.push(Span::styled(" [P] ", Theme::key_badge()));
        hints.push(Span::raw("Projects "));
    }

    hints
}