use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Wrap},
    Frame,
};

use crate::custom_llm_view::types::{CustomLlmTabState, FormField};
use crate::theme::Theme;

pub fn render_custom_llm_tab(frame: &mut Frame, area: Rect, state: &CustomLlmTabState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(8),
            Constraint::Length(10),
        ])
        .split(area);

    render_header_banner(frame, chunks[0]);
    render_accounts_table(frame, chunks[1], state);
    render_bottom_panes(frame, chunks[2], state);

    if state.is_form_open {
        render_form_modal(frame, area, state);
    }

    if state.delete_confirm_prefix.is_some() {
        render_delete_modal(frame, area, state);
    }
}

fn render_header_banner(frame: &mut Frame, area: Rect) {
    let title = Line::from(vec![
        Span::styled(" 🌐 CUSTOM LLM & OPENAI-COMPATIBLE ENDPOINTS ", Theme::header_title()),
        Span::raw(" "),
        Span::styled(
            "[Groq, DeepSeek, Together, Ollama, vLLM, LMStudio, OpenRouter]",
            Theme::text_secondary(),
        ),
    ]);

    let shortcuts = Line::from(vec![
        Span::styled(" [a] ", Theme::key_badge()),
        Span::styled("Add Endpoint   ", Theme::text_gold()),
        Span::styled(" [e] ", Theme::key_badge()),
        Span::styled("Edit   ", Theme::text_gold()),
        Span::styled(" [Space/t] ", Theme::key_badge()),
        Span::styled("Toggle Active   ", Theme::text_gold()),
        Span::styled(" [d] ", Theme::key_badge()),
        Span::styled("Delete   ", Theme::status_danger()),
        Span::styled(" [r] ", Theme::key_badge()),
        Span::styled("Fetch Models", Theme::status_info()),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::border_gold())
        .title(title);

    let paragraph = Paragraph::new(vec![Line::from(""), shortcuts])
        .block(block)
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
}

fn render_accounts_table(frame: &mut Frame, area: Rect, state: &CustomLlmTabState) {
    let header_cells = [
        "Prefix / ID",
        "Provider Name",
        "Base URL Endpoint",
        "API Key",
        "Status",
    ]
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
                    Theme::header_title().add_modifier(Modifier::BOLD)
                } else {
                    Theme::text_secondary()
                },
            ));

            let name_cell = Cell::from(Span::styled(
                acc.name.clone(),
                if is_selected {
                    Theme::text_gold().add_modifier(Modifier::BOLD)
                } else {
                    Theme::text_primary()
                },
            ));

            let url_cell = Cell::from(Span::styled(acc.base_url.clone(), Theme::text_secondary()));

            let masked_key = if acc.api_key.is_empty() {
                Span::styled("(none / local)", Theme::text_secondary())
            } else if acc.api_key.len() > 8 {
                let start = &acc.api_key[..4];
                let end = &acc.api_key[acc.api_key.len() - 3..];
                Span::styled(format!("{start}••••{end}"), Theme::text_secondary())
            } else {
                Span::styled("••••••••", Theme::text_secondary())
            };
            let key_cell = Cell::from(masked_key);

            let status_span = if acc.enabled {
                Span::styled(" ● ENABLED ", Theme::status_success().add_modifier(Modifier::BOLD))
            } else {
                Span::styled(" ○ DISABLED ", Theme::text_secondary())
            };
            let status_cell = Cell::from(status_span);

            let row_style = if is_selected {
                Theme::badge_gold().add_modifier(Modifier::BOLD)
            } else {
                Theme::card_base()
            };

            Row::new(vec![
                prefix_cell,
                name_cell,
                url_cell,
                key_cell,
                status_cell,
            ])
            .style(row_style)
            .height(1)
        })
        .collect();

    let widths = [
        Constraint::Length(16),
        Constraint::Percentage(25),
        Constraint::Percentage(35),
        Constraint::Length(18),
        Constraint::Length(16),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(if state.is_form_open {
                    Theme::border_subtle()
                } else {
                    Theme::border_gold()
                })
                .title(Span::styled(
                    format!(" Configured Providers ({}) ", state.accounts.len()),
                    Theme::header_title(),
                )),
        );

    frame.render_widget(table, area);
}

fn render_bottom_panes(frame: &mut Frame, area: Rect, state: &CustomLlmTabState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let selected_acc = state.selected_account();

    let details_text = if let Some(acc) = selected_acc {
        vec![
            Line::from(vec![
                Span::styled("Name:         ", Theme::text_secondary()),
                Span::styled(&acc.name, Theme::text_gold().add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("Model Prefix: ", Theme::text_secondary()),
                Span::styled(
                    format!("{}/<model_name>", acc.prefix),
                    Theme::status_info().add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Endpoint:     ", Theme::text_secondary()),
                Span::styled(&acc.base_url, Theme::text_primary()),
            ]),
            Line::from(vec![
                Span::styled("Status:       ", Theme::text_secondary()),
                if acc.enabled {
                    Span::styled("Active & Available for Agent routing", Theme::status_success())
                } else {
                    Span::styled("Inactive (Toggle with [Space])", Theme::text_secondary())
                },
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Usage Example: ", Theme::header_title()),
                Span::styled(
                    format!("ragasukma run --model {}/llama-3.3-70b-versatile", acc.prefix),
                    Theme::key_badge(),
                ),
            ]),
        ]
    } else {
        vec![Line::from(Span::styled(
            "No custom provider selected. Press 'a' to add a new one.",
            Theme::text_secondary(),
        ))]
    };

    let details_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::border_gold())
        .title(Span::styled(" Selected Provider Configuration ", Theme::header_title()));

    let details_para = Paragraph::new(details_text)
        .block(details_block)
        .wrap(Wrap { trim: true });
    frame.render_widget(details_para, chunks[0]);

    let models_text = if let Some(acc) = selected_acc {
        let prefix_models = state.models_for_prefix(&acc.prefix);
        if prefix_models.is_empty() {
            vec![
                Line::from(Span::styled(
                    "No cached models found for this prefix.",
                    Theme::text_secondary(),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Press [r] to fetch/probe remote models via /models endpoint",
                    Theme::status_info(),
                )),
            ]
        } else {
            prefix_models
                .iter()
                .map(|(m, _)| {
                    Line::from(vec![
                        Span::styled(" ✔ ", Theme::status_success()),
                        Span::styled(m, Theme::text_primary()),
                    ])
                })
                .collect()
        }
    } else {
        vec![Line::from(Span::styled(
            "Select a provider to inspect its registered models",
            Theme::text_secondary(),
        ))]
    };

    let models_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::border_subtle())
        .title(Span::styled(" Registered Model Catalog ", Theme::header_title()));

    let models_para = Paragraph::new(models_text).block(models_block);
    frame.render_widget(models_para, chunks[1]);
}

fn render_form_modal(frame: &mut Frame, area: Rect, state: &CustomLlmTabState) {
    let popup_area = centered_rect(65, 60, area);
    frame.render_widget(Clear, popup_area);

    let is_edit = state.editing_prefix.is_some();
    let title = if is_edit {
        " ✏ Edit Custom Provider "
    } else {
        " ➕ Add Custom LLM Provider "
    };

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::border_gold())
        .title(Span::styled(title, Theme::header_title().add_modifier(Modifier::BOLD)));
    frame.render_widget(outer_block, popup_area);

    let inner = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3), // Prefix
            Constraint::Length(3), // Name
            Constraint::Length(3), // Base URL
            Constraint::Length(3), // API Key
            Constraint::Length(2), // Enabled Checkbox
            Constraint::Length(2), // Error message if any
            Constraint::Length(3), // Action buttons
        ])
        .split(popup_area);

    let prefix_block = Block::default()
        .borders(Borders::ALL)
        .border_style(field_label_style(state.focused_field == FormField::Prefix))
        .title(" Model Prefix (lowercase, e.g. groq, vllm) ");
    let prefix_para = Paragraph::new(state.form_prefix.as_str()).block(prefix_block);
    frame.render_widget(prefix_para, inner[0]);

    let name_block = Block::default()
        .borders(Borders::ALL)
        .border_style(field_label_style(state.focused_field == FormField::Name))
        .title(" Display Name (e.g. Groq Cloud LPU) ");
    let name_para = Paragraph::new(state.form_name.as_str()).block(name_block);
    frame.render_widget(name_para, inner[1]);

    let url_block = Block::default()
        .borders(Borders::ALL)
        .border_style(field_label_style(state.focused_field == FormField::BaseUrl))
        .title(" OpenAI Base URL (e.g. https://api.groq.com/openai/v1) ");
    let url_para = Paragraph::new(state.form_base_url.as_str()).block(url_block);
    frame.render_widget(url_para, inner[2]);

    let key_block = Block::default()
        .borders(Borders::ALL)
        .border_style(field_label_style(state.focused_field == FormField::ApiKey))
        .title(" API Key (leave empty for local Ollama) ");
    let masked_input: String = if state.focused_field == FormField::ApiKey {
        state.form_api_key.clone()
    } else {
        "•".repeat(state.form_api_key.len())
    };
    let key_para = Paragraph::new(masked_input).block(key_block);
    frame.render_widget(key_para, inner[3]);

    let enabled_text = if state.form_enabled {
        "[X] Provider Enabled for auto-routing"
    } else {
        "[ ] Provider Disabled"
    };
    let enabled_para = Paragraph::new(Line::from(vec![
        Span::styled(
            enabled_text,
            if state.focused_field == FormField::Enabled {
                Theme::header_title().add_modifier(Modifier::BOLD)
            } else {
                Theme::text_secondary()
            },
        ),
        Span::raw(" "),
        Span::styled("(Press Space to toggle)", Theme::text_secondary()),
    ]));
    frame.render_widget(enabled_para, inner[4]);

    if let Some(err) = &state.form_error {
        let err_para = Paragraph::new(Line::from(vec![
            Span::styled(" ⚠ ", Theme::status_danger()),
            Span::styled(err, Theme::status_danger()),
        ]));
        frame.render_widget(err_para, inner[5]);
    }

    let save_style = if state.focused_field == FormField::SaveBtn {
        Theme::badge_gold().add_modifier(Modifier::BOLD)
    } else {
        Theme::header_title()
    };
    let cancel_style = if state.focused_field == FormField::CancelBtn {
        Theme::badge_gold().add_modifier(Modifier::BOLD)
    } else {
        Theme::text_secondary()
    };

    let btns = Line::from(vec![
        Span::styled(" [ Save & Apply (Enter) ] ", save_style),
        Span::raw("    "),
        Span::styled(" [ Cancel (Esc) ] ", cancel_style),
    ]);
    let btn_para = Paragraph::new(btns).alignment(Alignment::Center);
    frame.render_widget(btn_para, inner[6]);
}

fn render_delete_modal(frame: &mut Frame, area: Rect, state: &CustomLlmTabState) {
    let popup_area = centered_rect(50, 25, area);
    frame.render_widget(Clear, popup_area);

    let prefix = state.delete_confirm_prefix.as_deref().unwrap_or("unknown");
    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::status_danger())
        .title(Span::styled(" ⚠ Confirm Deletion ", Theme::status_danger()));
    frame.render_widget(outer_block, popup_area);

    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Are you sure you want to delete custom provider '", Theme::text_primary()),
            Span::styled(prefix, Theme::status_danger().add_modifier(Modifier::BOLD)),
            Span::styled("'?", Theme::text_primary()),
        ]),
        Line::from("This will remove stored API credentials for this endpoint."),
        Line::from(""),
        Line::from(vec![
            Span::styled(" [y / Enter] Confirm Delete ", Theme::status_danger().add_modifier(Modifier::BOLD)),
            Span::raw("    "),
            Span::styled(" [n / Esc] Cancel ", Theme::text_primary()),
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

fn field_label_style(is_focused: bool) -> ratatui::style::Style {
    if is_focused {
        Theme::border_gold()
    } else {
        Theme::border_subtle()
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
            Span::styled(" [j/k] ", Theme::key_badge()),
            Span::raw("Select "),
            Span::styled(" [a] ", Theme::key_badge()),
            Span::raw("Add Model "),
            Span::styled(" [e] ", Theme::key_badge()),
            Span::raw("Edit "),
            Span::styled(" [d] ", Theme::key_badge()),
            Span::raw("Delete "),
            Span::styled(" [Space] ", Theme::key_badge()),
            Span::raw("Toggle Active "),
            Span::styled(" [1-5] ", Theme::key_badge()),
            Span::raw("Tabs "),
            Span::styled(" [?] ", Theme::key_badge()),
            Span::raw("Help "),
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
            Span::raw("Pindah ke field input berikutnya"),
        ]));
        lines.push(Line::from(vec![
            Span::styled("   Shift+Tab / ↑  : ", Theme::badge_cyan()),
            Span::raw("Pindah ke field input sebelumnya"),
        ]));
        lines.push(Line::from(vec![
            Span::styled("   Space          : ", Theme::badge_cyan()),
            Span::raw("Toggle checkbox enable/disable pada field provider"),
        ]));
        lines.push(Line::from(vec![
            Span::styled("   Enter          : ", Theme::badge_cyan()),
            Span::raw("Simpan konfigurasi custom LLM endpoint"),
        ]));
        lines.push(Line::from(vec![
            Span::styled("   Esc            : ", Theme::badge_cyan()),
            Span::raw("Batal dan tutup form dialog"),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::styled("   j / k (↓ / ↑)  : ", Theme::badge_cyan()),
            Span::raw("Pilih provider LLM kustom dari daftar"),
        ]));
        lines.push(Line::from(vec![
            Span::styled("   a              : ", Theme::badge_cyan()),
            Span::raw("Tambah provider LLM kustom baru (OpenAI-compatible)"),
        ]));
        lines.push(Line::from(vec![
            Span::styled("   e              : ", Theme::badge_cyan()),
            Span::raw("Edit konfigurasi model & base URL yang dipilih"),
        ]));
        lines.push(Line::from(vec![
            Span::styled("   Space          : ", Theme::badge_cyan()),
            Span::raw("Aktifkan / nonaktifkan provider yang dipilih"),
        ]));
        lines.push(Line::from(vec![
            Span::styled("   d              : ", Theme::badge_cyan()),
            Span::raw("Hapus endpoint custom LLM terpilih"),
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
