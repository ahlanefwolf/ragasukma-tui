use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Row, Table, TableState},
};

use super::types::{McpFormField, McpServerTabState};
use crate::theme::Theme;

#[must_use]
pub fn get_footer_key_hints(state: &McpServerTabState) -> Vec<Span<'static>> {
    if state.delete_confirm_id.is_some() {
        vec![
            Span::styled(" [y/Enter] ", Theme::key_badge()),
            Span::raw("Confirm Delete "),
            Span::styled(" [n/Esc] ", Theme::key_badge()),
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
        vec![
            Span::styled(" [j/k] ", Theme::key_badge()),
            Span::raw("Navigate "),
            Span::styled(" [Space/Enter] ", Theme::key_badge()),
            Span::raw("Toggle Enabled "),
            Span::styled(" [a] ", Theme::key_badge()),
            Span::raw("Add "),
            Span::styled(" [e] ", Theme::key_badge()),
            Span::raw("Edit "),
            Span::styled(" [d] ", Theme::key_badge()),
            Span::raw("Delete "),
            Span::styled(" [r] ", Theme::key_badge()),
            Span::raw("Refresh "),
        ]
    }
}

pub fn render_mcp_server_tab(frame: &mut Frame, area: Rect, state: &McpServerTabState) {
    // Sisakan 1 baris di bawah sebagai clearance dari footer help, supaya tabel
    // dan modal tidak menempel/menutupi baris footer.
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),    // Server list
            Constraint::Length(1), // Clearance for footer
        ])
        .split(area);

    render_server_table(frame, layout[0], state);

    if state.is_form_open {
        render_form_modal(frame, layout[0], state);
    }

    if state.delete_confirm_id.is_some() {
        render_delete_modal(frame, layout[0], state);
    }
}

fn render_server_table(frame: &mut Frame, area: Rect, state: &McpServerTabState) {
    let block = Block::default()
        .title(" MCP Servers ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::BORDER_SUBTLE));

    if state.servers.is_empty() {
        let empty_msg = Paragraph::new("No MCP servers registered. Press [a] to add a new server.")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Theme::TEXT_MUTED))
            .block(block);
        frame.render_widget(empty_msg, area);
        return;
    }

    // Only 2 columns: Name and Status
    let header = Row::new(vec!["Server Name", "Status"])
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
            let status_span = if s.is_enabled != 0 {
                Span::styled(
                    "● Enabled",
                    Style::default()
                        .fg(Theme::STATUS_SUCCESS)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled("○ Disabled", Style::default().fg(Theme::TEXT_MUTED))
            };

            Row::new(vec![Line::from(s.name.as_str()), Line::from(status_span)])
        })
        .collect();

    let widths = [Constraint::Percentage(70), Constraint::Percentage(30)];

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
        .highlight_symbol("❯ ");

    frame.render_stateful_widget(table, area, &mut table_state);
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
            Constraint::Min(6),    // JSON textarea
            Constraint::Length(2), // Status (Aktif toggle)
            Constraint::Length(1), // Error
            Constraint::Length(3), // Buttons
        ])
        .split(inner_area);

    // 1. Name input
    let name_focused = state.focused_field == McpFormField::Name;
    let name_border_color = if name_focused {
        Theme::BORDER_GOLD
    } else {
        Theme::BORDER_SUBTLE
    };
    let name_widget = Paragraph::new(state.form_name.as_str()).block(
        Block::default()
            .title(" Server Name ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(name_border_color)),
    );
    frame.render_widget(name_widget, inner[0]);

    if name_focused {
        let name_col_offset = state.form_name[..state.name_cursor.min(state.form_name.len())]
            .chars()
            .count() as u16;
        let max_col = inner[0].width.saturating_sub(2);
        let cursor_x = inner[0].x + 1 + name_col_offset.min(max_col.saturating_sub(1));
        let cursor_y = inner[0].y + 1;
        frame.set_cursor_position((cursor_x, cursor_y));
    }

    // 2. JSON Config input
    let json_focused = state.focused_field == McpFormField::JsonConfig;
    let json_border_color = if json_focused {
        Theme::BORDER_GOLD
    } else {
        Theme::BORDER_SUBTLE
    };

    let visible_height = (inner[1].height.saturating_sub(2)) as usize;
    let (cursor_line, cursor_col) =
        super::state::get_line_and_col(&state.form_json, state.json_cursor);

    let scroll = if cursor_line < state.json_scroll {
        cursor_line
    } else if cursor_line >= state.json_scroll + visible_height && visible_height > 0 {
        cursor_line + 1 - visible_height
    } else {
        state.json_scroll
    };

    let all_lines: Vec<&str> = if state.form_json.is_empty() {
        vec![""]
    } else {
        state.form_json.split('\n').collect()
    };
    let display_lines: Vec<Line> = all_lines
        .into_iter()
        .skip(scroll)
        .take(visible_height)
        .map(Line::from)
        .collect();

    let json_widget = Paragraph::new(display_lines).block(
        Block::default()
            .title(" Configuration (JSON format) ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(json_border_color)),
    );
    frame.render_widget(json_widget, inner[1]);

    if json_focused {
        let cursor_y = inner[1].y + 1 + (cursor_line.saturating_sub(scroll)) as u16;
        let max_col = inner[1].width.saturating_sub(2);
        let cursor_x = inner[1].x + 1 + (cursor_col as u16).min(max_col.saturating_sub(1));
        frame.set_cursor_position((cursor_x, cursor_y));
    }

    // 3. Status (Enabled/Disabled toggle)
    let enabled_focused = state.focused_field == McpFormField::Enabled;
    let (box_char, status_text) = if state.form_enabled {
        ("[✔]", "Enabled")
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
            Paragraph::new(format!("⚠ {err}")).style(Style::default().fg(Theme::STATUS_ERROR));
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
