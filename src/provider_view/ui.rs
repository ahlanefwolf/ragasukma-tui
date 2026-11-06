use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
};

use super::types::{ProviderFocusArea, ProviderTabState};
use crate::theme::Theme;

pub fn render_provider_tab(frame: &mut Frame, area: Rect, state: &ProviderTabState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Header info
            Constraint::Min(10),   // Content
        ])
        .split(area);

    let header_lines = vec![Line::from(vec![
        Span::styled(
            format!("  {} Hub", state.provider_name),
            Theme::header_title(),
        ),
        Span::styled(
            if state.is_refreshing {
                "  [Refreshing Quotas...]"
            } else {
                ""
            },
            Theme::text_gold(),
        ),
    ])];
    let header = Paragraph::new(header_lines).block(
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
                Span::styled(
                    " ● ACTIVE ",
                    Theme::status_success().add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled(" ○ INACTIVE ", Theme::text_secondary())
            };

            let prefix = if is_selected { "❯ " } else { "  " };

            let line1 = Line::from(vec![
                Span::styled(
                    prefix,
                    if is_selected {
                        Theme::text_gold()
                    } else {
                        Theme::text_secondary()
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
                Span::styled(&acc.plan, Theme::text_secondary()),
                Span::raw(" "),
                status_badge,
            ]);

            ListItem::new(vec![line1, line2, Line::from("")])
        })
        .collect();

    let is_accounts_focused = state.focus_area == ProviderFocusArea::Accounts;
    let list_border_style = if is_accounts_focused {
        Theme::border_gold()
    } else {
        Theme::border_subtle()
    };
    let list_title = if is_accounts_focused {
        format!(" Accounts ({}) [FOCUSED] ", state.accounts.len())
    } else {
        format!(" Accounts ({}) [Tab to focus] ", state.accounts.len())
    };

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_style(list_border_style)
        .title(list_title);

    if state.accounts.is_empty() {
        let empty_msg = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled("  No accounts yet.", Theme::text_secondary())),
            Line::from(""),
            Line::from(vec![
                Span::raw("  Press "),
                Span::styled("[a]", Theme::key_badge()),
                Span::raw(" to add account."),
            ]),
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
                        "● Active (Ready to Use)"
                    } else {
                        "○ Inactive (Saved)"
                    },
                    if account.is_active {
                        Theme::status_success()
                    } else {
                        Theme::text_secondary()
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
                    Theme::text_secondary(),
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
        let card_height = right_chunks[1].height.saturating_sub(2) as usize; // remove border margins
        let items_per_page = (card_height / 3).max(1);

        let scroll_offset = state.quota_scroll_offset.min(quota_count.saturating_sub(1));
        let visible_quotas = if quota_count == 0 {
            &[][..]
        } else {
            let end = (scroll_offset + items_per_page).min(quota_count);
            &account.quotas[scroll_offset..end]
        };

        let constraints: Vec<Constraint> = if visible_quotas.is_empty() {
            vec![Constraint::Length(2), Constraint::Min(0)]
        } else {
            visible_quotas
                .iter()
                .map(|_| Constraint::Length(3))
                .chain(std::iter::once(Constraint::Min(0)))
                .collect()
        };

        let quota_chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints(constraints)
            .split(right_chunks[1]);

        let is_quotas_focused = state.focus_area == ProviderFocusArea::Quotas;
        let quota_border_style = if is_quotas_focused {
            Theme::border_gold()
        } else {
            Theme::border_subtle()
        };

        let title_scroll_info = if is_quotas_focused {
            if quota_count > items_per_page {
                format!(
                    " Rate Limits & Model Quotas ({}) [FOCUSED - {}-{}/{}] ",
                    quota_count,
                    scroll_offset + 1,
                    (scroll_offset + visible_quotas.len()).min(quota_count),
                    quota_count
                )
            } else {
                format!(" Rate Limits & Model Quotas ({}) [FOCUSED] ", quota_count)
            }
        } else {
            if quota_count > items_per_page {
                format!(
                    " Rate Limits & Model Quotas ({}) [{}-{}/{}] ",
                    quota_count,
                    scroll_offset + 1,
                    (scroll_offset + visible_quotas.len()).min(quota_count),
                    quota_count
                )
            } else {
                format!(" Rate Limits & Model Quotas ({}) ", quota_count)
            }
        };

        let quota_box = Block::default()
            .borders(Borders::ALL)
            .border_style(quota_border_style)
            .title(title_scroll_info);
        frame.render_widget(quota_box, right_chunks[1]);

        if account.quotas.is_empty() {
            let empty_text = vec![
                Line::from(Span::styled(
                    "No model quota data available.",
                    Theme::text_secondary(),
                )),
                Line::from(Span::styled(
                    "Press [r] to fetch/refresh quota from server...",
                    Theme::text_gold(),
                )),
            ];
            let p = Paragraph::new(empty_text);
            frame.render_widget(p, quota_chunks[0]);
        } else {
            for (i, quota) in visible_quotas.iter().enumerate() {
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
                        Span::styled(&quota.detail, Theme::text_secondary()),
                        Span::raw("  "),
                        Span::styled(format!("(Reset: {})", quota.reset_in), Theme::text_gold()),
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
                        .gauge_style(ratatui::style::Style::default().fg(gauge_color))
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
            .title(" Account & Quota Info ");
        let msg = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No account selected.",
                Theme::text_secondary(),
            )),
            Line::from(""),
            Line::from(vec![
                Span::raw("  Press "),
                Span::styled("[a]", Theme::key_badge()),
                Span::raw(" to add account."),
            ]),
        ])
        .block(empty_box);
        frame.render_widget(msg, main_chunks[1]);
    }
}

pub fn get_footer_key_hints(state: &ProviderTabState) -> Vec<Span<'static>> {
    match state.focus_area {
        ProviderFocusArea::Accounts => vec![
            Span::styled(" [Tab] ", Theme::key_badge()),
            Span::raw("Quotas "),
            Span::styled(" [j/k] ", Theme::key_badge()),
            Span::raw("Select "),
            Span::styled(" [Space] ", Theme::key_badge()),
            Span::raw("Active "),
            Span::styled(" [a] ", Theme::key_badge()),
            Span::raw("Add "),
            Span::styled(" [d] ", Theme::key_badge()),
            Span::raw("Delete "),
            Span::styled(" [r] ", Theme::key_badge()),
            Span::raw("Refresh "),
            Span::styled(" [1-5] ", Theme::key_badge()),
            Span::raw("Tabs "),
            Span::styled(" [Ctrl+Q] ", Theme::key_badge()),
            Span::raw("Quit"),
        ],
        ProviderFocusArea::Quotas => vec![
            Span::styled(" [Tab] ", Theme::key_badge()),
            Span::raw("Accounts "),
            Span::styled(" [j/k] ", Theme::key_badge()),
            Span::raw("Scroll "),
            Span::styled(" [r] ", Theme::key_badge()),
            Span::raw("Refresh "),
            Span::styled(" [1-5] ", Theme::key_badge()),
            Span::raw("Tabs "),
            Span::styled(" [Ctrl+Q] ", Theme::key_badge()),
            Span::raw("Quit"),
        ],
    }
}
