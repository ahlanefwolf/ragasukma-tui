use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::agent_view::state::AgentWorkspaceState;
use crate::agent_view::types::{AgentFocusArea, MessageRole, ToolExecutionStatus};
use crate::theme::Theme;

pub fn render_center_chat(frame: &mut Frame, area: Rect, state: &AgentWorkspaceState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(6), Constraint::Length(3)])
        .split(area);

    let is_chat_focused = state.focus_area == AgentFocusArea::Chat;
    let is_input_focused = state.focus_area == AgentFocusArea::Input || state.is_input_focused;

    let mut chat_lines = Vec::new();
    if state.messages.is_empty() {
        chat_lines.push(Line::from(vec![
            Span::styled("Welcome to ", Style::default().fg(Theme::TEXT_SECONDARY)),
            Span::styled(
                "Ragasukma AI Agent",
                Style::default().fg(Theme::GOLD_PRIMARY).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "! Type your message below or press @ to attach files.",
                Style::default().fg(Theme::TEXT_SECONDARY),
            ),
        ]));
    } else {
        for msg in &state.messages {
            let (badge, badge_style) = match msg.role {
                MessageRole::User => (" You ", Theme::badge_info()),
                MessageRole::Assistant => (" Assistant ", Theme::badge_gold()),
                MessageRole::System => (" System ", Theme::badge_gray()),
                MessageRole::Tool => (" Tool ", Theme::badge_cyan()),
            };

            chat_lines.push(Line::from(vec![
                Span::styled(badge, badge_style),
                Span::raw(" "),
                Span::styled(
                    format!("[{}]", msg.timestamp),
                    Style::default().fg(Theme::TEXT_MUTED),
                ),
            ]));

            for line in msg.content.lines() {
                chat_lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(line, Style::default().fg(Theme::TEXT_PRIMARY)),
                ]));
            }

            for tc in &msg.tool_calls {
                let status_icon = match tc.status {
                    ToolExecutionStatus::Running => "⏳ Running...",
                    ToolExecutionStatus::Success => "✓ Completed",
                    ToolExecutionStatus::Failed => "✗ Failed",
                    ToolExecutionStatus::PendingApproval => "⚠️ Pending Approval",
                };
                let status_color = match tc.status {
                    ToolExecutionStatus::Running => Theme::CYAN_ACCENT,
                    ToolExecutionStatus::Success => Theme::STATUS_SUCCESS,
                    ToolExecutionStatus::Failed => Theme::STATUS_ERROR,
                    ToolExecutionStatus::PendingApproval => Theme::STATUS_WARNING,
                };

                chat_lines.push(Line::from(vec![
                    Span::raw("  🛠️ ["),
                    Span::styled(
                        &tc.name,
                        Style::default().fg(Theme::GOLD_PRIMARY).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("] "),
                    Span::styled(status_icon, Style::default().fg(status_color)),
                ]));
            }
            chat_lines.push(Line::from(""));
        }
    }

    let chat_title = if is_chat_focused {
        Span::styled(" 💬 Agent Conversation [Focused] ", Theme::badge_gold())
    } else {
        Span::styled(" 💬 Agent Conversation ", Theme::header_title())
    };

    let chat_block = Block::default()
        .title(chat_title)
        .borders(Borders::ALL)
        .border_style(if is_chat_focused {
            Theme::border_focused()
        } else {
            Theme::border_subtle()
        })
        .style(Theme::block_base());

    let p = Paragraph::new(chat_lines)
        .block(chat_block)
        .wrap(Wrap { trim: false })
        .scroll((state.scroll_offset as u16, 0));
    frame.render_widget(p, chunks[0]);

    // Input Bar Card
    let input_border = if is_input_focused {
        Theme::border_focused()
    } else {
        Theme::border_subtle()
    };

    let input_title = if is_input_focused {
        Span::styled(" ✏️ Message Input [Active] ", Theme::badge_gold())
    } else {
        Span::styled(" ✏️ Message Input ", Theme::badge_gray())
    };

    let prompt_prefix = Span::styled(
        "❯ ",
        Style::default().fg(Theme::GOLD_PRIMARY).add_modifier(Modifier::BOLD),
    );
    let input_text = Span::styled(&state.input_buffer, Style::default().fg(Theme::TEXT_PRIMARY));

    let input_block = Block::default()
        .title(input_title)
        .borders(Borders::ALL)
        .border_style(input_border)
        .style(Theme::block_base());

    let input_p = Paragraph::new(Line::from(vec![prompt_prefix, input_text])).block(input_block);
    frame.render_widget(input_p, chunks[1]);
}
