use crate::app::App;
use crate::theme::Theme;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Tabs, Wrap},
    Frame,
};

pub fn draw(frame: &mut Frame, app: &App) {
    let size = frame.area();

    // Fill background with canvas color
    let bg_block = Block::default().style(Style::default().bg(Theme::BG_CANVAS));
    frame.render_widget(bg_block, size);

    // Main layout: Header (3), Body (Min 0), Footer (2)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(2),
        ])
        .split(size);

    render_header(frame, chunks[0], app);
    render_body(frame, chunks[1], app);
    render_footer(frame, chunks[2], app);

    if let Some(ref dialog) = app.confirm_dialog {
        render_confirm_dialog(frame, dialog, size);
    }

    if let Some(ref diff) = app.diff_modal {
        render_diff_preview_modal(frame, diff, size);
    }

    if let Some(ref err_msg) = app.error_modal {
        render_error_modal(frame, err_msg, size);
    }

    if app.show_help {
        render_help_modal(frame, size);
    }
}

fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(22),
            Constraint::Min(35),
            Constraint::Length(20),
        ])
        .split(area);

    // Title Block
    let title_line = Line::from(vec![
        Span::styled("⚡ RAGASUKMA ", Theme::header_title()),
        Span::styled("TUI", Theme::badge_cyan()),
    ]);
    let title_widget = Paragraph::new(title_line)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Theme::border_gold())
                .style(Theme::block_base()),
        )
        .alignment(Alignment::Center);
    frame.render_widget(title_widget, header_chunks[0]);

    // Tabs Block
    let tab_titles = vec![
        " [1] Agent ",
        " [2] Antigravity ",
        " [3] Codex ",
        " [4] Claude ",
        " [5] Custom LLM ",
    ];
    let tabs_widget = Tabs::new(tab_titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Theme::border_subtle())
                .style(Theme::block_base()),
        )
        .select(app.active_tab)
        .style(Theme::tab_inactive())
        .highlight_style(Theme::tab_active())
        .divider(Span::styled("│", Theme::border_subtle()));
    frame.render_widget(tabs_widget, header_chunks[1]);

    // Status Pill Block
    let status_line = Line::from(vec![
        Span::styled("● ", Theme::status_success()),
        Span::styled("READY", Style::default().fg(Theme::TEXT_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::raw(" | "),
        Span::styled(format!("T:{}", app.tick_count), Style::default().fg(Theme::CYAN_ACCENT)),
    ]);
    let status_widget = Paragraph::new(status_line)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Theme::border_subtle())
                .style(Theme::block_base()),
        )
        .alignment(Alignment::Center);
    frame.render_widget(status_widget, header_chunks[2]);
}

fn render_body(frame: &mut Frame, area: Rect, app: &App) {
    match app.active_tab {
        0 => crate::agent_view::render_agent_tab(frame, area, &app.agent),
        1 => crate::provider_views::render_provider_tab(frame, area, &app.antigravity),
        2 => crate::provider_views::render_provider_tab(frame, area, &app.codex),
        3 => crate::provider_views::render_provider_tab(frame, area, &app.claude),
        4 => crate::custom_llm_view::render_custom_llm_tab(frame, area, &app.custom_llm),
        _ => {}
    }
}

fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    let status_msg = app.status_message.as_deref().unwrap_or("Ready");
    let status_width = (status_msg.chars().count() as u16 + 12)
        .min(area.width.saturating_sub(20))
        .max(16);

    let footer_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(status_width), Constraint::Min(0)])
        .split(area);

    // Left: Status notification
    let status_line = Line::from(vec![
        Span::styled(" STATUS: ", Theme::key_badge()),
        Span::styled(format!(" {status_msg}"), Style::default().fg(Theme::TEXT_PRIMARY)),
    ]);
    frame.render_widget(Paragraph::new(status_line), footer_chunks[0]);

    // Right: Keybindings legend (context-sensitive)
    let key_spans = if app.show_help {
        vec![
            Span::styled(" [Esc/q/?] ", Theme::key_badge()),
            Span::raw("Close Help "),
        ]
    } else if app.active_tab == 0 {
        if app.agent.is_input_focused {
            vec![
                Span::styled(" [Enter] ", Theme::key_badge()),
                Span::raw("Send "),
                Span::styled(" [Esc] ", Theme::key_badge()),
                Span::raw("Unfocus "),
                Span::styled(" [Ctrl+U] ", Theme::key_badge()),
                Span::raw("Clear "),
                Span::styled(" [Ctrl+C] ", Theme::key_badge()),
                Span::raw("Quit "),
            ]
        } else if area.width < 90 {
            vec![
                Span::styled(" [i] ", Theme::key_badge()),
                Span::raw("Prompt "),
                Span::styled(" [j/k] ", Theme::key_badge()),
                Span::raw("Nav "),
                Span::styled(" [x] ", Theme::key_badge()),
                Span::raw("Tools "),
                Span::styled(" [?] ", Theme::key_badge()),
                Span::raw("Help "),
                Span::styled(" [q] ", Theme::key_badge()),
                Span::raw("Quit "),
            ]
        } else {
            match app.agent.focus_area {
                crate::agent_view::AgentFocusArea::Sessions => vec![
                    Span::styled(" [Enter] ", Theme::key_badge()),
                    Span::raw("Open "),
                    Span::styled(" [j/k] ", Theme::key_badge()),
                    Span::raw("Nav "),
                    Span::styled(" [n] ", Theme::key_badge()),
                    Span::raw("New "),
                    Span::styled(" [d] ", Theme::key_badge()),
                    Span::raw("Del "),
                    Span::styled(" [f/c] ", Theme::key_badge()),
                    Span::raw("Panel "),
                    Span::styled(" [1-5] ", Theme::key_badge()),
                    Span::raw("Tabs "),
                    Span::styled(" [?] ", Theme::key_badge()),
                    Span::raw("Help "),
                    Span::styled(" [q] ", Theme::key_badge()),
                    Span::raw("Quit "),
                ],
                crate::agent_view::AgentFocusArea::Files => vec![
                    Span::styled(" [Enter] ", Theme::key_badge()),
                    Span::raw("Insert "),
                    Span::styled(" [j/k] ", Theme::key_badge()),
                    Span::raw("Nav "),
                    Span::styled(" [r] ", Theme::key_badge()),
                    Span::raw("Scan "),
                    Span::styled(" [s/c] ", Theme::key_badge()),
                    Span::raw("Panel "),
                    Span::styled(" [1-5] ", Theme::key_badge()),
                    Span::raw("Tabs "),
                    Span::styled(" [?] ", Theme::key_badge()),
                    Span::raw("Help "),
                    Span::styled(" [q] ", Theme::key_badge()),
                    Span::raw("Quit "),
                ],
                crate::agent_view::AgentFocusArea::Chat => vec![
                    Span::styled(" [i/Enter] ", Theme::key_badge()),
                    Span::raw("Prompt "),
                    Span::styled(" [j/k] ", Theme::key_badge()),
                    Span::raw("Scroll "),
                    Span::styled(" [x] ", Theme::key_badge()),
                    Span::raw("Tools "),
                    Span::styled(" [n] ", Theme::key_badge()),
                    Span::raw("New "),
                    Span::styled(" [s/f] ", Theme::key_badge()),
                    Span::raw("Panel "),
                    Span::styled(" [1-5] ", Theme::key_badge()),
                    Span::raw("Tabs "),
                    Span::styled(" [?] ", Theme::key_badge()),
                    Span::raw("Help "),
                    Span::styled(" [q] ", Theme::key_badge()),
                    Span::raw("Quit "),
                ],
            }
        }
    } else if app.active_tab == 4 {
        if app.custom_llm.delete_confirm_prefix.is_some() {
            vec![
                Span::styled(" [y/Enter] ", Theme::key_badge()),
                Span::raw("Confirm Delete "),
                Span::styled(" [n/Esc] ", Theme::key_badge()),
                Span::raw("Cancel "),
            ]
        } else if app.custom_llm.is_form_open {
            vec![
                Span::styled(" [Tab/↓] ", Theme::key_badge()),
                Span::raw("Next Field "),
                Span::styled(" [Space] ", Theme::key_badge()),
                Span::raw("Toggle "),
                Span::styled(" [Enter] ", Theme::key_badge()),
                Span::raw("Save "),
                Span::styled(" [Esc] ", Theme::key_badge()),
                Span::raw("Cancel "),
            ]
        } else {
            vec![
                Span::styled(" [j/k] ", Theme::key_badge()),
                Span::raw("Nav "),
                Span::styled(" [Space] ", Theme::key_badge()),
                Span::raw("Toggle "),
                Span::styled(" [n] ", Theme::key_badge()),
                Span::raw("Add "),
                Span::styled(" [e] ", Theme::key_badge()),
                Span::raw("Edit "),
                Span::styled(" [d] ", Theme::key_badge()),
                Span::raw("Del "),
                Span::styled(" [r] ", Theme::key_badge()),
                Span::raw("Models "),
                Span::styled(" [?] ", Theme::key_badge()),
                Span::raw("Help "),
                Span::styled(" [q] ", Theme::key_badge()),
                Span::raw("Quit "),
            ]
        }
    } else {
        let mut spans = vec![
            Span::styled(" [1-5] ", Theme::key_badge()),
            Span::raw("Tabs "),
            Span::styled(" [Tab] ", Theme::key_badge()),
            Span::raw("Nav "),
        ];

        if app.active_tab >= 1 && app.active_tab <= 3 {
            spans.push(Span::styled(" [j/k] ", Theme::key_badge()));
            spans.push(Span::raw("Accts "));
            spans.push(Span::styled(" [Enter] ", Theme::key_badge()));
            spans.push(Span::raw("Active "));
            spans.push(Span::styled(" [r] ", Theme::key_badge()));
            spans.push(Span::raw("Refresh "));
        }

        spans.push(Span::styled(" [?] ", Theme::key_badge()));
        spans.push(Span::raw("Help "));
        spans.push(Span::styled(" [q] ", Theme::key_badge()));
        spans.push(Span::raw("Quit "));
        spans
    };

    let keys_line = Line::from(key_spans);
    frame.render_widget(
        Paragraph::new(keys_line).alignment(Alignment::Right),
        footer_chunks[1],
    );
}

fn render_help_modal(frame: &mut Frame, area: Rect) {
    let popup_area = centered_rect(75, 75, area);

    frame.render_widget(Clear, popup_area);

    let help_card = Block::default()
        .title(Span::styled(" ❓ Keyboard Shortcuts & Help Cheatsheet ", Theme::header_title()))
        .borders(Borders::ALL)
        .border_style(Theme::border_focused())
        .style(Theme::block_base());

    let help_text = vec![
        Line::from(vec![
            Span::styled("⚡ Global Navigation:", Style::default().fg(Theme::GOLD_PRIMARY).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("   1 - 5          : ", Theme::badge_cyan()),
            Span::raw("Langsung lompat ke Tab 1 (Agent), 2 (Antigravity), 3 (Codex), 4 (Claude), 5 (Custom LLM)"),
        ]),
        Line::from(vec![
            Span::styled("   Tab / l / →    : ", Theme::badge_cyan()),
            Span::raw("Pindah ke Tab berikutnya"),
        ]),
        Line::from(vec![
            Span::styled("   Shift+Tab/h/←  : ", Theme::badge_cyan()),
            Span::raw("Pindah ke Tab sebelumnya"),
        ]),
        Line::from(vec![
            Span::styled("   ? / F1         : ", Theme::badge_cyan()),
            Span::raw("Buka / Tutup jendela bantuan ini"),
        ]),
        Line::from(vec![
            Span::styled("   Esc / q        : ", Theme::badge_cyan()),
            Span::raw("Tutup popup / Keluar dari aplikasi"),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("🤖 Tab 1 - Agent Workspace:", Style::default().fg(Theme::CYAN_ACCENT).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("   i / Enter      : ", Theme::badge_cyan()),
            Span::raw("Fokus ke input prompt (ketik pertanyaan/instruksi)"),
        ]),
        Line::from(vec![
            Span::styled("   Enter          : ", Theme::badge_cyan()),
            Span::raw("Kirim prompt saat sedang mengetik"),
        ]),
        Line::from(vec![
            Span::styled("   Esc            : ", Theme::badge_cyan()),
            Span::raw("Keluar dari fokus prompt (kembali ke mode navigasi)"),
        ]),
        Line::from(vec![
            Span::styled("   f / s / w      : ", Theme::badge_cyan()),
            Span::raw("Fokus cepat ke File Tree (f), Sessions (s), atau Chat (w)"),
        ]),
        Line::from(vec![
            Span::styled("   [ / ] or h / l : ", Theme::badge_cyan()),
            Span::raw("Pindah fokus panel (Sessions <-> Files <-> Chat)"),
        ]),
        Line::from(vec![
            Span::styled("   j / k          : ", Theme::badge_cyan()),
            Span::raw("Navigasi item / scroll di panel yang sedang aktif"),
        ]),
        Line::from(vec![
            Span::styled("   Enter          : ", Theme::badge_cyan()),
            Span::raw("Sessions: muat sesi | Files: insert @file ke prompt | Chat: tulis prompt"),
        ]),
        Line::from(vec![
            Span::styled("   n              : ", Theme::badge_cyan()),
            Span::raw("Mulai sesi chat baru"),
        ]),
        Line::from(vec![
            Span::styled("   d              : ", Theme::badge_cyan()),
            Span::raw("Hapus sesi terpilih (saat di panel Sessions)"),
        ]),
        Line::from(vec![
            Span::styled("   r              : ", Theme::badge_cyan()),
            Span::raw("Rescan workspace files (saat di panel Files)"),
        ]),
        Line::from(vec![
            Span::styled("   x              : ", Theme::badge_cyan()),
            Span::raw("Toggle rincian input argumen & output pada Tool Call cards"),
        ]),
        Line::from(vec![
            Span::styled("   c              : ", Theme::badge_cyan()),
            Span::raw("Bersihkan riwayat percakapan sesi aktif"),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("📊 Tab 2-4 - Quota Visualizer (Antigravity, Codex, Claude):", Style::default().fg(Theme::STATUS_SUCCESS).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("   j / k          : ", Theme::badge_cyan()),
            Span::raw("Pilih akun berikutnya / sebelumnya"),
        ]),
        Line::from(vec![
            Span::styled("   Enter          : ", Theme::badge_cyan()),
            Span::raw("Set akun terpilih sebagai default aktif"),
        ]),
        Line::from(vec![
            Span::styled("   r              : ", Theme::badge_cyan()),
            Span::raw("Refresh kuota dan model limits akun"),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("🛠️ Tab 5 - Custom LLM Provider Manager:", Style::default().fg(Theme::GOLD_PRIMARY).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("   j / k          : ", Theme::badge_cyan()),
            Span::raw("Navigasi akun custom provider"),
        ]),
        Line::from(vec![
            Span::styled("   Space          : ", Theme::badge_cyan()),
            Span::raw("Toggle Enable / Disable provider terpilih"),
        ]),
        Line::from(vec![
            Span::styled("   n              : ", Theme::badge_cyan()),
            Span::raw("Buka modal tambah provider baru"),
        ]),
        Line::from(vec![
            Span::styled("   e              : ", Theme::badge_cyan()),
            Span::raw("Edit konfigurasi provider terpilih"),
        ]),
        Line::from(vec![
            Span::styled("   d              : ", Theme::badge_cyan()),
            Span::raw("Hapus provider terpilih (dengan dialog konfirmasi)"),
        ]),
        Line::from(vec![
            Span::styled("   r              : ", Theme::badge_cyan()),
            Span::raw("Buka modal daftar model yang tersedia"),
        ]),
    ];

    let help_widget = Paragraph::new(help_text)
        .block(help_card)
        .wrap(Wrap { trim: true });
    frame.render_widget(help_widget, popup_area);
}

fn render_confirm_dialog(frame: &mut Frame, dialog: &crate::app::ConfirmDialogState, area: Rect) {
    let popup_area = centered_rect(52, 28, area);
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(Span::styled(
            format!(" ⚠️ {} ", dialog.title),
            Style::default()
                .fg(Theme::STATUS_WARNING)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::STATUS_WARNING))
        .style(Theme::card_base());

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(" ", Style::default()),
            Span::styled(&dialog.prompt, Style::default().fg(Theme::TEXT_PRIMARY)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("   [ Y ] ", Style::default().fg(Theme::STATUS_SUCCESS).add_modifier(Modifier::BOLD)),
            Span::styled("Konfirmasi / Lanjutkan   ", Style::default().fg(Theme::TEXT_SECONDARY)),
            Span::styled("[ N / Esc ] ", Style::default().fg(Theme::STATUS_DANGER).add_modifier(Modifier::BOLD)),
            Span::styled("Batal", Style::default().fg(Theme::TEXT_SECONDARY)),
        ]),
    ];

    let widget = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
    frame.render_widget(widget, popup_area);
}

fn render_error_modal(frame: &mut Frame, error_msg: &str, area: Rect) {
    let popup_area = centered_rect(58, 30, area);
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(Span::styled(
            " ⛔ KESALAHAN SISTEM (SYSTEM ERROR) ",
            Style::default()
                .fg(Theme::STATUS_DANGER)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::STATUS_DANGER))
        .style(Theme::card_base());

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(" Error Detail: ", Style::default().fg(Theme::STATUS_WARNING).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled(format!("  {}", error_msg), Style::default().fg(Theme::TEXT_PRIMARY)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" [ Enter / Esc / Space / q ] ", Theme::key_badge()),
            Span::styled(" Tutup dialog", Style::default().fg(Theme::TEXT_MUTED)),
        ]),
    ];

    let widget = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: true });
    frame.render_widget(widget, popup_area);
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

fn render_diff_preview_modal(
    frame: &mut Frame,
    diff: &crate::app::DiffApprovalState,
    area: Rect,
) {
    let popup_area = centered_rect(80, 80, area);
    frame.render_widget(Clear, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header / File info
            Constraint::Min(5),    // Diff lines
            Constraint::Length(3), // Footer buttons
        ])
        .split(popup_area);

    // 1. Header Box
    let title = format!(" [⚡ DIFF APPROVAL] {} ", diff.tool_name);
    let header_text = vec![
        Line::from(vec![
            Span::styled("Target File: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(&diff.file_path, Style::default().fg(Theme::GOLD_PRIMARY).add_modifier(Modifier::BOLD)),
        ]),
    ];
    let header_widget = Paragraph::new(header_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Theme::TEXT_SECONDARY))
            .title(title),
    );
    frame.render_widget(header_widget, chunks[0]);

    // 2. Diff Lines Box
    let visible_height = (chunks[1].height as usize).saturating_sub(2);
    let total_lines = diff.diff_lines.len();
    let start_idx = diff.scroll_offset.min(total_lines.saturating_sub(1));
    let end_idx = (start_idx + visible_height).min(total_lines);

    let mut rendered_lines = Vec::new();
    for l in &diff.diff_lines[start_idx..end_idx] {
        let style = match l.line_type {
            crate::app::DiffLineType::Added => Style::default().fg(Theme::STATUS_SUCCESS),
            crate::app::DiffLineType::Removed => Style::default().fg(Theme::STATUS_DANGER),
            crate::app::DiffLineType::Header => Style::default().fg(Theme::GOLD_PRIMARY).add_modifier(Modifier::BOLD),
            crate::app::DiffLineType::Context => Style::default().fg(Theme::TEXT_MUTED),
        };
        rendered_lines.push(Line::from(Span::styled(&l.content, style)));
    }

    let scroll_info = format!(" Lines {}-{}/{} [↑/↓/PgUp/PgDn] ", start_idx + 1, end_idx, total_lines);
    let diff_widget = Paragraph::new(rendered_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Theme::BORDER_GOLD))
            .title(scroll_info),
    );
    frame.render_widget(diff_widget, chunks[1]);

    // 3. Footer Action Buttons
    let footer_text = vec![
        Line::from(vec![
            Span::styled(" [Y / Enter] ", Style::default().fg(Theme::TEXT_PRIMARY).bg(Theme::STATUS_SUCCESS).add_modifier(Modifier::BOLD)),
            Span::raw(" Approve & Apply Patch    "),
            Span::styled(" [N / Esc] ", Style::default().fg(Theme::TEXT_PRIMARY).bg(Theme::STATUS_DANGER).add_modifier(Modifier::BOLD)),
            Span::raw(" Reject Patch"),
        ]),
    ];
    let footer_widget = Paragraph::new(footer_text)
        .alignment(ratatui::layout::Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Theme::BORDER_GOLD)),
        );
    frame.render_widget(footer_widget, chunks[2]);
}
