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
            Span::styled(" [a/o] ", Theme::key_badge()),
            Span::styled("Connect New Account  ", Theme::text_secondary()),
            Span::styled(" [j/k/Down/Up] ", Theme::key_badge()),
            Span::styled("Select Account  ", Theme::text_secondary()),
            Span::styled(" [Enter] ", Theme::key_badge()),
            Span::styled("Set Active  ", Theme::text_secondary()),
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
                Span::styled(" ACTIVE ", Theme::badge_gold())
            } else {
                Span::styled(format!(" {} ", acc.status), Theme::badge_cyan())
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

    let account_list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Theme::border_gold())
            .title(format!(" Accounts ({}) ", state.accounts.len())),
    );
    frame.render_widget(account_list, main_chunks[0]);

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
                        "● Active Default Account"
                    } else {
                        "○ Standby"
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
        let constraints: Vec<Constraint> = (0..quota_count)
            .map(|_| Constraint::Length(3))
            .chain(std::iter::once(Constraint::Min(0)))
            .collect();

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
                            .fg(gauge_color)
                            .bg(Theme::BG_SURFACE_ELEVATED),
                    )
                    .percent(quota.percentage)
                    .label(format!("{}% Available", quota.percentage));

                frame.render_widget(gauge, sub_chunks[1]);
            }
        }
    }
}
