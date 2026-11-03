use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use super::state::AgentWorkspaceState;
use super::types::{AgentFocusArea, MessageRole, ToolExecutionStatus};
use crate::theme::Theme;

pub fn render_agent_tab(frame: &mut Frame, area: Rect, state: &AgentWorkspaceState) {
    if !state.is_sidebar_visible && !state.is_inspector_visible {
        render_center_chat(frame, area, state);
    } else if !state.is_sidebar_visible {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(area);
        render_center_chat(frame, chunks[0], state);
        render_right_inspector(frame, chunks[1], state);
    } else if !state.is_inspector_visible {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
            .split(area);
        render_left_sidebar(frame, chunks[0], state);
        render_center_chat(frame, chunks[1], state);
    } else {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(22),
                Constraint::Percentage(56),
                Constraint::Percentage(22),
            ])
            .split(area);
        render_left_sidebar(frame, chunks[0], state);
        render_center_chat(frame, chunks[1], state);
        render_right_inspector(frame, chunks[2], state);
    }

    if let Some(ref tq) = state.tag_query {
        render_tag_query_popup(frame, area, tq);
    }

    if state.add_folder_modal.is_open {
        render_add_folder_modal(frame, area, &state.add_folder_modal);
    }

    if state.project_modal.is_open {
        render_project_switcher_modal(frame, area, state);
    }

    if state.create_project_modal.is_open {
        render_create_project_modal(frame, area, &state.create_project_modal);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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

fn render_project_switcher_modal(frame: &mut Frame, area: Rect, state: &AgentWorkspaceState) {
    let popup_area = centered_rect(60, 50, area);
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::border_focused())
        .title(Span::styled(" 🗂️ Database Projects ", Theme::badge_gold()));

    let items: Vec<ListItem> = state
        .projects
        .iter()
        .enumerate()
        .map(|(idx, proj)| {
            let is_sel = idx == state.project_modal.selected_idx;
            let is_active = state.active_project_id.as_deref() == Some(&proj.id);
            let prefix = if is_active { "★ " } else { "  " };
            let style = if is_sel {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let desc = proj.description.as_deref().unwrap_or("No description");
            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(prefix, Style::default().fg(Color::Green)),
                    Span::styled(&proj.name, style),
                ]),
                Line::from(vec![
                    Span::raw("    "),
                    Span::styled(desc, Style::default().fg(Color::DarkGray)),
                ]),
            ])
        })
        .collect();

    let list = List::new(items).block(block);
    frame.render_widget(list, popup_area);
}

fn render_create_project_modal(
    frame: &mut Frame,
    area: Rect,
    modal: &crate::agent_view::types::CreateProjectModalState,
) {
    let popup_area = centered_rect(50, 20, area);
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::border_focused())
        .title(Span::styled(" ➕ Create New Project ", Theme::badge_gold()));

    let content = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Project Name: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(&modal.name_input, Style::default().fg(Color::Yellow)),
            Span::styled("█", Style::default().fg(Color::White)),
        ]),
    ];

    let paragraph = Paragraph::new(content).block(block);
    frame.render_widget(paragraph, popup_area);
}

fn render_left_sidebar(frame: &mut Frame, area: Rect, state: &AgentWorkspaceState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Length(3),
            Constraint::Min(8),
        ])
        .split(area);

    let is_focused = state.focus_area == AgentFocusArea::Files;

    // 1. Header Workspace Folders Bar (1:1 with web agent multi-folder header)
    let folder_block = Block::default()
        .title(vec![
            Span::styled(" 📦 Project: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(&state.active_project_name, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(" "),
        ])
        .borders(Borders::ALL)
        .border_style(if is_focused {
            Theme::border_focused()
        } else {
            Theme::border_subtle()
        })
        .style(Theme::block_base());

    let mut folder_spans = Vec::new();
    for (idx, folder) in state.workspace_folders.iter().enumerate() {
        let is_active = idx == state.active_folder_idx;
        let name = folder
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| folder.to_string_lossy().to_string());

        if is_active {
            folder_spans.push(Span::styled(
                format!(" [{}] ", name),
                Style::default().fg(Theme::GOLD_PRIMARY).add_modifier(Modifier::BOLD),
            ));
        } else {
            folder_spans.push(Span::styled(
                format!(" {} ", name),
                Style::default().fg(Theme::TEXT_MUTED),
            ));
        }
    }
    if folder_spans.is_empty() {
        folder_spans.push(Span::styled(" (No attached workspaces) ", Style::default().fg(Theme::TEXT_MUTED)));
    }
    frame.render_widget(Paragraph::new(Line::from(folder_spans)).block(folder_block), chunks[0]);

    // 2. Sidebar Tab Switcher (1:1 with AgentSidebar.tsx: Tasks vs Files)
    let is_tasks_active = state.sidebar_tab == super::types::AgentSidebarTab::Tasks;
    let tab_spans = vec![
        Span::raw("  "),
        if is_tasks_active {
            Span::styled(
                format!(" 💬 Tasks ({}) ", state.sessions.len()),
                Theme::badge_gold(),
            )
        } else {
            Span::styled(
                format!(" Tasks ({}) ", state.sessions.len()),
                Style::default().fg(Theme::TEXT_MUTED),
            )
        },
        Span::raw("  |  "),
        if !is_tasks_active {
            Span::styled(
                format!(" 📁 Files ({}) ", state.workspace_files.len()),
                Theme::badge_gold(),
            )
        } else {
            Span::styled(
                format!(" Files ({}) ", state.workspace_files.len()),
                Style::default().fg(Theme::TEXT_MUTED),
            )
        },
    ];
    let tab_block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Theme::border_subtle())
        .style(Theme::block_base());
    frame.render_widget(Paragraph::new(Line::from(tab_spans)).block(tab_block), chunks[1]);

    // 3. Tab Content
    let border_style = if is_focused {
        Theme::border_focused()
    } else {
        Theme::border_subtle()
    };

    match state.sidebar_tab {
        super::types::AgentSidebarTab::Tasks => {
            let task_items: Vec<ListItem> = if state.sessions.is_empty() {
                vec![ListItem::new(Line::from(vec![Span::styled(
                    " (No tasks created yet) ",
                    Style::default().fg(Theme::TEXT_MUTED),
                )]))]
            } else {
                state
                    .sessions
                    .iter()
                    .enumerate()
                    .map(|(idx, s)| {
                        let is_cursor = idx == state.selected_session_idx && is_focused;
                        let is_current = state.active_session_id.as_deref() == Some(&s.id);
                        let cursor_str = if is_cursor { "▶ " } else { "  " };
                        let marker = if is_current { "● " } else { "○ " };
                        let marker_color = if s.is_running {
                            Theme::CYAN_ACCENT
                        } else if is_current {
                            Theme::GOLD_PRIMARY
                        } else {
                            Theme::TEXT_MUTED
                        };

                        let status_tag = if s.is_running { " [Running]" } else { "" };
                        let total_tokens = s.total_tokens_input + s.total_tokens_output;
                        let token_tag = if total_tokens > 0 {
                            format!(" ({} tokens)", total_tokens)
                        } else {
                            String::new()
                        };

                        let style = if is_cursor {
                            Style::default().fg(Theme::GOLD_PRIMARY).add_modifier(Modifier::BOLD)
                        } else if is_current {
                            Style::default().fg(Theme::TEXT_PRIMARY).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(Theme::TEXT_SECONDARY)
                        };

                        ListItem::new(Line::from(vec![
                            Span::styled(cursor_str, Style::default().fg(Theme::GOLD_PRIMARY)),
                            Span::styled(marker, Style::default().fg(marker_color)),
                            Span::styled(&s.title, style),
                            Span::styled(status_tag, Style::default().fg(Theme::CYAN_ACCENT)),
                            Span::styled(token_tag, Style::default().fg(Theme::TEXT_MUTED)),
                        ]))
                    })
                    .collect()
            };

            let title = if is_focused {
                Span::styled(
                    format!(" 💬 Active Tasks [{}] [Focused] ", state.sessions.len()),
                    Theme::badge_gold(),
                )
            } else {
                Span::styled(
                    format!(" 💬 Active Tasks [{}] ", state.sessions.len()),
                    Theme::header_title(),
                )
            };

            let block = Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(border_style)
                .style(Theme::block_base());
            frame.render_widget(List::new(task_items).block(block), chunks[2]);
        }
        super::types::AgentSidebarTab::Files => {
            let file_items: Vec<ListItem> = if state.workspace_files.is_empty() {
                vec![ListItem::new(Line::from(vec![Span::styled(
                    " (No files found in workspace) ",
                    Style::default().fg(Theme::TEXT_MUTED),
                )]))]
            } else {
                state
                    .workspace_files
                    .iter()
                    .enumerate()
                    .map(|(idx, f)| {
                        let is_cursor = idx == state.selected_file_idx && is_focused;
                        let indent = "  ".repeat(f.depth);
                        let icon = if f.is_dir {
                            if f.is_expanded { "📂 " } else { "📁 " }
                        } else {
                            let ext = f.name.rsplit('.').next().unwrap_or("");
                            match ext {
                                "rs" => "🦀 ",
                                "toml" => "📦 ",
                                "json" => "🏷️ ",
                                "md" => "📝 ",
                                _ => "📄 ",
                            }
                        };

                        let cursor_str = if is_cursor { "▶ " } else { "  " };
                        let text_style = if is_cursor {
                            Style::default().fg(Theme::GOLD_PRIMARY).add_modifier(Modifier::BOLD)
                        } else if f.is_dir && f.depth == 0 {
                            Style::default().fg(Theme::CYAN_ACCENT).add_modifier(Modifier::BOLD)
                        } else if f.is_dir {
                            Style::default().fg(Theme::TEXT_PRIMARY).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(Theme::TEXT_PRIMARY)
                        };

                        let display_name = if f.is_dir && !f.name.ends_with('/') {
                            format!("{}/", f.name)
                        } else {
                            f.name.clone()
                        };

                        ListItem::new(Line::from(vec![
                            Span::styled(cursor_str, Style::default().fg(Theme::GOLD_PRIMARY)),
                            Span::raw(indent),
                            Span::raw(icon),
                            Span::styled(display_name, text_style),
                        ]))
                    })
                    .collect()
            };

            let title = if is_focused {
                Span::styled(" 📁 Workspace Files [Focused] ", Theme::badge_gold())
            } else {
                Span::styled(" 📁 Workspace Files ", Theme::header_title())
            };

            let block = Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(border_style)
                .style(Theme::block_base());
            frame.render_widget(List::new(file_items).block(block), chunks[2]);
        }
    }
}

fn render_center_chat(frame: &mut Frame, area: Rect, state: &AgentWorkspaceState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(6), Constraint::Length(3)])
        .split(area);

    let is_chat_focused = state.focus_area == AgentFocusArea::Chat;

    let mut chat_lines = Vec::new();
    if state.messages.is_empty() {
        chat_lines.push(Line::from(vec![
            Span::styled("Welcome to ", Style::default().fg(Theme::TEXT_SECONDARY)),
            Span::styled("Ragasukma AI Agent", Style::default().fg(Theme::GOLD_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled("! Type your message below or press @ to attach files.", Style::default().fg(Theme::TEXT_SECONDARY)),
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
                    Span::styled(&tc.name, Style::default().fg(Theme::GOLD_PRIMARY).add_modifier(Modifier::BOLD)),
                    Span::raw("] "),
                    Span::styled(status_icon, Style::default().fg(status_color)),
                ]));
            }
            chat_lines.push(Line::from(""));
        }
    }

    let chat_block = Block::default()
        .title(Span::styled(" 💬 Agent Conversation ", Theme::header_title()))
        .borders(Borders::ALL)
        .border_style(if is_chat_focused && !state.is_input_focused {
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

    // Input Bar
    let input_border = if state.is_input_focused {
        Theme::border_focused()
    } else {
        Theme::border_subtle()
    };

    let input_title = if state.is_input_focused {
        Span::styled(" Message Input (Active) ", Theme::badge_gold())
    } else {
        Span::styled(" Message Input ", Theme::badge_gray())
    };

    let prompt_prefix = Span::styled("❯ ", Style::default().fg(Theme::GOLD_PRIMARY).add_modifier(Modifier::BOLD));
    let input_text = Span::styled(&state.input_buffer, Style::default().fg(Theme::TEXT_PRIMARY));

    let input_block = Block::default()
        .title(input_title)
        .borders(Borders::ALL)
        .border_style(input_border)
        .style(Theme::block_base());

    let input_p = Paragraph::new(Line::from(vec![prompt_prefix, input_text]))
        .block(input_block);
    frame.render_widget(input_p, chunks[1]);
}

fn render_right_inspector(frame: &mut Frame, area: Rect, state: &AgentWorkspaceState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(7), Constraint::Min(8)])
        .split(area);

    let is_focused = state.focus_area == AgentFocusArea::Inspector;

    let breakdown = state.calculate_token_breakdown();
    let usage_pct = (breakdown.usage_ratio() * 100.0) as usize;

    let model_block = Block::default()
        .title(Span::styled(" 🧠 Model Telemetry ", Theme::header_title()))
        .borders(Borders::ALL)
        .border_style(if is_focused {
            Theme::border_focused()
        } else {
            Theme::border_subtle()
        })
        .style(Theme::block_base());

    let lines_model = vec![
        Line::from(vec![
            Span::styled("Active Model : ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(&state.active_model, Style::default().fg(Theme::GOLD_PRIMARY).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("Token Usage  : ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                format!("{} / {} ({}%)", breakdown.total_used(), breakdown.max_context_limit, usage_pct),
                Style::default().fg(if breakdown.is_danger() {
                    Theme::STATUS_ERROR
                } else if breakdown.is_warning() {
                    Theme::STATUS_WARNING
                } else {
                    Theme::CYAN_ACCENT
                }),
            ),
        ]),
        Line::from(vec![
            Span::styled("Remaining    : ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                format!("{} tokens", breakdown.remaining_tokens()),
                Style::default().fg(Theme::TEXT_PRIMARY),
            ),
        ]),
    ];
    frame.render_widget(Paragraph::new(lines_model).block(model_block), chunks[0]);

    let block_tools = Block::default()
        .title(Span::styled(" 🛠️ MCP Tool Status ", Theme::header_title()))
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

fn render_tag_query_popup(frame: &mut Frame, area: Rect, tq: &super::types::FileTagQuery) {
    let popup_area = Rect {
        x: area.x + 10,
        y: area.y + area.height.saturating_sub(12),
        width: area.width.saturating_sub(20).min(50),
        height: 8.min(area.height),
    };

    frame.render_widget(Clear, popup_area);

    let items: Vec<ListItem> = tq
        .matched_files
        .iter()
        .enumerate()
        .map(|(idx, file)| {
            let is_sel = idx == tq.selected_idx;
            let prefix = if is_sel { "▶ " } else { "  " };
            let style = if is_sel {
                Style::default().fg(Theme::GOLD_PRIMARY).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Theme::TEXT_PRIMARY)
            };
            ListItem::new(Line::from(vec![
                Span::styled(prefix, Style::default().fg(Theme::GOLD_PRIMARY)),
                Span::styled(file, style),
            ]))
        })
        .collect();

    let block = Block::default()
        .title(Span::styled(" @ Attach File to Prompt ", Theme::badge_gold()))
        .borders(Borders::ALL)
        .border_style(Theme::border_focused())
        .style(Theme::block_surface());

    frame.render_widget(List::new(items).block(block), popup_area);
}

fn render_add_folder_modal(
    frame: &mut Frame,
    area: Rect,
    modal_state: &super::types::AddFolderModalState,
) {
    let width = 68.min(area.width.saturating_sub(4));
    let height = 7.min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let popup_area = Rect {
        x: area.x + x,
        y: area.y + y,
        width,
        height,
    };

    frame.render_widget(Clear, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(2),
        ])
        .margin(1)
        .split(popup_area);

    let block = Block::default()
        .title(Span::styled(
            " ➕ Add Folder to Workspace ",
            Theme::badge_gold(),
        ))
        .borders(Borders::ALL)
        .border_style(Theme::border_focused())
        .style(Theme::block_surface());
    frame.render_widget(block, popup_area);

    let input_block = Block::default()
        .title(Span::styled(
            " Folder Path (e.g., /path/to/project or ~/repo) ",
            Style::default().fg(Theme::GOLD_PRIMARY).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Theme::border_focused());
    let input_line = Line::from(vec![
        Span::styled("📂 ", Style::default().fg(Theme::GOLD_PRIMARY)),
        Span::styled(&modal_state.input_path, Style::default().fg(Theme::TEXT_PRIMARY)),
        Span::styled("█", Style::default().fg(Theme::GOLD_PRIMARY)),
    ]);
    frame.render_widget(Paragraph::new(input_line).block(input_block), chunks[0]);

    let feedback_line = if let Some((is_valid, ref msg)) = modal_state.validation_message {
        let style = if is_valid {
            Style::default().fg(Theme::STATUS_SUCCESS)
        } else {
            Style::default().fg(Theme::STATUS_ERROR)
        };
        Line::from(vec![Span::styled(format!("⚠️ {}", msg), style)])
    } else {
        Line::from(vec![Span::styled(
            "Enter path to attached workspace directory",
            Style::default().fg(Theme::TEXT_MUTED),
        )])
    };
    frame.render_widget(Paragraph::new(feedback_line), chunks[1]);
}
