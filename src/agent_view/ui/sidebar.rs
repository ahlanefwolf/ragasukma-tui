use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
    },
};

use crate::agent_view::states::AgentWorkspaceState;
use crate::agent_view::types::{AgentFocusArea, AgentSidebarTab};
use crate::theme::Theme;

pub fn render_left_sidebar(frame: &mut Frame, area: Rect, state: &AgentWorkspaceState) {
    let is_folders_focused = state.focus_area == AgentFocusArea::Folders;

    // 1. Header Workspace Folders Bar (Card 1: Folders)
    let folder_title = if is_folders_focused {
        vec![
            Span::styled("  Project [Focused]: ", Theme::badge_gold()),
            Span::styled(
                &state.active_project_name,
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
        ]
    } else {
        vec![
            Span::styled(
                "  Project: ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                &state.active_project_name,
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
        ]
    };

    let folder_block = Block::default()
        .title(folder_title)
        .borders(Borders::ALL)
        .border_style(if is_folders_focused {
            Theme::border_focused()
        } else {
            Theme::border_subtle()
        })
        .style(Theme::block_base());

    let inner_width = area.width.saturating_sub(2).max(10) as usize;
    let mut folder_lines: Vec<Line> = Vec::new();
    let mut current_line_spans: Vec<Span> = Vec::new();
    let mut current_line_width: usize = 0;

    if state.workspace_folders.is_empty() {
        folder_lines.push(Line::from(vec![Span::styled(
            " (No attached workspaces) ",
            Style::default().fg(Theme::TEXT_SECONDARY),
        )]));
    } else {
        for (idx, folder) in state.workspace_folders.iter().enumerate() {
            let is_active = idx == state.active_folder_idx;
            let name = folder
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| folder.to_string_lossy().to_string());

            let text = if is_active {
                format!(" [{}] ", name)
            } else {
                format!(" {} ", name)
            };
            let style = if is_active {
                Style::default()
                    .fg(Theme::GOLD_PRIMARY)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Theme::TEXT_SECONDARY)
            };

            let tag_len = text.chars().count();
            if current_line_width > 0 && current_line_width + tag_len > inner_width {
                folder_lines.push(Line::from(std::mem::take(&mut current_line_spans)));
                current_line_width = 0;
            }

            current_line_width += tag_len;
            current_line_spans.push(Span::styled(text, style));
        }

        if !current_line_spans.is_empty() {
            folder_lines.push(Line::from(current_line_spans));
        }
    }

    let folder_lines_count = folder_lines.len().max(1);
    let max_folder_height = (area.height / 3).clamp(3, 8);
    let folder_card_height = ((folder_lines_count + 2) as u16)
        .min(max_folder_height)
        .max(3);

    // Bottom-anchored Model Telemetry card. On short terminals we shrink the
    // tab content first; the telemetry card keeps a sensible minimum.
    let telemetry_height = super::inspector::MODEL_TELEMETRY_HEIGHT
        .min(area.height.saturating_sub(folder_card_height + 1 + 5))
        .max(3);
    let show_telemetry = area.height >= telemetry_height + folder_card_height + 1 + 5;

    let mut v_constraints = vec![
        Constraint::Length(folder_card_height),
        Constraint::Length(1),
        Constraint::Min(5),
    ];
    if show_telemetry {
        v_constraints.push(Constraint::Length(telemetry_height));
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(v_constraints)
        .split(area);

    frame.render_widget(Paragraph::new(folder_lines).block(folder_block), chunks[0]);

    // 2. Sidebar Tab Switcher
    let is_tasks_active = state.sidebar_tab == AgentSidebarTab::Tasks;
    let is_files_active = state.sidebar_tab == AgentSidebarTab::Files;
    let is_mcp_active = state.sidebar_tab == AgentSidebarTab::Mcp;
    let mcp_online = state.mcp_active_ids.len();
    let mut tab_spans = vec![
        if is_tasks_active {
            Span::styled(
                format!("  Tasks ({}) ", state.sessions.len()),
                Theme::badge_gold(),
            )
        } else {
            Span::styled(
                format!(" Tasks ({}) ", state.sessions.len()),
                Style::default().fg(Theme::TEXT_SECONDARY),
            )
        },
        Span::raw("|"),
        if is_files_active {
            Span::styled(
                format!("  Files ({}) ", state.workspace_files.len()),
                Theme::badge_gold(),
            )
        } else {
            Span::styled(
                format!(" Files ({}) ", state.workspace_files.len()),
                Style::default().fg(Theme::TEXT_SECONDARY),
            )
        },
        Span::raw("|"),
        if is_mcp_active {
            Span::styled(
                format!(" MCP ({}/{}) ", mcp_online, state.mcp_servers.len()),
                Theme::badge_gold(),
            )
        } else {
            Span::styled(
                format!(" MCP ({}/{}) ", mcp_online, state.mcp_servers.len()),
                Style::default().fg(Theme::TEXT_SECONDARY),
            )
        },
    ];

    // Center the tab group horizontally within the sidebar width.
    let tab_width: usize = tab_spans.iter().map(|s| s.content.chars().count()).sum();
    let avail_width = chunks[1].width as usize;
    let pad_left = avail_width.saturating_sub(tab_width) / 2;
    if pad_left > 0 {
        tab_spans.insert(0, Span::raw(" ".repeat(pad_left)));
    }

    frame.render_widget(Paragraph::new(Line::from(tab_spans)), chunks[1]);

    // 3. Tab Content (Card 2: Tasks or Files)
    match state.sidebar_tab {
        AgentSidebarTab::Tasks => {
            let is_tasks_focused = state.focus_area == AgentFocusArea::Tasks;
            let border_style = if is_tasks_focused {
                Theme::border_focused()
            } else {
                Theme::border_subtle()
            };

            let total = state.sessions.len();
            let sel = state.selected_session_idx;
            let inner_height = (chunks[2].height.saturating_sub(2) as usize).max(1);
            let offset = if sel >= inner_height {
                sel.saturating_sub(inner_height - 1)
            } else {
                0
            };
            let end = (offset + inner_height).min(total);

            let task_items: Vec<ListItem> = if state.sessions.is_empty() {
                vec![ListItem::new(Line::from(vec![Span::styled(
                    " (No tasks created yet) ",
                    Style::default().fg(Theme::TEXT_SECONDARY),
                )]))]
            } else {
                state
                    .sessions
                    .iter()
                    .enumerate()
                    .skip(offset)
                    .take(end.saturating_sub(offset))
                    .map(|(idx, s)| {
                        let is_cursor = idx == sel && is_tasks_focused;
                        let is_current = state.active_session_id.as_deref() == Some(&s.id);
                        let cursor_str = if is_cursor { "❯ " } else { "  " };
                        let (marker, marker_color) = if s.is_running {
                            (format!("{} ", state.spinner_char()), Theme::CYAN_ACCENT)
                        } else if is_current {
                            ("● ".to_string(), Theme::GOLD_PRIMARY)
                        } else {
                            ("○ ".to_string(), Theme::TEXT_SECONDARY)
                        };

                        let (status_tag, status_style) = if s.is_running {
                            (
                                " [Running]",
                                Style::default()
                                    .fg(Theme::CYAN_ACCENT)
                                    .add_modifier(Modifier::BOLD),
                            )
                        } else {
                            (" [Idle]", Style::default().fg(Theme::TEXT_MUTED))
                        };

                        let total_tokens = s.total_tokens_input + s.total_tokens_output;
                        let token_tag = if total_tokens > 0 {
                            format!(
                                " [{} tokens]",
                                super::helpers::format_compact_number(total_tokens as usize)
                            )
                        } else {
                            String::new()
                        };

                        let style = if is_cursor {
                            Style::default()
                                .fg(Theme::GOLD_PRIMARY)
                                .add_modifier(Modifier::BOLD)
                        } else if is_current {
                            Style::default()
                                .fg(Theme::TEXT_PRIMARY)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(Theme::TEXT_SECONDARY)
                        };

                        ListItem::new(Line::from(vec![
                            Span::styled(cursor_str, Style::default().fg(Theme::GOLD_PRIMARY)),
                            Span::styled(marker, Style::default().fg(marker_color)),
                            Span::styled(&s.title, style),
                            Span::styled(status_tag, status_style),
                            Span::styled(token_tag, Style::default().fg(Theme::TEXT_SECONDARY)),
                        ]))
                    })
                    .collect()
            };

            let title_str = if total > 0 {
                format!("  Active Tasks [{}/{}] ", sel + 1, total)
            } else {
                "  Active Tasks [0] ".to_string()
            };

            let title = if is_tasks_focused {
                Span::styled(
                    format!("{} [Focused] ", title_str.trim_end()),
                    Theme::badge_gold(),
                )
            } else {
                Span::styled(title_str, Theme::header_title())
            };

            let block = Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(border_style)
                .style(Theme::block_base());
            frame.render_widget(List::new(task_items).block(block), chunks[2]);

            if total > inner_height {
                let mut scrollbar_state = ScrollbarState::default()
                    .content_length(total)
                    .position(sel);
                frame.render_stateful_widget(
                    Scrollbar::default()
                        .orientation(ScrollbarOrientation::VerticalRight)
                        .begin_symbol(Some("▲"))
                        .end_symbol(Some("▼")),
                    chunks[2],
                    &mut scrollbar_state,
                );
            }
        }
        AgentSidebarTab::Files => {
            let is_files_focused = state.focus_area == AgentFocusArea::Files;
            let border_style = if is_files_focused {
                Theme::border_focused()
            } else {
                Theme::border_subtle()
            };

            let total = state.workspace_files.len();
            let sel = state.selected_file_idx;
            let inner_height = (chunks[2].height.saturating_sub(2) as usize).max(1);
            let offset = if sel >= inner_height {
                sel.saturating_sub(inner_height - 1)
            } else {
                0
            };
            let end = (offset + inner_height).min(total);

            let file_items: Vec<ListItem> = if state.workspace_files.is_empty() {
                vec![ListItem::new(Line::from(vec![Span::styled(
                    " (No files found in workspace) ",
                    Style::default().fg(Theme::TEXT_SECONDARY),
                )]))]
            } else {
                state
                    .workspace_files
                    .iter()
                    .enumerate()
                    .skip(offset)
                    .take(end.saturating_sub(offset))
                    .map(|(idx, f)| {
                        let is_cursor = idx == sel && is_files_focused;
                        let indent = "  ".repeat(f.depth);
                        let icon = if f.is_dir {
                            if f.is_expanded { "▾ " } else { "▸ " }
                        } else {
                            "  "
                        };

                        let cursor_str = if is_cursor { "❯ " } else { "  " };
                        let text_style = if is_cursor {
                            Style::default()
                                .fg(Theme::GOLD_PRIMARY)
                                .add_modifier(Modifier::BOLD)
                        } else if f.is_dir && f.depth == 0 {
                            Style::default()
                                .fg(Theme::CYAN_ACCENT)
                                .add_modifier(Modifier::BOLD)
                        } else if f.is_dir {
                            Style::default()
                                .fg(Theme::TEXT_PRIMARY)
                                .add_modifier(Modifier::BOLD)
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

            let title_str = if total > 0 {
                format!("  Workspace Files [{}/{}] ", sel + 1, total)
            } else {
                "  Workspace Files [0] ".to_string()
            };

            let title = if is_files_focused {
                Span::styled(
                    format!("{} [Focused] ", title_str.trim_end()),
                    Theme::badge_gold(),
                )
            } else {
                Span::styled(title_str, Theme::header_title())
            };

            let title_bottom = if chunks[2].width >= 46 {
                Line::from(vec![
                    Span::styled(" [", Style::default().fg(Color::DarkGray)),
                    Span::styled(".skills/", Style::default().fg(Color::Cyan)),
                    Span::styled(" or ", Style::default().fg(Color::DarkGray)),
                    Span::styled(".skill/", Style::default().fg(Color::Cyan)),
                    Span::styled(" auto-injected to prompt] ", Style::default().fg(Color::DarkGray)),
                ])
            } else if chunks[2].width >= 32 {
                Line::from(vec![
                    Span::styled(" [", Style::default().fg(Color::DarkGray)),
                    Span::styled(".skills/", Style::default().fg(Color::Cyan)),
                    Span::styled(" auto-injected to prompt] ", Style::default().fg(Color::DarkGray)),
                ])
            } else {
                Line::from(vec![
                    Span::styled(" [", Style::default().fg(Color::DarkGray)),
                    Span::styled(".skills/ -> prompt", Style::default().fg(Color::Cyan)),
                    Span::styled("] ", Style::default().fg(Color::DarkGray)),
                ])
            };

            let block = Block::default()
                .title(title)
                .title_bottom(title_bottom)
                .borders(Borders::ALL)
                .border_style(border_style)
                .style(Theme::block_base());
            frame.render_widget(List::new(file_items).block(block), chunks[2]);

            if total > inner_height {
                let mut scrollbar_state = ScrollbarState::default()
                    .content_length(total)
                    .position(sel);
                frame.render_stateful_widget(
                    Scrollbar::default()
                        .orientation(ScrollbarOrientation::VerticalRight)
                        .begin_symbol(Some("▲"))
                        .end_symbol(Some("▼")),
                    chunks[2],
                    &mut scrollbar_state,
                );
            }
        }
        AgentSidebarTab::Mcp => {
            let is_mcp_focused = state.focus_area == AgentFocusArea::McpTools;
            let border_style = if is_mcp_focused {
                Theme::border_focused()
            } else {
                Theme::border_subtle()
            };

            let total = state.mcp_servers.len();
            let sel = if total > 0 {
                state.selected_mcp_idx.min(total - 1)
            } else {
                0
            };
            let inner_height = (chunks[2].height.saturating_sub(2) as usize).max(1);
            let offset = if sel >= inner_height {
                sel.saturating_sub(inner_height - 1)
            } else {
                0
            };
            let end = (offset + inner_height).min(total);

            let mcp_items: Vec<ListItem> = if state.mcp_servers.is_empty() {
                vec![
                    ListItem::new(Line::from(vec![Span::styled(
                        " (no MCP servers configured) ",
                        Style::default().fg(Theme::TEXT_SECONDARY),
                    )])),
                    ListItem::new(Line::from(vec![Span::styled(
                        "  add servers in MCP view ",
                        Style::default().fg(Theme::TEXT_SECONDARY),
                    )])),
                ]
            } else {
                state
                    .mcp_servers
                    .iter()
                    .enumerate()
                    .skip(offset)
                    .take(end.saturating_sub(offset))
                    .map(|(idx, server)| {
                        let enabled = state.mcp_active_ids.iter().any(|id| id == &server.id);
                        let is_cursor = idx == sel && is_mcp_focused;
                        let cursor_str = if is_cursor { "❯ " } else { "  " };
                        let marker = if enabled {
                            Span::styled("● ON  ", Theme::status_success())
                        } else {
                            Span::styled("○ OFF ", Style::default().fg(Theme::TEXT_SECONDARY))
                        };
                        let name_style = if is_cursor {
                            Style::default()
                                .fg(Theme::GOLD_PRIMARY)
                                .add_modifier(Modifier::BOLD)
                        } else if enabled {
                            Style::default().fg(Theme::TEXT_PRIMARY)
                        } else {
                            Style::default().fg(Theme::TEXT_SECONDARY)
                        };
                        ListItem::new(Line::from(vec![
                            Span::styled(cursor_str, Style::default().fg(Theme::GOLD_PRIMARY)),
                            marker,
                            Span::styled(server.name.clone(), name_style),
                        ]))
                    })
                    .collect()
            };

            let title_str = format!(" MCP Servers [{}/{}] ", mcp_online, total);
            let title = if is_mcp_focused {
                Span::styled(
                    format!("{} [Focused] ", title_str.trim_end()),
                    Theme::badge_gold(),
                )
            } else {
                Span::styled(title_str, Theme::header_title())
            };

            let block = Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(border_style)
                .style(Theme::block_base());
            frame.render_widget(List::new(mcp_items).block(block), chunks[2]);

            if total > inner_height {
                let mut scrollbar_state = ScrollbarState::default()
                    .content_length(total)
                    .position(sel);
                frame.render_stateful_widget(
                    Scrollbar::default()
                        .orientation(ScrollbarOrientation::VerticalRight)
                        .begin_symbol(Some("▲"))
                        .end_symbol(Some("▼")),
                    chunks[2],
                    &mut scrollbar_state,
                );
            }
        }
    }

    // 5. Model Telemetry card pinned at the bottom of the sidebar.
    if show_telemetry && let Some(telemetry_area) = chunks.get(3) {
        super::inspector::render_model_telemetry(frame, *telemetry_area, state);
    }
}
