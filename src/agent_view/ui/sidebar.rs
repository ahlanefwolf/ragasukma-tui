use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
    },
    Frame,
};

use crate::agent_view::state::AgentWorkspaceState;
use crate::agent_view::types::{AgentFocusArea, AgentSidebarTab};
use crate::theme::Theme;

pub fn render_left_sidebar(frame: &mut Frame, area: Rect, state: &AgentWorkspaceState) {
    let is_folders_focused = state.focus_area == AgentFocusArea::Folders;

    // 1. Header Workspace Folders Bar (Card 1: Folders)
    let folder_title = if is_folders_focused {
        vec![
            Span::styled(" 📦 Project [Focused]: ", Theme::badge_gold()),
            Span::styled(
                &state.active_project_name,
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
        ]
    } else {
        vec![
            Span::styled(" 📦 Project: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(
                &state.active_project_name,
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
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
                Style::default().fg(Theme::GOLD_PRIMARY).add_modifier(Modifier::BOLD)
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
    let folder_card_height = ((folder_lines_count + 2) as u16).min(max_folder_height).max(3);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(folder_card_height),
            Constraint::Length(3),
            Constraint::Min(8),
        ])
        .split(area);

    frame.render_widget(Paragraph::new(folder_lines).block(folder_block), chunks[0]);

    // 2. Sidebar Tab Switcher
    let is_tasks_active = state.sidebar_tab == AgentSidebarTab::Tasks;
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
                Style::default().fg(Theme::TEXT_SECONDARY),
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
                Style::default().fg(Theme::TEXT_SECONDARY),
            )
        },
    ];
    let tab_block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Theme::border_subtle())
        .style(Theme::block_base());
    frame.render_widget(Paragraph::new(Line::from(tab_spans)).block(tab_block), chunks[1]);

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
                        let cursor_str = if is_cursor { "▶ " } else { "  " };
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
                            format!(" [{} tokens]", super::helpers::format_compact_number(total_tokens as usize))
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
                            Span::styled(status_tag, status_style),
                            Span::styled(token_tag, Style::default().fg(Theme::TEXT_SECONDARY)),
                        ]))
                    })
                    .collect()
            };

            let title_str = if total > 0 {
                format!(" 💬 Active Tasks [{}/{}] ", sel + 1, total)
            } else {
                " 💬 Active Tasks [0] ".to_string()
            };

            let title = if is_tasks_focused {
                Span::styled(format!("{} [Focused] ", title_str.trim_end()), Theme::badge_gold())
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
                            if f.is_expanded { "📂 " } else { "📁 " }
                        } else {
                            "📄 "
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

            let title_str = if total > 0 {
                format!(" 📁 Workspace Files [{}/{}] ", sel + 1, total)
            } else {
                " 📁 Workspace Files [0] ".to_string()
            };

            let title = if is_files_focused {
                Span::styled(format!("{} [Focused] ", title_str.trim_end()), Theme::badge_gold())
            } else {
                Span::styled(title_str, Theme::header_title())
            };

            let block = Block::default()
                .title(title)
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
    }
}
