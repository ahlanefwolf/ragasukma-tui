use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

use crate::agent_view::state::AgentWorkspaceState;
use crate::agent_view::types::AgentFocusArea;
use crate::theme::Theme;

pub fn render_inspector(frame: &mut Frame, area: Rect, state: &AgentWorkspaceState) {
    frame.render_widget(Clear, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(7), Constraint::Min(5)])
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

    let cur_session = state
        .active_session_id
        .as_ref()
        .and_then(|aid| state.sessions.iter().find(|s| &s.id == aid))
        .or_else(|| state.sessions.get(state.selected_session_idx));
    let is_live_turn = state.is_running
        || cur_session
            .map(|s| s.is_running)
            .unwrap_or(false);
    let live_badge = if is_live_turn { " ⚡ live" } else { "" };

    let model_name = cur_session
        .and_then(|s| s.model.as_deref())
        .filter(|m| !m.is_empty())
        .unwrap_or(&state.active_model);

    let active_secs = state.session_start_instant.elapsed().as_secs();
    let hrs = active_secs / 3600;
    let mins = (active_secs % 3600) / 60;
    let secs = active_secs % 60;
    let dur_str = if hrs > 0 {
        format!("{}h {:02}m {:02}s", hrs, mins, secs)
    } else {
        format!("{}m {:02}s", mins, secs)
    };

    let in_formatted = super::helpers::format_compact_number(breakdown.task_input_tokens);
    let out_formatted = super::helpers::format_compact_number(breakdown.task_output_tokens);
    let total_formatted = super::helpers::format_compact_number(total_task);
    let total_exact = super::helpers::format_comma_number(total_task);

    let lines_model = vec![
        Line::from(vec![
            Span::styled("Model   : ", Style::default().fg(Theme::TEXT_SECONDARY)),
            Span::styled(
                format!("[◈ {}]", model_name),
                Style::default()
                    .fg(Theme::GOLD_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Requests: ", Style::default().fg(Theme::TEXT_SECONDARY)),
            Span::styled(
                format!("{} req{}{}", breakdown.task_requests, if breakdown.task_requests == 1 { "" } else { "s" }, live_badge),
                if is_live_turn {
                    Style::default().fg(Theme::CYAN_ACCENT).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Theme::CYAN_ACCENT)
                },
            ),
        ]),
        Line::from(vec![
            Span::styled("Tokens  : ", Style::default().fg(Theme::TEXT_SECONDARY)),
            Span::styled(
                format!("▲ {} in", in_formatted),
                Style::default().fg(Theme::TEXT_PRIMARY),
            ),
            Span::styled("  •  ", Style::default().fg(Theme::TEXT_SECONDARY)),
            Span::styled(
                format!("▼ {} out", out_formatted),
                if is_live_turn {
                    Style::default().fg(Theme::GOLD_PRIMARY).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Theme::TEXT_PRIMARY)
                },
            ),
        ]),
        Line::from(vec![
            Span::styled("Total   : ", Style::default().fg(Theme::TEXT_SECONDARY)),
            Span::styled(
                format!("{} tokens ({})", total_formatted, total_exact),
                Style::default()
                    .fg(Theme::GOLD_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Session : ", Style::default().fg(Theme::TEXT_SECONDARY)),
            Span::styled(
                if is_live_turn { "RUNNING" } else { "IDLE" },
                if is_live_turn {
                    Style::default().fg(Theme::CYAN_ACCENT).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Theme::TEXT_MUTED)
                },
            ),
            Span::styled("  •  ", Style::default().fg(Theme::TEXT_SECONDARY)),
            Span::styled("Uptime: ", Style::default().fg(Theme::TEXT_SECONDARY)),
            Span::styled(
                dur_str,
                Style::default().fg(Theme::TEXT_PRIMARY),
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
    let tools_inner = block_tools.inner(chunks[1]);
    let inner_height = tools_inner.height as usize;
    let max_scroll = total_tools.saturating_sub(inner_height);
    let scroll_y = state.inspector_scroll_offset.min(max_scroll);

    frame.render_widget(block_tools, chunks[1]);
    frame.render_widget(
        Paragraph::new(tools_lines).scroll((scroll_y as u16, 0)),
        tools_inner,
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
            tools_inner,
            &mut scrollbar_state,
        );
    }
}