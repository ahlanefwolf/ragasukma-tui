use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
    Frame,
};

use crate::theme::Theme;
use super::types::ProviderTabState;

pub fn render_provider_tab(frame: &mut Frame, area: Rect, state: &ProviderTabState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // Header info
            Constraint::Min(10),   // Main content
        ])
        .split(area);

    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(format!(" {} ", state.icon), Theme::header_title()),
            Span::styled(
                format!(" {} Provider Hub", state.provider_name),
                Theme::header_title().add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(" [j/k] ", Theme::key_badge()),
            Span::styled("Pilih  ", Theme::text_secondary()),
            Span::styled(" [n/a] ", Theme::key_badge()),
            Span::styled("+Tambah Akun  ", Theme::text_gold()),
            Span::styled(" [Space/t] ", Theme::key_badge()),
            Span::styled("Aktif/Nonaktif  ", Theme::text_secondary()),
            Span::styled(" [Enter] ", Theme::key_badge()),
            Span::styled("Set Default  ", Theme::text_secondary()),
            Span::styled(" [d] ", Theme::key_badge()),
            Span::styled("Hapus  ", Theme::text_secondary()),
            Span::styled(" [r] ", Theme::key_badge()),
            Span::styled(
                if state.is_refreshing {
                    "Refreshing...  "
                } else {
                    "Refresh Quota  "
                },
                if state.is_refreshing {
                    Theme::text_gold()
                } else {
                    Theme::text_secondary()
                },
            ),
            Span::styled(" [Tab] ", Theme::key_badge()),
            Span::styled("Switch View", Theme::text_secondary()),
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Theme::border_subtle()),
    );
    frame.render_widget(header, chunks[0]);

    render_provider_dashboard(frame, chunks[1], state);
}

pub fn render_provider_dashboard(frame: &mut Frame, area: Rect, state: &ProviderTabState) {
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    // Left: Account List
    let items: Vec<ListItem> = state
        .accounts
        .iter()
        .enumerate()
        .map(|(i, acc)| {
            let is_selected = i == state.selected_account_idx;
            let status_badge = if acc.is_active {
                Span::styled(" ● AKTIF ", Theme::status_success().add_modifier(Modifier::BOLD))
            } else {
                Span::styled(" ○ NONAKTIF ", Theme::text_muted())
            };

            let prefix = if is_selected { "▶ " } else { "  " };

            let line1 = Line::from(vec![
                Span::styled(
                    prefix,
                    if is_selected {
                        Theme::text_gold()
                    } else {
                        Theme::text_muted()
                    },
                ),
                Span::styled(
                    &acc.email,
                    if is_selected {
                        Theme::text_primary().add_modifier(Modifier::BOLD)
                    } else {
                        Theme::text_secondary()
                    },
                ),
            ]);

            let line2 = Line::from(vec![
                Span::raw("    "),
                Span::styled(&acc.plan, Theme::text_muted()),
                Span::raw(" "),
                status_badge,
            ]);

            ListItem::new(vec![line1, line2, Line::from("")])
        })
        .collect();

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::border_gold())
        .title(format!(" Accounts ({}) ", state.accounts.len()));

    if state.accounts.is_empty() {
        let empty_msg = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled("  Belum ada akun.", Theme::text_muted())),
            Line::from(""),
            Line::from(vec![
                Span::raw("  Tekan "),
                Span::styled("[n]", Theme::key_badge()),
                Span::raw(" atau "),
                Span::styled("[a]", Theme::key_badge()),
            ]),
            Line::from(Span::styled("  untuk login akun baru (OAuth).", Theme::text_gold())),
        ])
        .block(list_block);
        frame.render_widget(empty_msg, main_chunks[0]);
    } else {
        let account_list = List::new(items).block(list_block);
        frame.render_widget(account_list, main_chunks[0]);
    }

    // Right: Selected Account Quotas & Details
    if let Some(account) = state.selected_account() {
        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(8), Constraint::Min(8)])
            .split(main_chunks[1]);

        let detail_text = vec![
            Line::from(vec![
                Span::styled("Account ID: ", Theme::text_secondary()),
                Span::styled(&account.id, Theme::text_primary()),
            ]),
            Line::from(vec![
                Span::styled("Email:      ", Theme::text_secondary()),
                Span::styled(&account.email, Theme::text_primary()),
            ]),
            Line::from(vec![
                Span::styled("Tier/Plan:  ", Theme::text_secondary()),
                Span::styled(
                    &account.plan,
                    Theme::text_gold().add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Status:     ", Theme::text_secondary()),
                Span::styled(
                    if account.is_active {
                        "● Aktif (Siap Digunakan - Tekan [Space/t] utk Nonaktifkan)"
                    } else {
                        "○ Nonaktif (Disimpan - Tekan [Space/t] utk Aktifkan)"
                    },
                    if account.is_active {
                        Theme::status_success()
                    } else {
                        Theme::text_muted()
                    },
                ),
            ]),
            Line::from(vec![
                Span::styled("Sync State: ", Theme::text_secondary()),
                Span::styled(
                    state
                        .last_refreshed
                        .as_deref()
                        .unwrap_or("Realtime Streaming Active"),
                    Theme::text_muted(),
                ),
            ]),
        ];

        let account_detail = Paragraph::new(detail_text).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Theme::border_gold())
                .title(" Account Profile "),
        );
        frame.render_widget(account_detail, right_chunks[0]);

        // Model Quotas List with Ratatui Gauges
        let quota_count = account.quotas.len();
        let constraints: Vec<Constraint> = if quota_count == 0 {
            vec![Constraint::Length(2), Constraint::Min(0)]
        } else {
            (0..quota_count)
                .map(|_| Constraint::Length(3))
                .chain(std::iter::once(Constraint::Min(0)))
                .collect()
        };

        let quota_chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints(constraints)
            .split(right_chunks[1]);

        let quota_box = Block::default()
            .borders(Borders::ALL)
            .border_style(Theme::border_gold())
            .title(format!(" Rate Limits & Model Quotas ({}) ", quota_count));
        frame.render_widget(quota_box, right_chunks[1]);

        if account.quotas.is_empty() {
            let empty_text = vec![
                Line::from(Span::styled("Belum ada data kuota model.", Theme::text_muted())),
                Line::from(Span::styled("Tekan [r] untuk memuat/menyegarkan kuota dari server...", Theme::text_gold())),
            ];
            let p = Paragraph::new(empty_text);
            frame.render_widget(p, quota_chunks[0]);
        } else {
            for (i, quota) in account.quotas.iter().enumerate() {
                if i < quota_chunks.len() - 1 {
                    let sub_chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Length(1), Constraint::Length(1)])
                        .split(quota_chunks[i]);

                    let title_line = Line::from(vec![
                        Span::styled(
                            format!("● {}", quota.name),
                            Theme::text_primary().add_modifier(Modifier::BOLD),
                        ),
                        Span::raw("  "),
                        Span::styled(&quota.detail, Theme::text_muted()),
                        Span::raw("  "),
                        Span::styled(
                            format!("(Reset: {})", quota.reset_in),
                            Theme::text_gold(),
                        ),
                    ]);
                    let title_p = Paragraph::new(title_line);
                    frame.render_widget(title_p, sub_chunks[0]);

                    let gauge_color = if quota.percentage >= 80 {
                        Theme::STATUS_SUCCESS
                    } else if quota.percentage >= 40 {
                        Theme::GOLD_PRIMARY
                    } else {
                        Theme::STATUS_DANGER
                    };

                    let gauge = Gauge::default()
                        .gauge_style(
                            ratatui::style::Style::default()
                                .fg(gauge_color),
                        )
                        .percent(quota.percentage)
                        .label(format!("{}% Available", quota.percentage));

                    frame.render_widget(gauge, sub_chunks[1]);
                }
            }
        }
    } else {
        let empty_box = Block::default()
            .borders(Borders::ALL)
            .border_style(Theme::border_subtle())
            .title(" Informasi Akun & Kuota ");
        let msg = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled("  Tidak ada akun yang dipilih.", Theme::text_muted())),
            Line::from(""),
            Line::from(vec![
                Span::raw("  Tekan "),
                Span::styled("[n]", Theme::key_badge()),
                Span::raw(" atau "),
                Span::styled("[a]", Theme::key_badge()),
                Span::styled(" untuk login & tambah akun baru via OAuth browser.", Theme::text_gold()),
            ]),
        ])
        .block(empty_box);
        frame.render_widget(msg, main_chunks[1]);
    }
}

pub fn get_footer_key_hints() -> Vec<Span<'static>> {
    vec![
        Span::styled(" [j/k] ", Theme::key_badge()),
        Span::raw("Select "),
        Span::styled(" [Space] ", Theme::key_badge()),
        Span::raw("Toggle Active "),
        Span::styled(" [Enter] ", Theme::key_badge()),
        Span::raw("Set Default "),
        Span::styled(" [r] ", Theme::key_badge()),
        Span::raw("Refresh "),
        Span::styled(" [a/n] ", Theme::key_badge()),
        Span::raw("Add OAuth "),
        Span::styled(" [d] ", Theme::key_badge()),
        Span::raw("Delete "),
        Span::styled(" [1-5] ", Theme::key_badge()),
        Span::raw("Tabs "),
        Span::styled(" [?] ", Theme::key_badge()),
        Span::raw("Help "),
        Span::styled(" [q] ", Theme::key_badge()),
        Span::raw("Quit "),
    ]
}

pub fn get_help_lines(provider_name: &str) -> Vec<Line<'static>> {
    vec![
        Line::from(vec![
            Span::styled("   j / k (↓ / ↑)  : ", Theme::badge_cyan()),
            Span::raw(format!("Navigate {provider_name} accounts list")),
        ]),
        Line::from(vec![
            Span::styled("   Space          : ", Theme::badge_cyan()),
            Span::raw("Toggle active/inactive status of account"),
        ]),
        Line::from(vec![
            Span::styled("   Enter          : ", Theme::badge_cyan()),
            Span::raw("Set selected account as Default active"),
        ]),
        Line::from(vec![
            Span::styled("   r              : ", Theme::badge_cyan()),
            Span::raw("Refresh model rate limits & quota usage"),
        ]),
        Line::from(vec![
            Span::styled("   a / n          : ", Theme::badge_cyan()),
            Span::raw("Login & add new account via OAuth browser"),
        ]),
        Line::from(vec![
            Span::styled("   d              : ", Theme::badge_cyan()),
            Span::raw("Delete selected account"),
        ]),
    ]
}
