use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use super::state::AgentWorkspaceState;
use super::types::{AgentFocusArea, MessageRole, ToolExecutionStatus};
use crate::theme::Theme;

pub fn render_agent_tab(frame: &mut Frame, area: Rect, state: &AgentWorkspaceState) {
    let main_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(22),
            Constraint::Percentage(56),
            Constraint::Percentage(22),
        ])
        .split(area);

    render_left_sidebar(frame, main_layout[0], state);
    render_center_chat(frame, main_layout[1], state);
    render_right_inspector(frame, main_layout[2], state);
}

fn render_left_sidebar(frame: &mut Frame, area: Rect, state: &AgentWorkspaceState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Min(8)])
        .split(area);

    let is_sess_focused = state.focus_area == AgentFocusArea::Sessions;
    let is_files_focused = state.focus_area == AgentFocusArea::Files;

    let session_items: Vec<ListItem> = state
        .sessions
        .iter()
        .enumerate()
        .map(|(idx, s)| {
            let is_cursor = idx == state.selected_session_idx;
            let is_active = idx == state.active_session_idx;

            let icon = match (is_cursor, is_active) {
                (true, true) => "▶ ● ",
                (true, false) => "▶ ○ ",
                (false, true) => "  ● ",
                (false, false) => "  ○ ",
            };

            let title_style = if is_cursor {
                Style::default()
                    .fg(Theme::GOLD_PRIMARY)
                    .add_modifier(Modifier::BOLD)
            } else if is_active {
                Style::default().fg(Theme::TEXT_PRIMARY)
            } else {
                Style::default().fg(Theme::TEXT_SECONDARY)
            };

            let active_badge = if is_active {
                Span::styled(" [curr]", Style::default().fg(Theme::CYAN_ACCENT))
            } else {
                Span::raw("")
            };

            ListItem::new(Line::from(vec![
                Span::styled(icon, Style::default().fg(Theme::GOLD_PRIMARY)),
                Span::styled(&s.title, title_style),
                active_badge,
            ]))
        })
        .collect();

    let sess_border_style = if is_sess_focused {
        Style::default().fg(Theme::GOLD_PRIMARY)
    } else {
        Theme::border_subtle()
    };

    let sess_title = if is_sess_focused {
        Span::styled(" 🗂️ Sessions [FOCUS: ⏎:load, n:new, d:del] ", Theme::badge_gold())
    } else {
        Span::styled(" 🗂️ Sessions [s:focus] ", Theme::badge_gold())
    };

    let session_block = Block::default()
        .title(sess_title)
        .borders(Borders::ALL)
        .border_style(sess_border_style)
        .style(Theme::block_base());
    frame.render_widget(List::new(session_items).block(session_block), chunks[0]);

    let file_items: Vec<ListItem> = state
        .workspace_files
        .iter()
        .enumerate()
        .map(|(idx, f)| {
            let is_cursor = idx == state.selected_file_idx;
            let ext = f.rsplit('.').next().unwrap_or("");
            let file_icon = match ext {
                "rs" => "🦀 ",
                "toml" => "📦 ",
                "json" => "🏷️ ",
                "md" => "📝 ",
                _ => "📄 ",
            };

            let cursor_str = if is_cursor { "▶ " } else { "  " };

            let text_style = if is_cursor {
                Style::default()
                    .fg(Theme::CYAN_ACCENT)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Theme::TEXT_PRIMARY)
            };

            ListItem::new(Line::from(vec![
                Span::styled(cursor_str, Style::default().fg(Theme::CYAN_ACCENT)),
                Span::styled(file_icon, Style::default()),
                Span::styled(f, text_style),
            ]))
        })
        .collect();

    let files_border_style = if is_files_focused {
        Style::default().fg(Theme::CYAN_ACCENT)
    } else {
        Theme::border_subtle()
    };

    let files_title = if is_files_focused {
        Span::styled(" 📁 Files [FOCUS: ⏎:insert @file, r:scan] ", Theme::badge_cyan())
    } else {
        Span::styled(" 📁 Workspace Files [f:focus] ", Theme::header_title())
    };

    let files_block = Block::default()
        .title(files_title)
        .borders(Borders::ALL)
        .border_style(files_border_style)
        .style(Theme::block_base());
    frame.render_widget(List::new(file_items).block(files_block), chunks[1]);
}

fn render_center_chat(frame: &mut Frame, area: Rect, state: &AgentWorkspaceState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(10), Constraint::Length(4)])
        .split(area);

    let is_chat_focused = state.focus_area == AgentFocusArea::Chat;
    let chat_border_style = if is_chat_focused && !state.is_input_focused {
        Style::default().fg(Theme::CYAN_ACCENT)
    } else {
        Theme::border_subtle()
    };

    let chat_title = if is_chat_focused {
        Span::styled(
            format!(
                " 💬 Chat Stream [FOCUS] ({} msgs) [i:type, c:clear, x:tools] ",
                state.messages.len()
            ),
            Theme::header_title(),
        )
    } else {
        Span::styled(
            format!(
                " 💬 Chat Stream ({} msgs) ",
                state.messages.len()
            ),
            Theme::header_title(),
        )
    };

    let chat_block = Block::default()
        .title(chat_title)
        .borders(Borders::ALL)
        .border_style(chat_border_style)
        .style(Theme::block_base());

    let mut chat_lines: Vec<Line> = Vec::new();

    for (msg_idx, msg) in state.messages.iter().enumerate() {
        let (role_icon, role_name, role_style) = match msg.role {
            MessageRole::User => (
                " 👤 ",
                "You",
                Style::default()
                    .fg(Theme::GOLD_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
            MessageRole::Assistant => (
                " 🤖 ",
                msg.sender_name.as_str(),
                Style::default()
                    .fg(Theme::CYAN_ACCENT)
                    .add_modifier(Modifier::BOLD),
            ),
            MessageRole::System => (
                " ⚙️ ",
                "System",
                Style::default()
                    .fg(Theme::TEXT_MUTED)
                    .add_modifier(Modifier::ITALIC),
            ),
            MessageRole::Tool => (
                " 🛠️ ",
                "Tool Execution",
                Style::default()
                    .fg(Theme::STATUS_WARNING)
                    .add_modifier(Modifier::BOLD),
            ),
        };

        chat_lines.push(Line::from(vec![
            Span::styled(role_icon, Style::default()),
            Span::styled(role_name, role_style),
            Span::styled(
                format!("  {}", msg.timestamp),
                Style::default().fg(Theme::TEXT_MUTED),
            ),
        ]));

        if let Some(reasoning) = &msg.reasoning {
            chat_lines.push(Line::from(vec![
                Span::styled(
                    "  💭 Thought: ",
                    Style::default()
                        .fg(Theme::TEXT_MUTED)
                        .add_modifier(Modifier::ITALIC),
                ),
                Span::styled(reasoning, Style::default().fg(Theme::TEXT_MUTED)),
            ]));
        }

        for line in msg.content.lines() {
            chat_lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(line, Style::default().fg(Theme::TEXT_PRIMARY)),
            ]));
        }

        if !msg.tool_calls.is_empty() {
            let is_expanded = state.expanded_tool_idx == Some(msg_idx);
            for tool in &msg.tool_calls {
                let status_badge = match tool.status {
                    ToolExecutionStatus::Running => Span::styled(
                        " [RUNNING...] ",
                        Style::default().fg(Theme::STATUS_WARNING),
                    ),
                    ToolExecutionStatus::Success => {
                        Span::styled(" [SUCCESS] ", Theme::status_success())
                    }
                    ToolExecutionStatus::Failed => {
                        Span::styled(" [FAILED] ", Theme::status_danger())
                    }
                };
                let dur = tool
                    .duration_ms
                    .map_or(String::new(), |d| format!(" ({d}ms)"));
                let expand_hint = if is_expanded {
                    "[-]"
                } else {
                    "[+] [x to toggle]"
                };

                chat_lines.push(Line::from(vec![
                    Span::raw("    "),
                    status_badge,
                    Span::styled(
                        format!(" 🔧 {} ", tool.name),
                        Style::default()
                            .fg(Theme::CYAN_ACCENT)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(dur, Style::default().fg(Theme::TEXT_MUTED)),
                    Span::styled(
                        format!(" {expand_hint}"),
                        Style::default().fg(Theme::GOLD_FOREGROUND),
                    ),
                ]));

                if is_expanded {
                    chat_lines.push(Line::from(vec![
                        Span::raw("      "),
                        Span::styled("Args: ", Style::default().fg(Theme::TEXT_MUTED)),
                        Span::styled(&tool.args_summary, Style::default().fg(Theme::TEXT_SECONDARY)),
                    ]));
                    if let Some(out) = &tool.output_summary {
                        chat_lines.push(Line::from(vec![
                            Span::raw("      "),
                            Span::styled("Output: ", Style::default().fg(Theme::TEXT_MUTED)),
                            Span::styled(out, Style::default().fg(Theme::TEXT_PRIMARY)),
                        ]));
                    }
                }
            }
        }

        chat_lines.push(Line::raw(""));
    }

    let chat_widget = Paragraph::new(chat_lines)
        .block(chat_block)
        .wrap(Wrap { trim: false })
        .scroll((state.scroll_offset as u16, 0));
    frame.render_widget(chat_widget, chunks[0]);

    let input_border = if state.is_input_focused {
        Theme::border_focused()
    } else {
        Theme::border_subtle()
    };
    let title_prefix = if chunks[1].width < 45 {
        if state.is_input_focused {
            " ⌨️ [FOCUSED] "
        } else {
            " 💬 [Prompt] "
        }
    } else if state.is_input_focused {
        " ⌨️ [FOCUSED - Enter: Send, Esc: Unfocus] "
    } else {
        " 💬 [Press i or Enter to write prompt] "
    };

    let input_block = Block::default()
        .title(Span::styled(
            title_prefix,
            if state.is_input_focused {
                Theme::badge_gold()
            } else {
                Theme::header_title()
            },
        ))
        .borders(Borders::ALL)
        .border_style(input_border)
        .style(Theme::block_base());

    let prompt_display = if state.input_buffer.is_empty() && !state.is_input_focused {
        let placeholder = if chunks[1].width < 40 {
            "Type prompt... (i)"
        } else {
            "Type prompt for agent... (press i to focus)"
        };
        Line::from(vec![Span::styled(
            placeholder,
            Style::default().fg(Theme::TEXT_MUTED),
        )])
    } else {
        Line::from(vec![
            Span::styled("▶ ", Style::default().fg(Theme::GOLD_PRIMARY)),
            Span::styled(&state.input_buffer, Style::default().fg(Theme::TEXT_PRIMARY)),
            if state.is_input_focused {
                Span::styled("█", Style::default().fg(Theme::GOLD_PRIMARY))
            } else {
                Span::raw("")
            },
        ])
    };

    let input_widget = Paragraph::new(prompt_display)
        .block(input_block)
        .wrap(Wrap { trim: false });
    frame.render_widget(input_widget, chunks[1]);
}

fn render_right_inspector(frame: &mut Frame, area: Rect, state: &AgentWorkspaceState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(10), Constraint::Min(8)])
        .split(area);

    let block_model = Block::default()
        .title(Span::styled(" 🎯 Model Parameters ", Theme::badge_cyan()))
        .borders(Borders::ALL)
        .border_style(Theme::border_subtle())
        .style(Theme::block_base());

    let lines_model = vec![
        Line::from(vec![
            Span::styled("Provider: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                &state.active_provider,
                Style::default()
                    .fg(Theme::GOLD_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Model: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                &state.active_model,
                Style::default().fg(Theme::TEXT_PRIMARY),
            ),
        ]),
        Line::from(vec![
            Span::styled("Temp: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                format!("{:.1}", state.temperature),
                Style::default().fg(Theme::TEXT_SECONDARY),
            ),
            Span::styled("  Max: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                format!("{}", state.max_tokens),
                Style::default().fg(Theme::TEXT_SECONDARY),
            ),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("Prompt Tokens: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                format!("{}", state.total_prompt_tokens),
                Style::default().fg(Theme::CYAN_ACCENT),
            ),
        ]),
        Line::from(vec![
            Span::styled("Output Tokens: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                format!("{}", state.total_completion_tokens),
                Style::default().fg(Theme::CYAN_ACCENT),
            ),
        ]),
    ];
    frame.render_widget(Paragraph::new(lines_model).block(block_model), chunks[0]);

    let block_tools = Block::default()
        .title(Span::styled(" 🛠️ Active MCP Tools ", Theme::header_title()))
        .borders(Borders::ALL)
        .border_style(Theme::border_subtle())
        .style(Theme::block_base());

    let tools_lines = vec![
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
    ];
    frame.render_widget(Paragraph::new(tools_lines).block(block_tools), chunks[1]);
}
