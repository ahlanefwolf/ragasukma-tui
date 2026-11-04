use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

use crate::agent_view::state::AgentWorkspaceState;
use crate::agent_view::types::AgentFocusArea;
use crate::theme::Theme;

pub fn render_inspector(frame: &mut Frame, area: Rect, state: &AgentWorkspaceState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(6), Constraint::Min(5)])
        .split(area);

    let is_model_focused = state.focus_area == AgentFocusArea::ModelTelemetry;
    let is_tools_focused = state.focus_area == AgentFocusArea::McpTools;

    let model_block = Block::default()
        .title(if is_model_focused {
            Span::styled(" 🧠 Model Telemetry [Focused] ", Theme::badge_gold())
        } else {
            Span::styled(" 🧠 Model Telemetry ", Theme::header_title())
        })
        .borders(Borders::ALL)
        .border_style(if is_model_focused {
            Theme::border_focused()
        } else {
            Theme::border_subtle()
        })
        .style(Theme::block_base());

    let breakdown = state.calculate_token_breakdown();
    let total_task = breakdown.total_task_tokens();

    let lines_model = vec![
        Line::from(vec![
            Span::styled("Model   : ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                format!("[◈ {}]", state.active_model),
                Style::default()
                    .fg(Theme::GOLD_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Requests: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                format!("{} req{}", breakdown.task_requests, if breakdown.task_requests == 1 { "" } else { "s" }),
                Style::default().fg(Theme::CYAN_ACCENT),
            ),
        ]),
        Line::from(vec![
            Span::styled("Tokens  : ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                format!("▲ {} in", breakdown.task_input_tokens),
                Style::default().fg(Theme::TEXT_PRIMARY),
            ),
            Span::styled("  •  ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                format!("▼ {} out", breakdown.task_output_tokens),
                Style::default().fg(Theme::TEXT_PRIMARY),
            ),
        ]),
        Line::from(vec![
            Span::styled("Total   : ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                format!("{} tokens", total_task),
                Style::default()
                    .fg(Theme::GOLD_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ];
    frame.render_widget(Paragraph::new(lines_model).block(model_block), chunks[0]);

    let block_tools = Block::default()
        .title(if is_tools_focused {
            Span::styled(" 🛠️ MCP Tool Status [Focused] ", Theme::badge_gold())
        } else {
            Span::styled(" 🛠️ MCP Tool Status ", Theme::header_title())
        })
        .borders(Borders::ALL)
        .border_style(if is_tools_focused {
            Theme::border_focused()
        } else {
            Theme::border_subtle()
        })
        .style(Theme::block_base());

    let tools_lines: Vec<Line> = vec![
        Line::from(vec![
            Span::styled("● ", Theme::status_success()),
            Span::styled("workspace_list_dir", Style::default().fg(Theme::TEXT_PRIMARY)),
        ]),
        Line::from(vec![
            Span::styled("● ", Theme::status_success()),
            Span::styled("workspace_read_file", Style::default().fg(Theme::TEXT_PRIMARY)),
        ]),
        Line::from(vec![
            Span::styled("● ", Theme::status_success()),
            Span::styled("workspace_write_file", Style::default().fg(Theme::TEXT_PRIMARY)),
        ]),
        Line::from(vec![
            Span::styled("● ", Theme::status_success()),
            Span::styled("workspace_edit_file", Style::default().fg(Theme::TEXT_PRIMARY)),
        ]),
        Line::from(vec![
            Span::styled("● ", Theme::status_success()),
            Span::styled("workspace_grep", Style::default().fg(Theme::TEXT_PRIMARY)),
        ]),
        Line::from(vec![
            Span::styled("● ", Theme::status_success()),
            Span::styled("workspace_run_cmd", Style::default().fg(Theme::TEXT_PRIMARY)),
        ]),
        Line::from(vec![
            Span::styled("● ", Theme::status_success()),
            Span::styled("quota_fetch_all", Style::default().fg(Theme::TEXT_PRIMARY)),
        ]),
        Line::from(vec![
            Span::styled("● ", Theme::status_success()),
            Span::styled("db_query_execute", Style::default().fg(Theme::TEXT_PRIMARY)),
        ]),
        Line::from(vec![
            Span::styled("● ", Theme::status_success()),
            Span::styled("git_status_check", Style::default().fg(Theme::TEXT_PRIMARY)),
        ]),
        Line::from(vec![
            Span::styled("● ", Theme::status_success()),
            Span::styled("git_diff_inspect", Style::default().fg(Theme::TEXT_PRIMARY)),
        ]),
    ];

    let total_tools = tools_lines.len();
    let inner_height = chunks[1].height.saturating_sub(2) as usize;
    let max_scroll = total_tools.saturating_sub(inner_height);
    let scroll_y = state.inspector_scroll_offset.min(max_scroll);

    frame.render_widget(
        Paragraph::new(tools_lines)
            .scroll((scroll_y as u16, 0))
            .block(block_tools),
        chunks[1],
    );

    if total_tools > inner_height {
        let mut scrollbar_state = ScrollbarState::default()
            .content_length(total_tools)
            .position(scroll_y);
        frame.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("▲"))
                .end_symbol(Some("▼")),
            chunks[1],
            &mut scrollbar_state,
        );
    }
}