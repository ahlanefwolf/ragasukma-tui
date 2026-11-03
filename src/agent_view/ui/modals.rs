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

pub fn render_tag_query_popup(frame: &mut Frame, area: Rect, tq: &FileTagQuery) {
    let popup_area = Rect {
        x: area.x + 10,
        y: area.y + area.height.saturating_sub(12),
        width: area.width.saturating_sub(20).min(50),
        height: 8.min(area.height),
    };

    frame.render_widget(Clear, popup_area);

    let inner_height = (popup_area.height.saturating_sub(2) as usize).max(1);
    let total = tq.matched_files.len();
    let sel = tq.selected_idx;
    let offset = if sel >= inner_height {
        sel.saturating_sub(inner_height - 1)
    } else {
        0
    };
    let end = (offset + inner_height).min(total);

    let items: Vec<ListItem> = tq
        .matched_files
        .iter()
        .enumerate()
        .skip(offset)
        .take(end.saturating_sub(offset))
        .map(|(idx, file)| {
            let is_sel = idx == sel;
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
        .borders(Borders::ALL)
        .border_style(Theme::border_focused())
        .title(Span::styled(" @ File Reference ", Theme::badge_cyan()));

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