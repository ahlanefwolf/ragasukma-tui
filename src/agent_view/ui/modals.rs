use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Wrap,
    },
    Frame,
};

use super::helpers::centered_rect;
use crate::agent_view::state::AgentWorkspaceState;
use crate::agent_view::types::{AddFolderModalState, CreateProjectModalState, FileTagQuery};
use crate::app::types::{DiffApprovalState, DiffLineType};
use crate::theme::Theme;

pub fn render_project_switcher_modal(frame: &mut Frame, area: Rect, state: &AgentWorkspaceState) {
    let popup_area = centered_rect(60, 50, area);
    frame.render_widget(Clear, popup_area);

    let total = state.projects.len();
    let sel = state.project_modal.selected_idx;
    let title = if total > 0 {
        format!(" 🗂️ Database Projects [{}/{}] ", sel + 1, total)
    } else {
        " 🗂️ Database Projects ".to_string()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::border_focused())
        .title(Span::styled(title, Theme::badge_gold()));

    let inner_height = (popup_area.height.saturating_sub(2) as usize).max(1);
    let offset = if sel >= inner_height {
        sel.saturating_sub(inner_height - 1)
    } else {
        0
    };
    let end = (offset + inner_height).min(total);

    let items: Vec<ListItem> = state
        .projects
        .iter()
        .enumerate()
        .skip(offset)
        .take(end.saturating_sub(offset))
        .map(|(idx, proj)| {
            let is_sel = idx == sel;
            let is_active = state.active_project_id.as_deref() == Some(&proj.id);
            let selector = if is_sel { "▶ " } else { "  " };
            let active_mark = if is_active { "★ " } else { "  " };
            let name_style = if is_sel {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else if is_active {
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let desc = proj.description.as_deref().unwrap_or("No description");
            let line = Line::from(vec![
                Span::styled(selector, Style::default().fg(Color::Yellow)),
                Span::styled(active_mark, Style::default().fg(Color::Green)),
                Span::styled(&proj.name, name_style),
                Span::raw(" "),
                Span::styled(format!("({desc})"), Style::default().fg(Color::DarkGray)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items).block(block);
    frame.render_widget(list, popup_area);

    if total > inner_height {
        let mut scrollbar_state = ScrollbarState::default()
            .content_length(total)
            .position(sel);
        frame.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("▲"))
                .end_symbol(Some("▼")),
            popup_area,
            &mut scrollbar_state,
        );
    }
}

pub fn render_create_project_modal(frame: &mut Frame, area: Rect, modal: &CreateProjectModalState) {
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

pub fn render_add_folder_modal(frame: &mut Frame, area: Rect, modal: &AddFolderModalState) {
    let popup_area = centered_rect(55, 22, area);
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::border_focused())
        .title(Span::styled(" 📁 Attach Workspace Folder ", Theme::badge_gold()));

    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Folder Path: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(&modal.input_path, Style::default().fg(Color::Yellow)),
            Span::styled("█", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
    ];

    if let Some((is_valid, ref msg)) = modal.validation_message {
        let style = if is_valid {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::Red)
        };
        lines.push(Line::from(Span::styled(format!("  {}", msg), style)));
    } else {
        lines.push(Line::from(Span::styled(
            "  Press Enter to validate & add folder, Esc to cancel",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let p = Paragraph::new(lines).block(block);
    frame.render_widget(p, popup_area);
}

pub fn render_tag_query_popup(frame: &mut Frame, anchor_area: Rect, tq: &FileTagQuery) {
    let loaded = tq.matched_items.len();
    let total = tq.all_matches.len();
    let visible_items = if loaded == 0 { 1 } else { loaded.min(7) as u16 };
    let popup_height = (visible_items + 3).min(anchor_area.y + anchor_area.height);
    let popup_width = (anchor_area.width.saturating_sub(4)).clamp(38, 72);

    let popup_y = anchor_area.y.saturating_sub(popup_height);
    let popup_x = anchor_area.x + 2;

    let popup_area = Rect {
        x: popup_x,
        y: popup_y,
        width: popup_width,
        height: popup_height,
    };

    frame.render_widget(Clear, popup_area);

    let inner_height = (popup_area.height.saturating_sub(2) as usize).max(1);
    let sel = tq.selected_idx;
    let offset = if sel >= inner_height {
        sel.saturating_sub(inner_height - 1)
    } else {
        0
    };
    let end = (offset + inner_height).min(loaded);

    let items: Vec<ListItem> = if total == 0 {
        vec![ListItem::new(Line::from(vec![
            Span::styled("  🔍 No items matching ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                format!("\"@{}\"", tq.query),
                Style::default().fg(Theme::GOLD_PRIMARY).add_modifier(Modifier::BOLD),
            ),
        ]))]
    } else {
        tq.matched_items
            .iter()
            .enumerate()
            .skip(offset)
            .take(end.saturating_sub(offset))
            .map(|(idx, item)| {
                let is_sel = idx == sel;
                let icon = if item.is_dir { "📁 " } else { "📄 " };

                let clean_p = item.path.trim_end_matches('/');
                let (dir_prefix, file_name) = match clean_p.rfind('/') {
                    Some(slash_pos) => (&clean_p[..=slash_pos], &clean_p[slash_pos + 1..]),
                    None => ("", clean_p),
                };

                let prefix = if is_sel { " ▸ " } else { "   " };
                let item_style = if is_sel {
                    Style::default().bg(Theme::BG_SURFACE_ELEVATED)
                } else {
                    Style::default()
                };

                let name_style = if is_sel {
                    Style::default()
                        .fg(Theme::GOLD_PRIMARY)
                        .add_modifier(Modifier::BOLD)
                } else if item.is_dir {
                    Style::default()
                        .fg(Theme::CYAN_ACCENT)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Theme::TEXT_PRIMARY)
                };

                let dir_style = Style::default().fg(Theme::TEXT_MUTED);

                ListItem::new(Line::from(vec![
                    Span::styled(
                        prefix,
                        if is_sel {
                            Style::default().fg(Theme::GOLD_PRIMARY).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default()
                        },
                    ),
                    Span::raw(icon),
                    Span::styled(file_name, name_style),
                    if item.is_dir {
                        Span::styled("/", Style::default().fg(Theme::CYAN_ACCENT))
                    } else {
                        Span::raw("")
                    },
                    Span::styled(format!("  ({})", dir_prefix), dir_style),
                ]))
                .style(item_style)
            })
            .collect()
    };

    let count_badge = if tq.has_more() {
        Span::styled(
            format!("[{}/{} • Scroll ↓ for more] ", loaded, total),
            Style::default().fg(Theme::CYAN_ACCENT),
        )
    } else {
        Span::styled(
            format!("[{} found] ", total),
            Style::default().fg(Theme::TEXT_SECONDARY),
        )
    };

    let title_line = Line::from(vec![
        Span::styled(" 🏷️ Tag Item ", Theme::badge_cyan()),
        if !tq.query.is_empty() {
            Span::styled(format!("(@{}) ", tq.query), Theme::badge_gold())
        } else {
            Span::raw("")
        },
        count_badge,
    ]);

    let bottom_line = Line::from(vec![
        Span::styled(" [↑/↓] ", Theme::key_badge()),
        Span::raw("Scroll "),
        Span::styled(" [Tab/Enter] ", Theme::key_badge()),
        Span::raw("Tag "),
        Span::styled(" [Esc] ", Theme::key_badge()),
        Span::raw("Close "),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::CYAN_ACCENT))
        .title(title_line)
        .title_bottom(bottom_line)
        .style(Theme::block_base());

    let list = List::new(items).block(block);
    frame.render_widget(list, popup_area);
}

pub fn render_diff_preview_modal(frame: &mut Frame, diff: &DiffApprovalState, area: Rect) {
    let popup_area = centered_rect(80, 80, area);
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::border_focused())
        .title(Span::styled(
            format!(" 📝 Diff: {} (Tool: {}) ", diff.file_path, diff.tool_name),
            Theme::badge_gold(),
        ));

    let mut lines = Vec::new();
    for diff_line in &diff.diff_lines {
        let style = match diff_line.line_type {
            DiffLineType::Added => Style::default().fg(Color::Green),
            DiffLineType::Removed => Style::default().fg(Color::Red),
            DiffLineType::Header => Style::default().fg(Color::Cyan),
            DiffLineType::Context => Style::default().fg(Color::Gray),
        };
        lines.push(Line::from(Span::styled(&diff_line.content, style)));
    }

    let p = Paragraph::new(lines).block(block).wrap(Wrap { trim: false });
    frame.render_widget(p, popup_area);
}

pub fn render_model_selector_modal(frame: &mut Frame, state: &AgentWorkspaceState, area: Rect) {
    let popup_area = centered_rect(60, 50, area);
    frame.render_widget(Clear, popup_area);

    let total = state.available_models.len();
    let sel = state.model_modal.selected_idx;
    let title = if total > 0 {
        format!(" 🏷️ Select Model [{}/{}] ", sel + 1, total)
    } else {
        " 🏷️ Select Model ".to_string()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::border_focused())
        .title(Span::styled(title, Theme::header_title()))
        .title_bottom(Line::from(vec![
            Span::styled(" [↑/↓/←/→] ", Style::default().fg(Theme::GOLD_PRIMARY)),
            Span::styled("Navigate  ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("[Enter] ", Style::default().fg(Theme::GOLD_PRIMARY)),
            Span::styled("Select  ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled("[Esc] ", Style::default().fg(Theme::GOLD_PRIMARY)),
            Span::styled("Cancel", Style::default().fg(Theme::TEXT_MUTED)),
        ]))
        .style(Theme::block_base());

    let inner_height = (popup_area.height.saturating_sub(2) as usize).max(1);
    let offset = if sel >= inner_height {
        sel.saturating_sub(inner_height - 1)
    } else {
        0
    };
    let end = (offset + inner_height).min(total);

    let mut items: Vec<ListItem> = Vec::new();

    if total == 0 {
        items.push(ListItem::new(Line::from(vec![
            Span::styled(" [No models available] ", Style::default().fg(Theme::TEXT_MUTED)),
        ])));
    } else {
        for (idx, m) in state.available_models[offset..end].iter().enumerate() {
            let actual_idx = offset + idx;
            let is_selected = actual_idx == sel;
            let is_active = m == &state.active_model;

            let line = if is_selected {
                Line::from(vec![
                    Span::styled(" ❯ ", Style::default().fg(Theme::GOLD_PRIMARY).add_modifier(Modifier::BOLD)),
                    Span::styled(format!("[◈ {}]", m), Style::default().fg(Theme::GOLD_PRIMARY).add_modifier(Modifier::BOLD)),
                    if is_active {
                        Span::styled(" (current)", Style::default().fg(Theme::CYAN_ACCENT))
                    } else {
                        Span::raw("")
                    },
                ])
            } else if is_active {
                Line::from(vec![
                    Span::raw("   "),
                    Span::styled(format!("[✓ {}]", m), Style::default().fg(Theme::CYAN_ACCENT)),
                    Span::styled(" (current)", Style::default().fg(Theme::CYAN_ACCENT)),
                ])
            } else {
                Line::from(vec![
                    Span::raw("   "),
                    Span::styled(format!("[○ {}]", m), Style::default().fg(Theme::TEXT_SECONDARY)),
                ])
            };

            items.push(ListItem::new(line));
        }
    }

    let list = List::new(items).block(block);
    frame.render_widget(list, popup_area);

    if total > inner_height {
        let mut scrollbar_state = ScrollbarState::default()
            .content_length(total)
            .position(sel);
        frame.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("▲"))
                .end_symbol(Some("▼")),
            popup_area,
            &mut scrollbar_state,
        );
    }
}