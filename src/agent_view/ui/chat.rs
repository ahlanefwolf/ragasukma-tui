use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

use super::message_renderer::{render_message_standard, wrap_lines_to_width};
use crate::agent_view::states::AgentWorkspaceState;
use crate::agent_view::types::AgentFocusArea;
use crate::theme::Theme;

pub fn render_center_chat(frame: &mut Frame, area: Rect, state: &AgentWorkspaceState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(6), Constraint::Length(6)])
        .split(area);

    let is_chat_focused = state.focus_area == AgentFocusArea::Chat;
    let is_input_focused = state.focus_area == AgentFocusArea::Input || state.is_input_focused;

    let chat_title = if is_chat_focused {
        Span::styled("  Agent Conversation [Focused] ", Theme::badge_gold())
    } else {
        Span::styled("  Agent Conversation ", Theme::header_title())
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

    let message_count = state.messages.len();
    let last_content_len = state.messages.last().map(|m| m.content.len()).unwrap_or(0);
    let is_running = state.is_running;
    let expanded_tools_len = state.expanded_tools.len();

    let mut cache = state.chat_layout_cache.borrow_mut();
    let is_cache_valid = cache.width == text_width
        && cache.message_count == message_count
        && cache.last_content_len == last_content_len
        && cache.is_running == is_running
        && cache.expanded_tools_len == expanded_tools_len
        && !cache.lines.is_empty();

    if !is_cache_valid {
        let mut chat_lines = Vec::new();
        if state.messages.is_empty() {
            chat_lines.push(Line::from(vec![
                Span::styled("Welcome to ", Style::default().fg(Theme::TEXT_SECONDARY)),
                Span::styled(
                    "Ragasukma AI Agent",
                    Style::default()
                        .fg(Theme::GOLD_PRIMARY)
                        .add_modifier(Modifier::BOLD),
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
        }

        // Loading indicator at the bottom of agent conversation when agent is active
        if state.is_running {
            chat_lines.push(Line::from(vec![
                Span::styled("   ", Style::default().fg(Theme::GOLD_PRIMARY)),
                Span::styled(
                    "Agent is thinking & generating response...",
                    Style::default()
                        .fg(Theme::CYAN_ACCENT)
                        .add_modifier(Modifier::BOLD | Modifier::ITALIC),
                ),
            ]));
        }

        // Spacer padding of 1 line at the bottom so bottom text does not stick to border
        if !chat_lines.is_empty() {
            chat_lines.push(Line::from(""));
        }

        // Pre-wrap all lines to chat area width so total lines map 1:1 to visual screen lines
        cache.lines = wrap_lines_to_width(chat_lines, text_width);
        cache.width = text_width;
        cache.message_count = message_count;
        cache.last_content_len = last_content_len;
        cache.is_running = is_running;
        cache.expanded_tools_len = expanded_tools_len;
    }

    let total_visual_lines = cache.lines.len();

    let max_scroll = total_visual_lines.saturating_sub(inner_height);
    state.chat_max_scroll.set(max_scroll);
    let effective_scroll = if state.auto_scroll {
        max_scroll
    } else {
        max_scroll.saturating_sub(state.scroll_offset)
    }
    .min(max_scroll);

    let scrollbar_area = Rect {
        x: chat_inner.x + chat_inner.width.saturating_sub(1),
        y: chat_inner.y,
        width: 1.min(chat_inner.width),
        height: chat_inner.height,
    };

    let text_area = Rect {
        x: chat_inner.x,
        y: chat_inner.y,
        width: chat_inner.width.saturating_sub(1),
        height: chat_inner.height,
    };

    let p = Paragraph::new(cache.lines.clone()).scroll((effective_scroll as u16, 0));
    frame.render_widget(chat_block, chunks[0]);
    frame.render_widget(p, text_area);

    if max_scroll > 0 && scrollbar_area.width > 0 {
        let mut scrollbar_state =
            ScrollbarState::new(total_visual_lines).position(effective_scroll.min(max_scroll));
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
        Span::styled("  Tag File Mode (@) ", Theme::badge_cyan())
    } else if is_input_focused {
        Span::styled("  Message Input [Active] ", Theme::badge_gold())
    } else {
        Span::styled("  Message Input ", Theme::badge_gray())
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

    let prompt_width = 2usize; // "❯ "
    let available_width = (chunks[1].width as usize)
        .saturating_sub(2 + prompt_width)
        .max(5);
    state.input_wrap_width.set(available_width);

    let vlines = state.compute_visual_lines(available_width);
    let (cursor_row, cursor_col) = state.get_cursor_visual_position(available_width);

    let visible_lines: Vec<Line> = if state.input_buffer.is_empty() {
        let placeholder = if is_input_focused {
            Span::styled(
                "Type a prompt, or @ to attach file context...",
                Style::default()
                    .fg(Theme::TEXT_SECONDARY)
                    .add_modifier(Modifier::ITALIC),
            )
        } else {
            Span::styled(
                "Press [i] or [Enter] to write a message...",
                Style::default().fg(Theme::TEXT_SECONDARY),
            )
        };
        vec![Line::from(vec![prompt_prefix, placeholder])]
    } else {
        let start_line = state.input_scroll_line;
        let end_line = (start_line + 3).min(vlines.len());
        let mut rendered = Vec::new();

        for (line_idx, vl) in vlines.iter().enumerate().take(end_line).skip(start_line) {
            let line_prefix = if vl.is_paragraph_start {
                if line_idx == 0 {
                    prompt_prefix.clone()
                } else {
                    Span::styled("❯ ", Style::default().fg(Theme::TEXT_SECONDARY))
                }
            } else {
                Span::styled("  ", Style::default().fg(Theme::TEXT_SECONDARY))
            };

            let visible_slice = if vl.start_byte < vl.end_byte
                && vl.end_byte <= state.input_buffer.len()
                && state.input_buffer.is_char_boundary(vl.start_byte)
                && state.input_buffer.is_char_boundary(vl.end_byte)
            {
                &state.input_buffer[vl.start_byte..vl.end_byte]
            } else {
                ""
            };

            let mut spans = vec![line_prefix];
            let mut current_idx = 0;
            let buf = visible_slice;

            while let Some(at_idx) = buf[current_idx..].find('@') {
                let actual_at = current_idx + at_idx;
                if actual_at > current_idx {
                    spans.push(Span::styled(
                        buf[current_idx..actual_at].to_string(),
                        Style::default().fg(Theme::TEXT_PRIMARY),
                    ));
                }
                let rest = &buf[actual_at..];
                let end_idx = rest.find(|c: char| c.is_whitespace()).unwrap_or(rest.len());
                spans.push(Span::styled(
                    buf[actual_at..actual_at + end_idx].to_string(),
                    Style::default()
                        .fg(Theme::CYAN_ACCENT)
                        .add_modifier(Modifier::BOLD),
                ));
                current_idx = actual_at + end_idx;
            }

            if current_idx < buf.len() {
                spans.push(Span::styled(
                    buf[current_idx..].to_string(),
                    Style::default().fg(Theme::TEXT_PRIMARY),
                ));
            }

            rendered.push(Line::from(spans));
        }

        rendered
    };

    let input_block = Block::default()
        .title(input_title)
        .borders(Borders::ALL)
        .border_style(input_border)
        .style(Theme::block_base());

    let input_p = Paragraph::new(visible_lines).block(input_block);
    frame.render_widget(input_p, chunks[1]);

    // Position terminal cursor when input is focused for intuitive typing UX
    if is_input_focused {
        let visual_row = cursor_row.saturating_sub(state.input_scroll_line) as u16;
        let visual_col = cursor_col as u16;
        let cursor_x = (chunks[1].x + 1 + prompt_width as u16 + visual_col)
            .min(chunks[1].x + chunks[1].width.saturating_sub(2));
        let cursor_y =
            (chunks[1].y + 1 + visual_row).min(chunks[1].y + chunks[1].height.saturating_sub(2));
        frame.set_cursor_position((cursor_x, cursor_y));
    }
}
