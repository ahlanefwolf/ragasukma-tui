use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use crate::theme::Theme;
use crate::agent_view::types::AgentFocusArea;

/// Return border style based on whether this card is currently focused
pub fn area_border(current_focus: AgentFocusArea, target_focus: AgentFocusArea) -> Style {
    if current_focus == target_focus {
        Theme::border_focused()
    } else {
        Theme::border_subtle()
    }
}

/// Helper function to create a centered Rect with given percentage width & height
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
