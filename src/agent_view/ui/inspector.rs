use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::agent_view::states::AgentWorkspaceState;
use crate::agent_view::types::AgentFocusArea;
use crate::theme::Theme;

/// Fixed inner content height needed by the Model Telemetry card.
/// 4 data lines + 2 borders (top/bottom).
pub const MODEL_TELEMETRY_HEIGHT: u16 = 6;

/// Render the Model Telemetry card into the given `area`.
///
/// The card is anchored at the bottom of the left sidebar. It is a plain
/// widget render (no `Clear`/scroll) so it can share the sidebar column with
/// the folder bar and the tab content above it.
pub fn render_model_telemetry(frame: &mut Frame, area: Rect, state: &AgentWorkspaceState) {
    let is_model_focused = state.focus_area == AgentFocusArea::ModelTelemetry;

    let model_block = Block::default()
        .title(if is_model_focused {
            Span::styled("  Model Telemetry [Focused] ", Theme::badge_gold())
        } else {
            Span::styled("  Model Telemetry ", Theme::header_title())
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
    frame.render_widget(Paragraph::new(lines_model).block(model_block), area);
}
