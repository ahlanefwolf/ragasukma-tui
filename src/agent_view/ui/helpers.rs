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

/// Format integer with thousand commas (e.g. 1,234,567)
pub fn format_comma_number(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::new();
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    for (i, c) in chars.iter().enumerate() {
        if i > 0 && (len - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(*c);
    }
    out
}

/// Format tokens compactly (e.g. 1.2M, 45.6K, 950)
pub fn format_compact_number(n: usize) -> String {
    if n >= 1_000_000 {
        format!("{:.2}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

