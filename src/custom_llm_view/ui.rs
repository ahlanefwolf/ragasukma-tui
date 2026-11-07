use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Wrap},
};

use crate::custom_llm_view::types::{CustomLlmFocus, CustomLlmTabState, FormField};
use crate::theme::Theme;
use crate::ui::render_text_input;
pub fn render_custom_llm_tab(frame: &mut Frame, area: Rect, state: &CustomLlmTabState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(52), Constraint::Percentage(48)])
        .split(area);

    render_providers_card(frame, chunks[0], state);
    render_models_card(frame, chunks[1], state);

    if state.is_form_open {
        render_form_modal(frame, area, state);
    }

    if state.delete_confirm_prefix.is_some() {
        render_delete_modal(frame, area, state);
    }
}

fn render_providers_card(frame: &mut Frame, area: Rect, state: &CustomLlmTabState) {
    let is_focused = state.focus == CustomLlmFocus::Providers;

    let title_spans = if is_focused {
        vec![
            Span::styled(
                format!("  Custom LLM Providers ({}) ", state.accounts.len()),
                Theme::badge_gold(),
            ),
            Span::styled(
                " [● Active Card - Tab to switch] ",
                Theme::text_gold().add_modifier(Modifier::BOLD),
            ),
        ]
    } else {
        vec![
            Span::styled(
                format!("  Custom LLM Providers ({}) ", state.accounts.len()),
                Theme::badge_gray(),
            ),
            Span::styled(" [Tab to focus] ", Theme::text_secondary()),
        ]
    };

    let border_style = if is_focused {
        Theme::border_focused()
    } else {
        Theme::border_subtle()
    };

    let header_cells = ["Prefix", "Provider Name", "Base URL Endpoint", "Status"]
        .iter()
        .map(|h| Cell::from(*h).style(Theme::header_title().add_modifier(Modifier::BOLD)));
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows: Vec<Row> = state
        .accounts
        .iter()
        .enumerate()
        .map(|(idx, acc)| {
            let is_selected = idx == state.selected_idx;

            let prefix_cell = Cell::from(Span::styled(
                format!(" /{}", acc.prefix),
                if is_selected {
                    Theme::text_cyan().add_modifier(Modifier::BOLD)
                } else {
                    Theme::text_secondary()
                },
            ));

            let name_cell = Cell::from(Span::styled(
                &acc.name,
                if is_selected {
                    Theme::text_gold().add_modifier(Modifier::BOLD)
                } else {
                    Theme::text_secondary()
                },
            ));

            let url_cell = Cell::from(Span::styled(
                &acc.base_url,
                if is_selected {
                    Theme::text_primary()
                } else {
                    Theme::text_secondary()
                },
            ));

            let status_span = if acc.enabled {
                Span::styled("● ON", Theme::text_gold().add_modifier(Modifier::BOLD))
            } else {
                Span::styled("○ OFF", Theme::text_secondary())
            };
            let status_cell = Cell::from(status_span);

            let row_style = ratatui::style::Style::default();

            Row::new(vec![prefix_cell, name_cell, url_cell, status_cell])
                .style(row_style)
                .height(1)
        })
        .collect();

    let widths = [
        Constraint::Length(12),
        Constraint::Length(16),
        Constraint::Min(16),
        Constraint::Length(8),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(Line::from(title_spans))
        .style(Theme::block_base());

    let table = Table::new(rows, widths).header(header).block(block);

    frame.render_widget(table, area);
}

fn render_models_card(frame: &mut Frame, area: Rect, state: &CustomLlmTabState) {
    let is_focused = state.focus == CustomLlmFocus::Models;

    let selected_acc = state.selected_account();
    let prefix_models = selected_acc
        .map(|acc| state.models_for_prefix(&acc.prefix))
        .unwrap_or_default();

    let title_prefix = selected_acc
        .map(|a| {
            format!(
                "  Models for /{prefix} ({count}) ",
                prefix = a.prefix,
                count = prefix_models.len()
            )
        })
        .unwrap_or_else(|| "  Models Catalog ".to_string());

    let title_spans = if is_focused {
        vec![
            Span::styled(title_prefix, Theme::badge_gold()),
            Span::styled(
                " [● Active Card - Tab to switch] ",
                Theme::text_gold().add_modifier(Modifier::BOLD),
            ),
        ]
    } else {
        vec![
            Span::styled(title_prefix, Theme::badge_gray()),
            Span::styled(" [Tab to focus] ", Theme::text_secondary()),
        ]
    };

    let border_style = if is_focused {
        Theme::border_focused()
    } else {
        Theme::border_subtle()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(Line::from(title_spans))
        .style(Theme::block_base());

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    if selected_acc.is_none() {
        let para = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No custom provider selected.",
                Theme::text_secondary(),
            )),
            Line::from(Span::styled(
                "  Press [a] to add a custom endpoint.",
                Theme::text_secondary(),
            )),
        ]);
        frame.render_widget(para, inner_area);
        return;
    }

    if prefix_models.is_empty() {
        let para = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No cached models found for this provider prefix.",
                Theme::text_secondary(),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  Press [r] to fetch remote models from /models endpoint.",
                Theme::text_cyan(),
            )),
        ]);
        frame.render_widget(para, inner_area);
        return;
    }

    let visible_height = inner_area.height as usize;
    let total_models = prefix_models.len();
    let selected_model = state.selected_model_idx.min(total_models.saturating_sub(1));

    let scroll_offset = if total_models <= visible_height {
        0
    } else if selected_model >= visible_height {
        selected_model + 1 - visible_height
    } else {
        0
    };

    let lines: Vec<Line> = prefix_models
        .iter()
        .enumerate()
        .skip(scroll_offset)
        .take(visible_height)
        .map(|(idx, (model_id, _))| {
            let is_selected = idx == selected_model;
            if is_selected && is_focused {
                Line::from(vec![
                    Span::styled(" ▶ ", Theme::text_gold().add_modifier(Modifier::BOLD)),
                    Span::styled(
                        model_id.clone(),
                        Theme::text_gold().add_modifier(Modifier::BOLD),
                    ),
                ])
            } else if is_selected {
                Line::from(vec![
                    Span::styled(" ▷ ", Theme::text_cyan().add_modifier(Modifier::BOLD)),
                    Span::styled(model_id.clone(), Theme::text_gold()),
                ])
            } else {
                Line::from(vec![
                    Span::styled("   ", Theme::text_secondary()),
                    Span::styled(model_id.clone(), Theme::text_secondary()),
                ])
            }
        })
        .collect();

    let para = Paragraph::new(lines);
    frame.render_widget(para, inner_area);
}

fn render_form_modal(frame: &mut Frame, area: Rect, state: &CustomLlmTabState) {
    let popup_area = centered_rect(70, 85, area);
    frame.render_widget(Clear, popup_area);

    let is_edit = state.editing_prefix.is_some();
    let title = if is_edit {
        "  Edit Custom Provider "
    } else {
        "  Add Custom LLM Provider "
    };

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::border_focused())
        .title(Span::styled(
            title,
            Theme::badge_gold().add_modifier(Modifier::BOLD),
        ))
        .style(Theme::block_base());
    let inner_area = outer_block.inner(popup_area);
    frame.render_widget(outer_block, popup_area);
    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // 0: Top Spacer (margin from modal header)
            Constraint::Length(3), // 1: Prefix
            Constraint::Length(1), // 2: Spacer
            Constraint::Length(3), // 3: Name
            Constraint::Length(1), // 4: Spacer
            Constraint::Length(3), // 5: Base URL
            Constraint::Length(1), // 6: Spacer
            Constraint::Length(3), // 7: API Key
            Constraint::Length(1), // 8: Spacer
            Constraint::Length(2), // 9: Enabled Checkbox
            Constraint::Length(1), // 10: Error message if any
            Constraint::Length(2), // 11: Action buttons
        ])
        .split(inner_area);

    render_text_input(frame, inner[1], &state.form_prefix, "Model Prefix", &Theme);
    render_text_input(frame, inner[3], &state.form_name, "Display Name", &Theme);
    render_text_input(frame, inner[5], &state.form_base_url, "Base URL", &Theme);

    let key_focused = state.focused_field == FormField::ApiKey;
    let mut api_key_input = state.form_api_key.clone();
    api_key_input.is_password = !key_focused;
    render_text_input(frame, inner[7], &api_key_input, "API Key", &Theme);

    let enabled_text = if state.form_enabled {
        " [✓] Provider Enabled for auto-routing "
    } else {
        " [ ] Provider Disabled "
    };
    let enabled_para = Paragraph::new(Line::from(vec![
        Span::styled(
            enabled_text,
            if state.focused_field == FormField::Enabled {
                Theme::badge_gold().add_modifier(Modifier::BOLD)
            } else if state.form_enabled {
                Theme::text_gold()
            } else {
                Theme::text_secondary()
            },
        ),
        Span::raw(" "),
        Span::styled("(Press Space to toggle)", Theme::text_secondary()),
    ]));
    frame.render_widget(enabled_para, inner[9]);

    if let Some(err) = &state.form_error {
        let err_para = Paragraph::new(Line::from(vec![
            Span::styled(" ! ", Theme::text_gold()),
            Span::styled(err, Theme::text_gold()),
        ]));
        frame.render_widget(err_para, inner[10]);
    }

    let save_style = if state.focused_field == FormField::SaveBtn {
        Theme::badge_gold().add_modifier(Modifier::BOLD)
    } else {
        Theme::key_badge()
    };
    let cancel_style = if state.focused_field == FormField::CancelBtn {
        Theme::badge_gold().add_modifier(Modifier::BOLD)
    } else {
        Theme::key_badge()
    };

    let btns = Line::from(vec![
        Span::styled(" [ Save & Apply (Enter) ] ", save_style),
        Span::raw("    "),
        Span::styled(" [ Cancel (Esc) ] ", cancel_style),
    ]);
    let btn_para = Paragraph::new(btns).alignment(Alignment::Center);
    frame.render_widget(btn_para, inner[11]);
}

fn render_delete_modal(frame: &mut Frame, area: Rect, state: &CustomLlmTabState) {
    let popup_area = centered_rect(50, 25, area);
    frame.render_widget(Clear, popup_area);

    let prefix = state.delete_confirm_prefix.as_deref().unwrap_or("unknown");
    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::text_gold())
        .title(Span::styled(
            " ! Confirm Deletion ",
            Theme::text_gold().add_modifier(Modifier::BOLD),
        ))
        .style(Theme::block_base());
    frame.render_widget(outer_block, popup_area);

    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "Are you sure you want to delete custom provider '",
                Theme::text_primary(),
            ),
            Span::styled(prefix, Theme::text_gold().add_modifier(Modifier::BOLD)),
            Span::styled("'?", Theme::text_primary()),
        ]),
        Line::from("This will remove stored API credentials for this endpoint."),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                " [y / Enter] Confirm Delete ",
                Theme::text_gold().add_modifier(Modifier::BOLD),
            ),
            Span::raw("    "),
            Span::styled(" [n / Esc] Cancel ", Theme::key_badge()),
        ]),
    ];

    let para = Paragraph::new(text)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

    let inner_area = Rect {
        x: popup_area.x + 2,
        y: popup_area.y + 2,
        width: popup_area.width.saturating_sub(4),
        height: popup_area.height.saturating_sub(4),
    };

    frame.render_widget(para, inner_area);
}

pub fn get_footer_key_hints(state: &CustomLlmTabState) -> Vec<Span<'static>> {
    if state.is_form_open {
        vec![
            Span::styled(" [Tab/Shift+Tab] ", Theme::key_badge()),
            Span::raw("Next/Prev Field "),
            Span::styled(" [Enter] ", Theme::key_badge()),
            Span::raw("Submit "),
            Span::styled(" [Esc] ", Theme::key_badge()),
            Span::raw("Cancel "),
        ]
    } else {
        vec![
            Span::styled(" [Tab] ", Theme::key_badge()),
            Span::raw("Switch Focus "),
            Span::styled(" [j/k] ", Theme::key_badge()),
            Span::raw("Select "),
            Span::styled(" [a] ", Theme::key_badge()),
            Span::raw("Add Provider "),
            Span::styled(" [e] ", Theme::key_badge()),
            Span::raw("Edit "),
            Span::styled(" [Space] ", Theme::key_badge()),
            Span::raw("Toggle "),
            Span::styled(" [d] ", Theme::key_badge()),
            Span::raw("Delete "),
            Span::styled(" [r] ", Theme::key_badge()),
            Span::raw("Fetch Models "),
            Span::styled(" [1-5] ", Theme::key_badge()),
            Span::raw("Tabs "),
            Span::styled(" [Ctrl+Q] ", Theme::key_badge()),
            Span::raw("Quit "),
        ]
    }
}

pub fn get_help_lines(state: &CustomLlmTabState) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    if state.is_form_open {
        lines.push(Line::from(vec![
            Span::styled("   Tab / ↓        : ", Theme::badge_cyan()),
            Span::raw("Move to next input field"),
        ]));
        lines.push(Line::from(vec![
            Span::styled("   Shift+Tab / ↑  : ", Theme::badge_cyan()),
            Span::raw("Move to previous input field"),
        ]));
        lines.push(Line::from(vec![
            Span::styled("   Space          : ", Theme::badge_cyan()),
            Span::raw("Toggle enable/disable checkbox on provider field"),
        ]));
        lines.push(Line::from(vec![
            Span::styled("   Enter          : ", Theme::badge_cyan()),
            Span::raw("Save custom LLM endpoint configuration"),
        ]));
        lines.push(Line::from(vec![
            Span::styled("   Esc            : ", Theme::badge_cyan()),
            Span::raw("Cancel and close form dialog"),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::styled("   Tab            : ", Theme::badge_cyan()),
            Span::raw("Switch focus between Providers and Models card"),
        ]));
        lines.push(Line::from(vec![
            Span::styled("   j / k (↓ / ↑)  : ", Theme::badge_cyan()),
            Span::raw("Navigate items inside the active card"),
        ]));
        lines.push(Line::from(vec![
            Span::styled("   a              : ", Theme::badge_cyan()),
            Span::raw("Add new custom LLM provider (OpenAI-compatible)"),
        ]));
        lines.push(Line::from(vec![
            Span::styled("   e              : ", Theme::badge_cyan()),
            Span::raw("Edit selected custom LLM provider configuration"),
        ]));
        lines.push(Line::from(vec![
            Span::styled("   Space          : ", Theme::badge_cyan()),
            Span::raw("Enable / disable selected provider"),
        ]));
        lines.push(Line::from(vec![
            Span::styled("   d              : ", Theme::badge_cyan()),
            Span::raw("Delete selected custom LLM endpoint"),
        ]));
        lines.push(Line::from(vec![
            Span::styled("   r              : ", Theme::badge_cyan()),
            Span::raw("Probe / fetch remote model list from /models endpoint"),
        ]));
    }
    lines
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