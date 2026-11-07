use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Row, Table, TableState},
};

use super::types::{McpFormField, McpServerTabState};
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
    } else if state.is_tools_modal_open {
        vec![
            Span::styled(" [j/k] ", Theme::key_badge()),
            Span::raw("Navigate Tools "),
            Span::styled(" [Esc/q/t] ", Theme::key_badge()),
            Span::raw("Close "),
        ]
    } else if state.is_sync_modal_open {
        vec![
            Span::styled(" [j/k] ", Theme::key_badge()),
            Span::raw("Select Model "),
            Span::styled(" [Enter] ", Theme::key_badge()),
            Span::raw("Start Sync "),
            Span::styled(" [Esc/q] ", Theme::key_badge()),
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
            Span::styled(" [t] ", Theme::key_badge()),
            Span::raw("Tools "),
            Span::styled(" [s] ", Theme::key_badge()),
            Span::raw("Sync Tools "),
            Span::styled(" [d] ", Theme::key_badge()),
            Span::raw("Delete "),
            Span::styled(" [r] ", Theme::key_badge()),
            Span::raw("Refresh "),
            Span::styled(" [Ctrl+Q] ", Theme::key_badge()),
            Span::raw("Quit "),
        ]
    }
}

pub fn render_mcp_server_tab(frame: &mut Frame, area: Rect, state: &McpServerTabState) {
    // Render full-height so the card matches every other tab (no bottom gap).
    render_server_table(frame, area, state);

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

    render_text_input(frame, inner[0], &state.form_name, "Server Name", &Theme);
    render_text_input(frame, inner[1], &state.form_json, "Configuration", &Theme);

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
            .style(Style::default().fg(Theme::TEXT_PRIMARY));
        frame.render_widget(doc, chunks[1]);
    }
}

fn render_sync_modal(frame: &mut Frame, area: Rect, state: &McpServerTabState) {
    let popup_area = centered_rect(55, 45, area);
    frame.render_widget(Clear, popup_area);

    let server_name = state
        .selected_server()
        .map_or("Unknown", |s| s.name.as_str());

    let block = Block::default()
        .title(format!(" Sync Tools: {} ", server_name))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::GOLD_PRIMARY));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Description / instructions
            Constraint::Min(3),    // Model list
            Constraint::Length(2), // Status / Hint
        ])
        .split(inner);

    let intro = Paragraph::new("Select model to execute server tools & Markdown sync:")
        .style(Style::default().fg(Theme::TEXT_MUTED));
    frame.render_widget(intro, chunks[0]);

    let items: Vec<ListItem> = state
        .available_models
        .iter()
        .enumerate()
        .map(|(idx, model)| {
            let is_sel = idx == state.selected_model_idx;
            let style = if is_sel {
                Style::default()
                    .fg(Theme::GOLD_PRIMARY)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Theme::TEXT_PRIMARY)
            };
            let prefix = if is_sel { "▶ " } else { "  " };
            ListItem::new(format!("{}{}", prefix, model)).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Theme::BORDER_SUBTLE))
            .title(" Available Models "),
    );
    frame.render_widget(list, chunks[1]);

    let status_text = if state.is_syncing {
        "⏳ Syncing tools and generating Markdown documentation...".to_string()
    } else if let Some(ref msg) = state.sync_status_message {
        msg.clone()
    } else {
        "[Enter] Start Sync    [Esc/q] Cancel".to_string()
    };

    let status_paragraph = Paragraph::new(status_text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Theme::GOLD_PRIMARY));
    frame.render_widget(status_paragraph, chunks[2]);
}