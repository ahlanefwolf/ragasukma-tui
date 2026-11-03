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
    let center_chat_area = chunks[current_idx];
    chat::render_center_chat(frame, center_chat_area, state);
    current_idx += 1;

    let chat_sub_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(6), Constraint::Length(3)])
        .split(center_chat_area);
    let input_area = chat_sub_chunks[1];

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
        modals::render_tag_query_popup(frame, input_area, tq);
    }
}

pub fn get_footer_key_hints(state: &AgentWorkspaceState, width: u16) -> Vec<Span<'static>> {
    let mut hints = Vec::new();

    // 1. Modal: Add Folder to Workspace
    if state.add_folder_modal.is_open {
        hints.push(Span::styled(" [📂 ADD FOLDER] ", Theme::badge_cyan()));
        hints.push(Span::styled(" [Enter] ", Theme::key_badge()));
        hints.push(Span::raw("Confirm "));
        hints.push(Span::styled(" [Tab] ", Theme::key_badge()));
        hints.push(Span::raw("Browse/Complete "));
        hints.push(Span::styled(" [Esc] ", Theme::key_badge()));
        hints.push(Span::raw("Cancel "));
        return hints;
    }

    // 2. Modal: Create Project
    if state.create_project_modal.is_open {
        hints.push(Span::styled(" [➕ NEW PROJECT] ", Theme::badge_cyan()));
        hints.push(Span::styled(" [Enter] ", Theme::key_badge()));
        hints.push(Span::raw("Create "));
        hints.push(Span::styled(" [Esc] ", Theme::key_badge()));
        hints.push(Span::raw("Cancel "));
        return hints;
    }

    // 3. Modal: Project Switcher / List Project
    if state.project_modal.is_open {
        hints.push(Span::styled(" [📦 PROJECTS] ", Theme::badge_cyan()));
        hints.push(Span::styled(" [↑/↓|j/k] ", Theme::key_badge()));
        hints.push(Span::raw("Select "));
        hints.push(Span::styled(" [Enter] ", Theme::key_badge()));
        hints.push(Span::raw("Switch "));
        hints.push(Span::styled(" [n] ", Theme::key_badge()));
        hints.push(Span::raw("New Project "));
        hints.push(Span::styled(" [Esc] ", Theme::key_badge()));
        hints.push(Span::raw("Close "));
        return hints;
    }

    // 4. Modal: Pending Tool Approval / Diff Review
    if state.pending_approval.is_some() {
        hints.push(Span::styled(" [⚠️ APPROVAL] ", Theme::badge_gold()));
        hints.push(Span::styled(" [y] ", Theme::key_badge()));
        hints.push(Span::raw("Approve "));
        hints.push(Span::styled(" [n] ", Theme::key_badge()));
        hints.push(Span::raw("Reject "));
        hints.push(Span::styled(" [↑/↓|j/k] ", Theme::key_badge()));
        hints.push(Span::raw("Scroll Diff "));
        return hints;
    }

    // 5. Popup: Tag Picker / Tag Query active
    if let Some(ref tq) = state.tag_query {
        hints.push(Span::styled(" [🏷️ TAG POPUP] ", Theme::badge_cyan()));
        hints.push(Span::styled(" [↑/↓] ", Theme::key_badge()));
        hints.push(Span::raw("Select "));
        hints.push(Span::styled(" [Enter/Tab] ", Theme::key_badge()));
        hints.push(Span::raw("Insert "));
        hints.push(Span::styled(" [Esc] ", Theme::key_badge()));
        hints.push(Span::raw("Close "));
        if tq.has_more() && width > 80 {
            hints.push(Span::styled(" [↓ Scroll more] ", Theme::badge_gold()));
        }
        return hints;
    }

    // Reactive Focus Card Badge
    let card_badge = match state.focus_area {
        AgentFocusArea::Folders => Span::styled(" [📁 FOLDERS] ", Theme::badge_gold()),
        AgentFocusArea::Tasks => Span::styled(" [📋 TASKS] ", Theme::badge_gold()),
        AgentFocusArea::Files => Span::styled(" [🗂️ FILES] ", Theme::badge_gold()),
        AgentFocusArea::Chat => Span::styled(" [💬 CHAT] ", Theme::badge_gold()),
        AgentFocusArea::Input if state.is_input_focused => {
            Span::styled(" [✏️ INPUT:INSERT] ", Theme::badge_gold())
        }
        AgentFocusArea::Input => Span::styled(" [✏️ INPUT:NORMAL] ", Theme::badge_cyan()),
        AgentFocusArea::Inspector => Span::styled(" [🔍 INSPECTOR] ", Theme::badge_gold()),
    };
    hints.push(card_badge);

    // Context-sensitive card hints matching actual controller keybindings
    match state.focus_area {
        AgentFocusArea::Folders => {
            hints.push(Span::styled(" [↑/↓|j/k] ", Theme::key_badge()));
            hints.push(Span::raw("Select "));
            hints.push(Span::styled(" [Enter] ", Theme::key_badge()));
            hints.push(Span::raw("Focus Files "));
            hints.push(Span::styled(" [a] ", Theme::key_badge()));
            hints.push(Span::raw("Add "));
            hints.push(Span::styled(" [d] ", Theme::key_badge()));
            hints.push(Span::raw("Remove "));
            if width > 90 {
                hints.push(Span::styled(" [o] ", Theme::key_badge()));
                hints.push(Span::raw("Open "));
                hints.push(Span::styled(" [P] ", Theme::key_badge()));
                hints.push(Span::raw("Projects "));
            }
        }
        AgentFocusArea::Tasks => {
            hints.push(Span::styled(" [↑/↓|j/k] ", Theme::key_badge()));
            hints.push(Span::raw("Select "));
            hints.push(Span::styled(" [Enter] ", Theme::key_badge()));
            hints.push(Span::raw("Switch Task "));
            hints.push(Span::styled(" [n] ", Theme::key_badge()));
            hints.push(Span::raw("New "));
            hints.push(Span::styled(" [d] ", Theme::key_badge()));
            hints.push(Span::raw("Delete "));
            hints.push(Span::styled(" [s] ", Theme::key_badge()));
            hints.push(Span::raw("Files Tab "));
            if width > 95 {
                hints.push(Span::styled(" [P] ", Theme::key_badge()));
                hints.push(Span::raw("Projects "));
            }
        }
        AgentFocusArea::Files => {
            hints.push(Span::styled(" [↑/↓|j/k] ", Theme::key_badge()));
            hints.push(Span::raw("Select "));
            hints.push(Span::styled(" [Enter] ", Theme::key_badge()));
            hints.push(Span::raw("Expand/Collapse "));
            hints.push(Span::styled(" [Space] ", Theme::key_badge()));
            hints.push(Span::raw("Insert Path "));
            hints.push(Span::styled(" [s] ", Theme::key_badge()));
            hints.push(Span::raw("Tasks Tab "));
            if width > 90 {
                hints.push(Span::styled(" [a] ", Theme::key_badge()));
                hints.push(Span::raw("Add Folder "));
                hints.push(Span::styled(" [r] ", Theme::key_badge()));
                hints.push(Span::raw("Rescan "));
                hints.push(Span::styled(" [o] ", Theme::key_badge()));
                hints.push(Span::raw("Open "));
            }
        }
        AgentFocusArea::Chat => {
            hints.push(Span::styled(" [i/Enter] ", Theme::key_badge()));
            hints.push(Span::raw("Write "));
            hints.push(Span::styled(" [↑/↓|j/k] ", Theme::key_badge()));
            hints.push(Span::raw("Scroll "));
            hints.push(Span::styled(" [m] ", Theme::key_badge()));
            hints.push(Span::raw("Model "));
            hints.push(Span::styled(" [c] ", Theme::key_badge()));
            hints.push(Span::raw("Clear "));
            if width > 95 {
                hints.push(Span::styled(" [x] ", Theme::key_badge()));
                hints.push(Span::raw("Tools "));
                hints.push(Span::styled(" [r] ", Theme::key_badge()));
                hints.push(Span::raw("Reasoning "));
                hints.push(Span::styled(" [b] ", Theme::key_badge()));
                hints.push(Span::raw("Sidebar "));
            }
        }
        AgentFocusArea::Input if state.is_input_focused => {
            hints.push(Span::styled(" [Enter] ", Theme::key_badge()));
            hints.push(Span::raw("Send Prompt "));
            hints.push(Span::styled(" [@] ", Theme::key_badge()));
            hints.push(Span::raw("Tag File/Folder "));
            hints.push(Span::styled(" [Esc] ", Theme::key_badge()));
            hints.push(Span::raw("Normal Mode "));
            hints.push(Span::styled(" [Ctrl+U] ", Theme::key_badge()));
            hints.push(Span::raw("Clear "));
        }
        AgentFocusArea::Input => {
            hints.push(Span::styled(" [i/Enter] ", Theme::key_badge()));
            hints.push(Span::raw("Insert Mode "));
            hints.push(Span::styled(" [m] ", Theme::key_badge()));
            hints.push(Span::raw("Model "));
            hints.push(Span::styled(" [↑/↓] ", Theme::key_badge()));
            hints.push(Span::raw("Scroll Chat "));
        }
        AgentFocusArea::Inspector => {
            hints.push(Span::styled(" [↑/↓|j/k] ", Theme::key_badge()));
            hints.push(Span::raw("Scroll "));
            hints.push(Span::styled(" [t] ", Theme::key_badge()));
            hints.push(Span::raw("Filter "));
            hints.push(Span::styled(" [x] ", Theme::key_badge()));
            hints.push(Span::raw("Expand "));
            hints.push(Span::styled(" [r] ", Theme::key_badge()));
            hints.push(Span::raw("Reasoning "));
            hints.push(Span::styled(" [m] ", Theme::key_badge()));
            hints.push(Span::raw("Model "));
        }
    }

    // Common navigation & model badge
    hints.push(Span::styled(" [Tab] ", Theme::key_badge()));
    hints.push(Span::raw("Next Card "));

    // Reactive Model Badge
    if width > 115 {
        let model_display = format!(" 🤖 {} ", state.active_model);
        hints.push(Span::styled(model_display, Theme::badge_cyan()));
    }

    hints
}