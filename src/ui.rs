pub mod text_input;
pub use text_input::{render_text_input, InputResult, TextInput, TextInputMode};

use crate::app::App;
use crate::theme::Theme;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Tabs, Wrap},
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
}

fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(22), Constraint::Min(35)])
        .split(area);

    // Title Block
    let title_line = Line::from(vec![
        Span::styled(" RAGASUKMA ", Theme::header_title()),
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
    let tabs = Tabs::new(tab_titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Theme::border_subtle())
                .style(Theme::block_base()),
        )
        .highlight_style(Theme::tab_active())
        .select(app.active_tab);
    frame.render_widget(tabs, header_chunks[1]);
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
    let key_spans = match app.active_tab {
        0 => crate::agent_view::ui::get_footer_key_hints(&app.agent, area.width),
        1 => crate::provider_view::ui::get_footer_key_hints(&app.antigravity),
        2 => crate::provider_view::ui::get_footer_key_hints(&app.codex),
        3 => crate::provider_view::ui::get_footer_key_hints(&app.claude),
        4 => crate::custom_llm_view::ui::get_footer_key_hints(&app.custom_llm),
        _ => vec![],
    };

    let legend_line = Line::from(key_spans);
    frame.render_widget(
        Paragraph::new(legend_line)
            .alignment(Alignment::Right)
            .style(Theme::text_secondary()),
        area,
    );
}

fn render_confirm_dialog(frame: &mut Frame, dialog: &crate::app::ConfirmDialogState, area: Rect) {
    let popup_area = centered_rect(50, 20, area);
    frame.render_widget(Clear, popup_area);

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::border_gold())
        .title(Span::styled(
            format!(" {} ", dialog.title),
            Theme::header_title(),
        ));
    frame.render_widget(outer_block, popup_area);

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(&dialog.message, Theme::text_primary())),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                " [y / Enter] Confirm ",
                Theme::badge_gold().add_modifier(Modifier::BOLD),
            ),
            Span::raw("    "),
            Span::styled(" [n / Esc] Cancel ", Theme::text_secondary()),
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
        .title(Span::styled(
            " ! SYSTEM ERROR ",
            Theme::status_danger().add_modifier(Modifier::BOLD),
        ));
    frame.render_widget(outer_block, popup_area);

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "An unexpected error occurred:",
            Theme::text_secondary(),
        )),
        Line::from(""),
        Line::from(Span::styled(err_msg, Theme::status_danger())),
        Line::from(""),
        Line::from(vec![
            Span::styled(" [Esc] ", Theme::key_badge()),
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

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let px = percent_x.min(100);
    let py = percent_y.min(100);
    let vertical_margin = (100 - py) / 2;
    let horizontal_margin = (100 - px) / 2;

    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(vertical_margin),
            Constraint::Percentage(py),
            Constraint::Percentage(100 - py - vertical_margin),
        ])
        .split(r);

    let target_row = popup_layout.get(1).copied().unwrap_or(r);

    let horizontal_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(horizontal_margin),
            Constraint::Percentage(px),
            Constraint::Percentage(100 - px - horizontal_margin),
        ])
        .split(target_row);

    horizontal_layout.get(1).copied().unwrap_or(target_row)
}
