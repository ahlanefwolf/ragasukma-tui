use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap,
    },
    Frame,
};

use super::message_renderer::{calculate_total_visual_lines, render_message_standard};
use crate::agent_view::state::AgentWorkspaceState;
use crate::agent_view::types::AgentFocusArea;
use crate::theme::Theme;

pub fn render_center_chat(frame: &mut Frame, area: Rect, state: &AgentWorkspaceState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(6), Constraint::Length(3)])
        .split(area);

    let is_chat_focused = state.focus_area == AgentFocusArea::Chat;
    let is_input_focused = state.focus_area == AgentFocusArea::Input || state.is_input_focused;

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

    let chat_inner = chat_block.inner(chunks[0]);
    let text_width = chat_inner.width.saturating_sub(1) as usize;
    let inner_height = chat_inner.height as usize;

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
            chat_lines.extend(render_message_standard(msg));
        }
        // Tambahkan baris kosong ekstra di akhir agar pesan terbawah selalu terlihat utuh dan tidak mepet
        chat_lines.push(Line::from(""));
        chat_lines.push(Line::from(""));
    }

    let calculated_visual = calculate_total_visual_lines(&chat_lines, text_width);
    let total_visual_lines = calculated_visual.max(chat_lines.len());

    let max_scroll = total_visual_lines.saturating_sub(inner_height);
    state.chat_max_scroll.set(max_scroll);
    let effective_scroll = if state.auto_scroll {
        max_scroll
    } else {
        max_scroll.saturating_sub(state.scroll_offset)
    };

    frame.render_widget(Clear, chunks[0]);
    frame.render_widget(chat_block, chunks[0]);

    let text_area = Rect {
        x: chat_inner.x,
        y: chat_inner.y,
        width: chat_inner.width.saturating_sub(1),
        height: chat_inner.height,
    };
    let scrollbar_area = Rect {
        x: chat_inner.x + chat_inner.width.saturating_sub(1),
        y: chat_inner.y,
        width: 1.min(chat_inner.width),
        height: chat_inner.height,
    };

    let p = Paragraph::new(chat_lines)
        .wrap(Wrap { trim: false })
        .scroll((effective_scroll as u16, 0));
    frame.render_widget(p, text_area);

    if max_scroll > 0 && scrollbar_area.width > 0 {
        let mut scrollbar_state = ScrollbarState::new(max_scroll).position(effective_scroll);
        frame.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("▲"))
                .end_symbol(Some("▼"))
                .thumb_symbol("█")
                .track_symbol(Some("│"))
                .style(Style::default().fg(Theme::CYAN_ACCENT)),
            scrollbar_area,
            &mut scrollbar_state,
        );
    }

    // Input Bar Card
    let input_border = if state.tag_query.is_some() {
        Style::default().fg(Theme::CYAN_ACCENT)
    } else if is_input_focused {
        Theme::border_focused()
    } else {
        Theme::border_subtle()
    };

    let input_title = if state.tag_query.is_some() {
        Span::styled(" 🏷️ Tag File Mode (@) ", Theme::badge_cyan())
    } else if is_input_focused {
        Span::styled(" ✏️ Message Input [Active] ", Theme::badge_gold())
    } else {
        Span::styled(" ✏️ Message Input ", Theme::badge_gray())
    };

    let prompt_prefix = Span::styled(
        "❯ ",
        Style::default()
            .fg(if state.tag_query.is_some() {
                Theme::CYAN_ACCENT
            } else {
                Theme::GOLD_PRIMARY
            })
            .add_modifier(Modifier::BOLD),
    );

    let input_line = if state.input_buffer.is_empty() {
        let placeholder = if is_input_focused {
            Span::styled(
                "Type a prompt, or @ to attach file context... (Enter to send)",
                Style::default().fg(Theme::TEXT_SECONDARY).add_modifier(Modifier::ITALIC),
            )
        } else {
            Span::styled(
                "Press [i] or [Enter] to write a message...",
                Style::default().fg(Theme::TEXT_SECONDARY),
            )
        };
        Line::from(vec![prompt_prefix, placeholder])
    } else {
        // Dynamic syntax highlighting for @file tags in input buffer
        let mut spans = vec![prompt_prefix];
        let mut current_idx = 0;
        let buf = &state.input_buffer;

        while let Some(at_idx) = buf[current_idx..].find('@') {
            let actual_at = current_idx + at_idx;
            if actual_at > current_idx {
                spans.push(Span::styled(
                    &buf[current_idx..actual_at],
                    Style::default().fg(Theme::TEXT_PRIMARY),
                ));
            }
            // Find end of token (whitespace or end of string)
            let rest = &buf[actual_at..];
            let end_idx = rest.find(|c: char| c.is_whitespace()).unwrap_or(rest.len());
            spans.push(Span::styled(
                &buf[actual_at..actual_at + end_idx],
                Style::default()
                    .fg(Theme::CYAN_ACCENT)
                    .add_modifier(Modifier::BOLD),
            ));
            current_idx = actual_at + end_idx;
        }

        if current_idx < buf.len() {
            spans.push(Span::styled(
                &buf[current_idx..],
                Style::default().fg(Theme::TEXT_PRIMARY),
            ));
        }

        Line::from(spans)
    };

    let model_tag_span = Span::styled(
        format!(" [◈ {}] ", state.active_model),
        Style::default()
            .fg(Theme::GOLD_PRIMARY)
            .add_modifier(Modifier::BOLD),
    );

    let input_block = Block::default()
        .title(input_title)
        .title_top(Line::from(model_tag_span).right_aligned())
        .borders(Borders::ALL)
        .border_style(input_border)
        .style(Theme::block_base());

    let input_p = Paragraph::new(input_line).block(input_block);
    frame.render_widget(input_p, chunks[1]);

    // Position terminal cursor when input is focused for intuitive typing UX
    if is_input_focused {
        let prompt_width = 2u16; // "❯ "
        let cursor_offset = state.input_buffer[..state.cursor_pos].chars().count() as u16;
        let cursor_x = (chunks[1].x + 1 + prompt_width + cursor_offset)
            .min(chunks[1].x + chunks[1].width.saturating_sub(2));
        let cursor_y = chunks[1].y + 1;
        frame.set_cursor_position((cursor_x, cursor_y));
    }
}
