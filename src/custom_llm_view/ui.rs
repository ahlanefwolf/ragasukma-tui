use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Wrap},
};

use crate::custom_llm_view::types::{CustomLlmFocus, CustomLlmTabState, FormField};
use crate::theme::Theme;

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

    let prefix_focused = state.focused_field == FormField::Prefix;
    render_text_field(
        frame,
        inner[1],
        "Model Prefix",
        "",
        state.form_prefix.as_str(),
        prefix_focused,
        "e.g. groq, vllm, ollama",
    );

    let name_focused = state.focused_field == FormField::Name;
    render_text_field(
        frame,
        inner[3],
        "Display Name",
        "",
        state.form_name.as_str(),
        name_focused,
        "e.g. Groq Cloud LPU",
    );

    let url_focused = state.focused_field == FormField::BaseUrl;
    render_text_field(
        frame,
        inner[5],
        "Base URL",
        "",
        state.form_base_url.as_str(),
        url_focused,
        "e.g. https://api.groq.com/openai/v1",
    );

    let key_focused = state.focused_field == FormField::ApiKey;
    let masked_key: String = if key_focused {
        state.form_api_key.clone()
    } else {
        "•".repeat(state.form_api_key.len())
    };
    render_text_field(
        frame,
        inner[7],
        "API Key",
        "",
        &masked_key,
        key_focused,
        "e.g. gsk_... (leave empty for local)",
    );

    let enabled_text = if state.form_enabled {
        " [✔] Provider Enabled for auto-routing "
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
            Span::styled(" ⚠ ", Theme::text_gold()),
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
            " ⚠ Confirm Deletion ",
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

fn render_text_field(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    icon: &str,
    value: &str,
    is_focused: bool,
    placeholder: &str,
) {
    let title_text = if is_focused {
        format!(" {icon} {title} [Active] ")
    } else {
        format!(" {icon} {title} ")
    };

    let title_span = if is_focused {
        Span::styled(title_text, Theme::badge_gold())
    } else {
        Span::styled(title_text, Theme::badge_gray())
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(if is_focused {
            Theme::border_focused()
        } else {
            Theme::border_subtle()
        })
        .title(title_span)
        .style(Theme::block_base());

    let prompt_prefix = if is_focused {
        Span::styled(
            "❯ ",
            ratatui::style::Style::default()
                .fg(Theme::GOLD_PRIMARY)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled("❯ ", Theme::text_secondary())
    };

    let prompt_width = 2usize;
    let available_width = (area.width.saturating_sub(2) as usize).saturating_sub(prompt_width);

    let content_span = if value.is_empty() {
        Span::styled(
            placeholder,
            Theme::text_secondary().add_modifier(Modifier::ITALIC),
        )
    } else {
        let total_chars = value.chars().count();
        let scroll_offset = if total_chars > available_width && available_width > 0 {
            total_chars - available_width
        } else {
            0
        };
        let visible_slice: String = value.chars().skip(scroll_offset).collect();
        Span::styled(visible_slice, Theme::text_primary())
    };

    let para = Paragraph::new(Line::from(vec![prompt_prefix, content_span])).block(block);
    frame.render_widget(para, area);

    if is_focused {
        let prompt_width_u16 = 2u16; // "❯ "
        let cursor_offset = (value.chars().count() as u16).min(available_width as u16);
        let cursor_x = (area.x + 1 + prompt_width_u16 + cursor_offset)
            .min(area.x + area.width.saturating_sub(2));
        let cursor_y = area.y + 1;
        frame.set_cursor_position((cursor_x, cursor_y));
    }
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
            Span::styled(" [q] ", Theme::key_badge()),
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
