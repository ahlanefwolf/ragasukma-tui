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

    // Render base frame without hardcoded background
    let bg_block = Block::default();
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
        crate::agent_view::ui::render_diff_preview_modal(frame, diff, size);
    }

    if let Some(ref err_msg) = app.error_modal {
        render_error_modal(frame, err_msg, size);
    }

    if app.show_help {
        render_help_modal(frame, app, size);
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
        1 => crate::provider_view::render_provider_tab(frame, area, &app.antigravity),
        2 => crate::provider_view::render_provider_tab(frame, area, &app.codex),
        3 => crate::provider_view::render_provider_tab(frame, area, &app.claude),
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

    // Right: Keybindings legend (delegated context-sensitively to domain modules)
    let key_spans = if app.show_help {
        vec![
            Span::styled(" [Esc/q/?] ", Theme::key_badge()),
            Span::raw("Close Help "),
        ]
    } else {
        match app.active_tab {
            0 => crate::agent_view::ui::get_footer_key_hints(&app.agent, area.width),
            1..=3 => crate::provider_view::ui::get_footer_key_hints(),
            4 => crate::custom_llm_view::ui::get_footer_key_hints(&app.custom_llm),
            _ => vec![],
        }
    };

    let legend_line = Line::from(key_spans);
    frame.render_widget(
        Paragraph::new(legend_line)
            .alignment(Alignment::Right)
            .style(Theme::text_muted()),
        footer_chunks[1],
    );
}

fn render_confirm_dialog(frame: &mut Frame, dialog: &crate::app::ConfirmDialogState, area: Rect) {
    let popup_area = centered_rect(50, 20, area);
    frame.render_widget(Clear, popup_area);

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::border_gold())
        .title(Span::styled(format!(" {} ", dialog.title), Theme::header_title()));
    frame.render_widget(outer_block, popup_area);

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(&dialog.message, Theme::text_primary())),
        Line::from(""),
        Line::from(vec![
            Span::styled(" [y / Enter] Confirm ", Theme::badge_gold().add_modifier(Modifier::BOLD)),
            Span::raw("    "),
            Span::styled(" [n / Esc] Cancel ", Theme::text_muted()),
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

fn render_error_modal(frame: &mut Frame, err_msg: &str, area: Rect) {
    let popup_area = centered_rect(60, 30, area);
    frame.render_widget(Clear, popup_area);

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::status_danger())
        .title(Span::styled(" ⚠ SYSTEM ERROR ", Theme::status_danger().add_modifier(Modifier::BOLD)));
    frame.render_widget(outer_block, popup_area);

    let text = vec![
        Line::from(""),
        Line::from(Span::styled("An unexpected error occurred:", Theme::text_muted())),
        Line::from(""),
        Line::from(Span::styled(err_msg, Theme::status_danger())),
        Line::from(""),
        Line::from(vec![
            Span::styled(" [Esc / Enter] ", Theme::key_badge()),
            Span::raw("Dismiss"),
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

fn render_help_modal(frame: &mut Frame, app: &App, area: Rect) {
    let popup_area = centered_rect(75, 75, area);
    frame.render_widget(Clear, popup_area);

    let (active_tab_title, focus_str, specific_lines) = match app.active_tab {
        0 => crate::agent_view::ui::get_help_lines(&app.agent),
        1 => (
            "Antigravity Provider (Tab [2])".to_string(),
            "Accounts & Model Quota",
            crate::provider_view::ui::get_help_lines("Antigravity"),
        ),
        2 => (
            "Codex Provider (Tab [3])".to_string(),
            "Accounts & Model Quota",
            crate::provider_view::ui::get_help_lines("Codex"),
        ),
        3 => (
            "Claude Provider (Tab [4])".to_string(),
            "Accounts & Model Quota",
            crate::provider_view::ui::get_help_lines("Claude"),
        ),
        4 => (
            "Custom LLM Endpoints (Tab [5])".to_string(),
            if app.custom_llm.is_form_open {
                "Add/Edit Custom Provider Form"
            } else {
                "Custom Provider Endpoints List"
            },
            crate::custom_llm_view::ui::get_help_lines(&app.custom_llm),
        ),
        _ => ("Unknown".to_string(), "General", vec![]),
    };

    let mut lines = vec![
        Line::from(vec![
            Span::styled("  TAB / FOKUS SAAT INI: ", Theme::badge_gold()),
            Span::styled(active_tab_title, Theme::header_title().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  KOMPONEN AKTIF       : ", Theme::text_muted()),
            Span::styled(focus_str, Style::default().fg(Theme::CYAN_ACCENT)),
        ]),
        Line::from(""),
        Line::from(Span::styled("  --- Perintah Khusus Konteks Aktif ---", Theme::badge_cyan())),
    ];

    lines.extend(specific_lines);

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("  --- Navigasi Global ---", Theme::badge_cyan())));
    lines.push(Line::from(vec![
        Span::styled("   1 .. 5         : ", Theme::badge_cyan()),
        Span::raw("Switch tabs (1:Agent, 2:Antigravity, 3:Codex, 4:Claude, 5:Custom LLM)"),
    ]));
    lines.push(Line::from(vec![
        Span::styled("   Tab / Shift+Tab: ", Theme::badge_cyan()),
        Span::raw("Cycle fokus antar panel / field aktif"),
    ]));
    lines.push(Line::from(vec![
        Span::styled("   ?              : ", Theme::badge_cyan()),
        Span::raw("Buka / tutup panduan bantuan ini (Help modal)"),
    ]));
    lines.push(Line::from(vec![
        Span::styled("   Ctrl+C         : ", Theme::badge_cyan()),
        Span::raw("Keluar dari aplikasi (Force Quit)"),
    ]));
    lines.push(Line::from(vec![
        Span::styled("   q              : ", Theme::badge_cyan()),
        Span::raw("Keluar aplikasi (ketika sedang tidak mengetik prompt)"),
    ]));

    let help_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::border_gold())
        .title(Span::styled(" 📖 Panduan Tombol Cepat (Bantuan) ", Theme::badge_gold()));

    let help_para = Paragraph::new(lines).block(help_block).wrap(Wrap { trim: true });
    frame.render_widget(help_para, popup_area);
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
