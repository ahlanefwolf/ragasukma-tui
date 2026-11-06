use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

use crate::agent_view::state::AgentWorkspaceState;
use crate::agent_view::types::AgentFocusArea;
use crate::theme::Theme;

pub fn render_inspector(frame: &mut Frame, area: Rect, state: &AgentWorkspaceState) {
    frame.render_widget(Clear, area);

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

    let cur_session = state
        .active_session_id
        .as_ref()
        .and_then(|aid| state.sessions.iter().find(|s| &s.id == aid))
        .or_else(|| state.sessions.get(state.selected_session_idx));

    let model_name = cur_session
        .and_then(|s| s.model.as_deref())
        .filter(|m| !m.is_empty())
        .unwrap_or(&state.active_model);

    let in_formatted = super::helpers::format_compact_number(breakdown.task_input_tokens);
    let out_formatted = super::helpers::format_compact_number(breakdown.task_output_tokens);
    let total_formatted = super::helpers::format_compact_number(total_task);
    let total_exact = super::helpers::format_comma_number(total_task);

    let lines_model = vec![
        Line::from(vec![
            Span::styled("Model   : ", Style::default().fg(Theme::TEXT_SECONDARY)),
            Span::styled(
                format!("[◈ {}]", model_name),
                Style::default().fg(Theme::TEXT_PRIMARY),
            ),
        ]),
        Line::from(vec![
            Span::styled("Requests: ", Style::default().fg(Theme::TEXT_SECONDARY)),
            Span::styled(
                format!(
                    "{} req{}",
                    super::helpers::format_comma_number(breakdown.task_requests),
                    if breakdown.task_requests == 1 {
                        ""
                    } else {
                        "s"
                    }
                ),
                Style::default().fg(Theme::TEXT_PRIMARY),
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
                Style::default().fg(Theme::TEXT_PRIMARY),
            ),
        ]),
        Line::from(vec![
            Span::styled("Total   : ", Style::default().fg(Theme::TEXT_SECONDARY)),
            Span::styled(
                format!("{} tokens ({})", total_formatted, total_exact),
                Style::default().fg(Theme::TEXT_PRIMARY),
            ),
        ]),
    ];
    frame.render_widget(Paragraph::new(lines_model).block(model_block), chunks[0]);

    let block_tools = Block::default()
        .title(if is_tools_focused {
            Span::styled(" 🔌 MCP Servers [Focused] ", Theme::badge_gold())
        } else {
            Span::styled(" 🔌 MCP Servers ", Theme::header_title())
        })
        .borders(Borders::ALL)
        .border_style(if is_tools_focused {
            Theme::border_focused()
        } else {
            Theme::border_subtle()
        })
        .style(Theme::block_base());

    // All MCP servers from the global `mcp_servers` table.
    // [ON]/[OFF] reflects the per-project link in `project_mcp_servers`;
    // Space toggles the link (add/remove), synced via AgentToggleMcpServer.
    let mut tools_lines: Vec<Line> = Vec::new();
    if state.mcp_servers.is_empty() {
        tools_lines.push(Line::from(Span::styled(
            "  (no MCP servers configured)",
            Style::default().fg(Theme::TEXT_SECONDARY),
        )));
        tools_lines.push(Line::from(Span::styled(
            "  Press 'r' to load; add servers in MCP view",
            Style::default().fg(Theme::TEXT_SECONDARY),
        )));
    } else {
        let selected = state.selected_mcp_idx.min(state.mcp_servers.len() - 1);
        for (idx, server) in state.mcp_servers.iter().enumerate() {
            let enabled = state.mcp_active_ids.iter().any(|id| id == &server.id);
            let is_selected = is_tools_focused && idx == selected;
            let cursor = if is_selected { "▶ " } else { "  " };
            let marker = if enabled {
                Span::styled("[ON]  ", Theme::status_success())
            } else {
                Span::styled("[OFF] ", Style::default().fg(Theme::TEXT_SECONDARY))
            };
            let name_style = if is_selected {
                Style::default()
                    .fg(Theme::TEXT_PRIMARY)
                    .add_modifier(ratatui::style::Modifier::BOLD)
            } else if enabled {
                Style::default().fg(Theme::TEXT_PRIMARY)
            } else {
                Style::default().fg(Theme::TEXT_SECONDARY)
            };
            tools_lines.push(Line::from(vec![
                Span::styled(cursor, Theme::badge_gold()),
                marker,
                Span::styled(server.name.clone(), name_style),
            ]));
        }
    }

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
