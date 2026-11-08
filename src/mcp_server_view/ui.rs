use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Row, Table, TableState, Wrap},
};

use super::types::{McpFormField, McpPane, McpServerTabState};
use crate::theme::Theme;
use crate::ui::render_text_input;

#[must_use]
pub fn get_footer_key_hints(state: &McpServerTabState) -> Vec<Span<'static>> {
    if state.delete_confirm_id.is_some() {
        vec![
            Span::styled(" [y/Enter] ", Theme::key_badge()),
            Span::raw("Confirm Delete "),
            Span::styled(" [n/Esc] ", Theme::key_badge()),
            Span::raw("Cancel "),
        ]
    } else if state.is_tool_detail_open {
        vec![
            Span::styled(" [↑/↓/j/k] ", Theme::key_badge()),
            Span::raw("Scroll Spec "),
            Span::styled(" [Esc/q/Enter] ", Theme::key_badge()),
            Span::raw("Close Detail "),
        ]
    } else if state.is_tools_modal_open {
        vec![
            Span::styled(" [j/k] ", Theme::key_badge()),
            Span::raw("Navigate Tools "),
            Span::styled(" [Enter/d] ", Theme::key_badge()),
            Span::raw("View Spec "),
            Span::styled(" [Esc/q/t] ", Theme::key_badge()),
            Span::raw("Close "),
        ]
    } else if state.is_sync_modal_open {
        vec![
            Span::styled(" [Type] ", Theme::key_badge()),
            Span::raw("Filter "),
            Span::styled(" [↑/↓/Tab] ", Theme::key_badge()),
            Span::raw("Navigate "),
            Span::styled(" [Enter] ", Theme::key_badge()),
            Span::raw("Start Sync "),
            Span::styled(" [Esc] ", Theme::key_badge()),
            Span::raw("Cancel "),
        ]
    } else if state.is_form_open {
        vec![
            Span::styled(" [Tab] ", Theme::key_badge()),
            Span::raw("Next Field "),
            Span::styled(" [Enter] ", Theme::key_badge()),
            Span::raw("Submit "),
            Span::styled(" [Esc] ", Theme::key_badge()),
            Span::raw("Cancel "),
        ]
    } else {
        match state.active_pane {
            McpPane::Servers => vec![
                Span::styled(" [Tab/→] ", Theme::key_badge()),
                Span::raw("Tools Pane "),
                Span::styled(" [j/k] ", Theme::key_badge()),
                Span::raw("Navigate "),
                Span::styled(" [Space] ", Theme::key_badge()),
                Span::raw("Toggle Enabled "),
                Span::styled(" [a] ", Theme::key_badge()),
                Span::raw("Add "),
                Span::styled(" [e] ", Theme::key_badge()),
                Span::raw("Edit "),
                Span::styled(" [s] ", Theme::key_badge()),
                Span::raw("Sync Tools "),
                Span::styled(" [d] ", Theme::key_badge()),
                Span::raw("Delete "),
                Span::styled(" [r] ", Theme::key_badge()),
                Span::raw("Refresh "),
            ],
            McpPane::Tools => vec![
                Span::styled(" [Tab/Esc/←] ", Theme::key_badge()),
                Span::raw("Servers Pane "),
                Span::styled(" [j/k] ", Theme::key_badge()),
                Span::raw("Select Tool "),
                Span::styled(" [Enter/d] ", Theme::key_badge()),
                Span::raw("Tool Detail/Config "),
                Span::styled(" [s] ", Theme::key_badge()),
                Span::raw("Sync Tools "),
                Span::styled(" [r] ", Theme::key_badge()),
                Span::raw("Refresh "),
            ],
        }
    }
}

pub fn render_mcp_server_tab(frame: &mut Frame, area: Rect, state: &McpServerTabState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
        .split(area);

    render_servers_card(frame, chunks[0], state);
    render_tools_card(frame, chunks[1], state);

    if state.is_tool_detail_open {
        render_tool_detail_modal(frame, area, state);
    }

    if state.is_tools_modal_open {
        render_tools_modal(frame, area, state);
    }

    if state.is_sync_modal_open {
        render_sync_modal(frame, area, state);
    }

    if state.is_form_open {
        render_form_modal(frame, area, state);
    }

    if state.delete_confirm_id.is_some() {
        render_delete_modal(frame, area, state);
    }
}

fn render_servers_card(frame: &mut Frame, area: Rect, state: &McpServerTabState) {
    let is_active = state.active_pane == McpPane::Servers;
    let border_color = if is_active {
        Theme::GOLD_PRIMARY
    } else {
        Theme::BORDER_SUBTLE
    };

    let title = format!(
        " MCP Servers ({}) {} ",
        state.servers.len(),
        if is_active { "●" } else { "" }
    );

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    if state.servers.is_empty() {
        let empty_msg = Paragraph::new("No MCP servers registered.\nPress [a] to add a new server.")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Theme::TEXT_MUTED))
            .block(block);
        frame.render_widget(empty_msg, area);
        return;
    }

    let header = Row::new(vec!["Server", "Type", "Status"])
        .style(
            Style::default()
                .fg(Theme::GOLD_PRIMARY)
                .add_modifier(Modifier::BOLD),
        )
        .bottom_margin(1);

    let rows: Vec<Row> = state
        .servers
        .iter()
        .map(|s| {
            let transport = if s.config.contains("transport: sse")
                || s.config.contains("transport: http")
                || s.config.contains("url:")
            {
                "http/sse"
            } else {
                "stdio"
            };

            let status_span = if s.is_enabled != 0 {
                Span::styled(
                    "● Active",
                    Style::default()
                        .fg(Theme::STATUS_SUCCESS)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled("○ Off", Style::default().fg(Theme::TEXT_MUTED))
            };

            Row::new(vec![
                Line::from(s.name.as_str()),
                Line::from(Span::styled(
                    transport,
                    Style::default().fg(Theme::CYAN_ACCENT),
                )),
                Line::from(status_span),
            ])
        })
        .collect();

    let widths = [
        Constraint::Percentage(50),
        Constraint::Percentage(25),
        Constraint::Percentage(25),
    ];

    let mut table_state = TableState::default();
    table_state.select(Some(state.selected_idx));

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(
            Style::default()
                .fg(Theme::GOLD_PRIMARY)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(if is_active { "❯ " } else { "  " });

    frame.render_stateful_widget(table, area, &mut table_state);
}

fn render_tools_card(frame: &mut Frame, area: Rect, state: &McpServerTabState) {
    let is_active = state.active_pane == McpPane::Tools;
    let border_color = if is_active {
        Theme::GOLD_PRIMARY
    } else {
        Theme::BORDER_SUBTLE
    };

    let server_name = state
        .selected_server()
        .map_or("No Server", |s| s.name.as_str());

    let title = format!(
        " Tools: {} ({} tools) {} ",
        server_name,
        state.server_tools.len(),
        if is_active { "●" } else { "" }
    );

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    if state.server_tools.is_empty() {
        let msg = if state.selected_server().is_none() {
            "Select an MCP server from the left pane to view its tools."
        } else {
            "No tools registered for this server yet.\n\nPress [s] to sync/discover tools from the server."
        };
        let empty_p = Paragraph::new(msg)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Theme::TEXT_MUTED))
            .block(block);
        frame.render_widget(empty_p, area);
        return;
    }

    let items: Vec<ListItem> = state
        .server_tools
        .iter()
        .enumerate()
        .map(|(i, tool)| {
            let is_selected = i == state.tools_selected_idx;
            let marker = if is_selected && is_active {
                "❯ "
            } else if is_selected {
                "• "
            } else {
                "  "
            };

            // Extract brief description if available
            let brief_desc = extract_tool_description(&tool.config);
            let max_w = (area.width as usize).saturating_sub(8).max(30);
            let display_desc = if brief_desc.chars().count() > max_w {
                let truncated: String = brief_desc.chars().take(max_w.saturating_sub(3)).collect();
                format!("{}...", truncated)
            } else {
                brief_desc
            };

            let header_style = if is_selected {
                Style::default()
                    .fg(Theme::GOLD_PRIMARY)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Theme::TEXT_PRIMARY)
            };

            let lines = vec![
                Line::from(vec![
                    Span::styled(marker, Style::default().fg(Theme::GOLD_PRIMARY)),
                    Span::styled(tool.name.as_str(), header_style),
                ]),
                Line::from(vec![
                    Span::raw("    "),
                    Span::styled(
                        display_desc,
                        Style::default().fg(Theme::TEXT_MUTED),
                    ),
                ]),
            ];

            ListItem::new(lines)
        })
        .collect();

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}

fn extract_tool_description(config: &str) -> String {
    for line in config.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("description:") {
            return trimmed
                .trim_start_matches("description:")
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .trim()
                .to_string();
        }
        if trimmed.starts_with("- description:") {
            return trimmed
                .trim_start_matches("- description:")
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .trim()
                .to_string();
        }
        if !trimmed.is_empty()
            && !trimmed.starts_with('#')
            && !trimmed.starts_with("```")
            && !trimmed.starts_with("name:")
            && !trimmed.starts_with("tools:")
        {
            let desc = trimmed.trim_start_matches("- ").trim();
            return desc.to_string();
        }
    }
    "No description available".to_string()
}

fn render_tool_detail_modal(frame: &mut Frame, area: Rect, state: &McpServerTabState) {
    let popup_area = centered_rect(85, 85, area);
    frame.render_widget(Clear, popup_area);

    let selected_tool = match state.selected_tool() {
        Some(t) => t,
        None => return,
    };

    let server_name = state
        .selected_server()
        .map_or("Unknown Server", |s| s.name.as_str());

    let title = format!(
        " Tool Detail & Spec: {} ({}) ([Esc/q/Enter] Close, [↑/↓/j/k] Scroll) ",
        selected_tool.name, server_name
    );

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::GOLD_PRIMARY));

    let content_area = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let lines: Vec<Line> = if selected_tool.config.trim().is_empty() {
        vec![Line::from(Span::styled(
            "No specification details available for this tool.",
            Style::default().fg(Theme::TEXT_MUTED),
        ))]
    } else {
        selected_tool
            .config
            .lines()
            .map(|line| {
                let l = line.trim();
                let style = if l.starts_with('#') {
                    Style::default()
                        .fg(Theme::GOLD_PRIMARY)
                        .add_modifier(Modifier::BOLD)
                } else if l.starts_with("- ") || l.starts_with("* ") {
                    Style::default().fg(Theme::CYAN_ACCENT)
                } else if l.starts_with("```") {
                    Style::default().fg(Theme::TEXT_MUTED)
                } else {
                    Style::default().fg(Theme::TEXT_PRIMARY)
                };
                Line::from(Span::styled(line.to_string(), style))
            })
            .collect()
    };

    let paragraph = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((state.tool_detail_scroll as u16, 0));
    frame.render_widget(paragraph, content_area);
}

fn render_form_modal(frame: &mut Frame, area: Rect, state: &McpServerTabState) {
    let modal_area = centered_rect(65, 75, area);
    frame.render_widget(Clear, modal_area);

    let title = if state.editing_id.is_some() {
        " Edit MCP Server "
    } else {
        " Add MCP Server "
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::BORDER_GOLD));
    let inner_area = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Name input
            Constraint::Min(6),    // Markdown textarea
            Constraint::Length(2), // Status (Aktif toggle)
            Constraint::Length(1), // Error
            Constraint::Length(3), // Buttons
        ])
        .split(inner_area);

    render_text_input(frame, inner[0], &state.form_name, "Server Name", &Theme);
    render_text_input(frame, inner[1], &state.form_json, "Configuration (Markdown)", &Theme);

    // 3. Status (Enabled/Disabled toggle)
    let enabled_focused = state.focused_field == McpFormField::Enabled;
    let (box_char, status_text) = if state.form_enabled {
        ("[✓]", "Enabled")
    } else {
        ("[ ]", "Disabled")
    };
    let status_style = if enabled_focused {
        Style::default()
            .fg(Theme::GOLD_PRIMARY)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Theme::TEXT_PRIMARY)
    };
    let status_line = Line::from(vec![
        Span::styled(format!("{box_char} Status: {status_text} "), status_style),
        Span::styled(
            "(Press Space to toggle)",
            Style::default().fg(Theme::TEXT_MUTED),
        ),
    ]);
    frame.render_widget(Paragraph::new(status_line), inner[2]);

    // 4. Error line
    if let Some(err) = &state.form_error {
        let err_widget =
            Paragraph::new(format!("! {err}")).style(Style::default().fg(Theme::STATUS_ERROR));
        frame.render_widget(err_widget, inner[3]);
    }

    // 5. Buttons
    let btn_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner[4]);

    let save_focused = state.focused_field == McpFormField::SaveBtn;
    let save_style = if save_focused {
        Style::default()
            .fg(Theme::GOLD_PRIMARY)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Theme::TEXT_MUTED)
    };
    let save_btn = Paragraph::new("[ Save & Apply (Enter) ]")
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(if save_focused {
                    Style::default().fg(Theme::GOLD_PRIMARY)
                } else {
                    Style::default().fg(Theme::BORDER_SUBTLE)
                }),
        )
        .style(save_style);
    frame.render_widget(save_btn, btn_chunks[0]);

    let cancel_focused = state.focused_field == McpFormField::CancelBtn;
    let cancel_style = if cancel_focused {
        Style::default()
            .fg(Theme::GOLD_PRIMARY)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Theme::TEXT_MUTED)
    };
    let cancel_btn = Paragraph::new("[ Cancel (Esc) ]")
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(if cancel_focused {
                    Style::default().fg(Theme::GOLD_PRIMARY)
                } else {
                    Style::default().fg(Theme::BORDER_SUBTLE)
                }),
        )
        .style(cancel_style);
    frame.render_widget(cancel_btn, btn_chunks[1]);
}

fn render_delete_modal(frame: &mut Frame, area: Rect, state: &McpServerTabState) {
    let modal_area = centered_rect(50, 20, area);
    frame.render_widget(Clear, modal_area);

    let server_name = state
        .servers
        .iter()
        .find(|s| Some(&s.id) == state.delete_confirm_id.as_ref())
        .map(|s| s.name.as_str())
        .unwrap_or("server");

    let block = Block::default()
        .title(" Delete Confirmation ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::STATUS_ERROR));

    let content = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("Are you sure you want to delete MCP server '"),
            Span::styled(
                server_name,
                Style::default()
                    .fg(Theme::GOLD_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("' ?"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" [y/Enter] ", Theme::key_badge()),
            Span::raw("Confirm Delete    "),
            Span::styled(" [n/Esc] ", Theme::key_badge()),
            Span::raw("Cancel"),
        ]),
    ];

    let p = Paragraph::new(content)
        .alignment(Alignment::Center)
        .block(block);
    frame.render_widget(p, modal_area);
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

fn render_tools_modal(frame: &mut Frame, area: Rect, state: &McpServerTabState) {
    let popup_area = centered_rect(75, 75, area);
    frame.render_widget(Clear, popup_area);

    let server_name = state
        .selected_server()
        .map_or("Unknown", |s| s.name.as_str());
    let title = format!(" Tools for Server: {} ([Esc/q] Close) ", server_name);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::GOLD_PRIMARY));

    if state.server_tools.is_empty() {
        let msg = Paragraph::new("No tools registered or synced for this server yet.\nRun sync or add tools to mcp_server_tools.")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Theme::TEXT_MUTED))
            .block(block);
        frame.render_widget(msg, popup_area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(block.inner(popup_area));

    frame.render_widget(block, popup_area);

    let items: Vec<ListItem> = state
        .server_tools
        .iter()
        .enumerate()
        .map(|(idx, tool)| {
            let is_sel = idx == state.tools_selected_idx;
            let style = if is_sel {
                Style::default()
                    .fg(Theme::GOLD_PRIMARY)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Theme::TEXT_PRIMARY)
            };
            let prefix = if is_sel { "> " } else { "  " };
            ListItem::new(format!("{prefix}{}", tool.name)).style(style)
        })
        .collect();

    let list_block = Block::default()
        .title(" Tools List ")
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(Theme::BORDER_SUBTLE));
    let list = List::new(items).block(list_block);
    frame.render_widget(list, chunks[0]);

    if let Some(selected_tool) = state.server_tools.get(state.tools_selected_idx) {
        let doc = Paragraph::new(selected_tool.config.as_str())
            .block(
                Block::default()
                    .title(format!(" Spec: {} ", selected_tool.name))
                    .borders(Borders::NONE),
            )
            .style(Style::default().fg(Theme::TEXT_PRIMARY))
            .wrap(Wrap { trim: false });
        frame.render_widget(doc, chunks[1]);
    }
}

fn render_sync_modal(frame: &mut Frame, area: Rect, state: &McpServerTabState) {
    let popup_area = centered_rect(65, 55, area);
    frame.render_widget(Clear, popup_area);

    let server_name = state
        .selected_server()
        .map_or("Unknown", |s| s.name.as_str());

    let filtered_models = state.filtered_sync_models();
    let total = filtered_models.len();
    let sel = state.selected_model_idx;

    let block = Block::default()
        .title(Span::styled(
            format!("  Sync Tools: {} ", server_name),
            Theme::header_title(),
        ))
        .borders(Borders::ALL)
        .border_style(Theme::border_focused())
        .title_bottom(Line::from(vec![
            Span::styled(" [Type] ", Style::default().fg(Theme::GOLD_PRIMARY)),
            Span::styled("Filter  ", Style::default().fg(Theme::TEXT_SECONDARY)),
            Span::styled("[↑/↓/Tab] ", Style::default().fg(Theme::GOLD_PRIMARY)),
            Span::styled("Navigate  ", Style::default().fg(Theme::TEXT_SECONDARY)),
            Span::styled("[Enter] ", Style::default().fg(Theme::GOLD_PRIMARY)),
            Span::styled("Start Sync  ", Style::default().fg(Theme::TEXT_SECONDARY)),
            Span::styled("[Esc] ", Style::default().fg(Theme::GOLD_PRIMARY)),
            Span::styled("Cancel ", Style::default().fg(Theme::TEXT_SECONDARY)),
        ]))
        .style(Theme::block_base());

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Search filter text input
            Constraint::Min(4),    // Model list
            Constraint::Length(2), // Status / feedback
        ])
        .split(inner);

    render_text_input(
        frame,
        chunks[0],
        &state.sync_search_input,
        "Filter Models",
        &Theme,
    );

    let visible_height = chunks[1].height.saturating_sub(2) as usize;
    let (start, end) = if visible_height == 0 || total <= visible_height {
        (0, total)
    } else {
        let half = visible_height / 2;
        let start = if sel < half {
            0
        } else if sel + (visible_height - half) >= total {
            total.saturating_sub(visible_height)
        } else {
            sel.saturating_sub(half)
        };
        (start, (start + visible_height).min(total))
    };

    let items: Vec<ListItem> = filtered_models[start..end]
        .iter()
        .enumerate()
        .map(|(rel_idx, m)| {
            let abs_idx = start + rel_idx;
            let is_sel = abs_idx == sel;

            let line = if is_sel {
                Line::from(vec![
                    Span::styled(
                        " ❯ ",
                        Style::default()
                            .fg(Theme::GOLD_PRIMARY)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("[◈ {}]", m),
                        Style::default()
                            .fg(Theme::GOLD_PRIMARY)
                            .add_modifier(Modifier::BOLD),
                    ),
                ])
            } else {
                Line::from(vec![
                    Span::raw("   "),
                    Span::styled(m.as_str(), Style::default().fg(Theme::TEXT_PRIMARY)),
                ])
            };
            ListItem::new(line)
        })
        .collect();

    let counter = format!(" Available Models ({total}) ");
    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::border_subtle())
        .title(Span::styled(counter, Theme::header_title()));

    let list = List::new(items).block(list_block);
    frame.render_widget(list, chunks[1]);

    let status_text = if state.is_syncing {
        "Syncing tools and generating Markdown documentation...".to_string()
    } else if let Some(ref msg) = state.sync_status_message {
        msg.clone()
    } else {
        "Press Enter to run sync with the selected model".to_string()
    };

    let status_paragraph = Paragraph::new(status_text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Theme::GOLD_PRIMARY));
    frame.render_widget(status_paragraph, chunks[2]);
}