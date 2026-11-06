use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

use super::types::{ApprovalRequest, ToolCallInfo, ToolExecutionStatus};
use crate::theme::Theme;

pub fn render_tool_card(tc: &ToolCallInfo, is_expanded: bool, index: usize) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    let (status_badge, status_color) = match tc.status {
        ToolExecutionStatus::PendingApproval => (" ⏸ PENDING APPROVAL ", Color::Yellow),
        ToolExecutionStatus::Running => (" ⏳ RUNNING ", Theme::GOLD_PRIMARY),
        ToolExecutionStatus::Success => (" ✓ SUCCESS ", Color::Green),
        ToolExecutionStatus::Failed => (" ✗ FAILED ", Color::Red),
    };

    let duration_str = tc
        .duration_ms
        .map(|d| format!(" ({}ms)", d))
        .unwrap_or_default();

    let exit_code_str = tc
        .exit_code
        .map(|c| format!(" [exit: {}]", c))
        .unwrap_or_default();

    let toggle_icon = if is_expanded { "▼" } else { "▶" };

    lines.push(Line::from(vec![
        Span::raw("    "),
        Span::styled(
            format!("[{}] {} Tool: {} ", index + 1, toggle_icon, tc.name),
            Style::default()
                .fg(Theme::TEXT_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            status_badge,
            Style::default()
                .fg(status_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{}{}", duration_str, exit_code_str),
            Style::default().fg(Theme::TEXT_SECONDARY),
        ),
    ]));

    if is_expanded {
        // Arguments section
        lines.push(Line::from(vec![
            Span::raw("      ┌─ "),
            Span::styled(
                "Arguments: ",
                Style::default()
                    .fg(Theme::TEXT_SECONDARY)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                tc.args_summary.clone(),
                Style::default().fg(Theme::TEXT_SECONDARY),
            ),
        ]));

        // Terminal output stream
        if let Some(stdout) = &tc.stdout {
            lines.push(Line::from(vec![
                Span::raw("      ├─ "),
                Span::styled(
                    "Terminal Output (stdout):",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
            for line in stdout.lines() {
                lines.push(Line::from(vec![
                    Span::raw("      │  "),
                    Span::styled(line.to_string(), Style::default().fg(Theme::TEXT_PRIMARY)),
                ]));
            }
        }

        if let Some(stderr) = &tc.stderr {
            lines.push(Line::from(vec![
                Span::raw("      ├─ "),
                Span::styled(
                    "Terminal Errors (stderr):",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
            ]));
            for line in stderr.lines() {
                lines.push(Line::from(vec![
                    Span::raw("      │  "),
                    Span::styled(line.to_string(), Style::default().fg(Color::LightRed)),
                ]));
            }
        }

        // Inline Diff Preview
        if let Some(diff) = &tc.diff_preview {
            lines.push(Line::from(vec![
                Span::raw("      ├─ "),
                Span::styled(
                    "Unified Diff Preview:",
                    Style::default()
                        .fg(Theme::GOLD_PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
            for diff_line in render_diff_lines(diff) {
                let mut combined_spans = vec![Span::raw("      │  ")];
                combined_spans.extend(diff_line.spans);
                lines.push(Line::from(combined_spans));
            }
        } else if tc.stdout.is_none()
            && tc.stderr.is_none()
            && let Some(out) = &tc.output_summary
        {
            lines.push(Line::from(vec![
                Span::raw("      ├─ "),
                Span::styled(
                    "Result: ",
                    Style::default()
                        .fg(Theme::TEXT_SECONDARY)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(out.clone(), Style::default().fg(Theme::TEXT_PRIMARY)),
            ]));
        }

        lines.push(Line::from(vec![Span::raw("      └───")]));
    }

    lines
}

pub fn render_diff_lines(diff: &str) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    for line in diff.lines() {
        if line.starts_with("+++") || line.starts_with("---") {
            lines.push(Line::from(Span::styled(
                line.to_string(),
                Style::default()
                    .fg(Theme::TEXT_SECONDARY)
                    .add_modifier(Modifier::BOLD),
            )));
        } else if line.starts_with('+') {
            lines.push(Line::from(Span::styled(
                line.to_string(),
                Style::default().fg(Color::Green),
            )));
        } else if line.starts_with('-') {
            lines.push(Line::from(Span::styled(
                line.to_string(),
                Style::default().fg(Color::Red),
            )));
        } else if line.starts_with("@@") {
            lines.push(Line::from(Span::styled(
                line.to_string(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::ITALIC),
            )));
        } else {
            lines.push(Line::from(Span::styled(
                line.to_string(),
                Style::default().fg(Theme::TEXT_SECONDARY),
            )));
        }
    }

    lines
}

pub fn render_approval_modal(req: &ApprovalRequest, frame: &mut Frame, area: Rect) {
    let modal_width = 74.min(area.width.saturating_sub(4));
    let modal_height = 20.min(area.height.saturating_sub(4));

    let x = area.x + (area.width.saturating_sub(modal_width)) / 2;
    let y = area.y + (area.height.saturating_sub(modal_height)) / 2;
    let modal_area = Rect::new(x, y, modal_width, modal_height);

    frame.render_widget(Clear, modal_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // Header & tool details
            Constraint::Min(4),    // Diff/Command preview
            Constraint::Length(3), // Action buttons
        ])
        .margin(1)
        .split(modal_area);

    let outer_block = Block::default()
        .title(Line::from(vec![
            Span::styled(
                " ⚠ APPROVAL REQUIRED: ",
                Style::default()
                    .fg(Theme::GOLD_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                &req.tool_name,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
        ]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::GOLD_PRIMARY))
        .style(Theme::block_base());

    frame.render_widget(outer_block, modal_area);

    let mut header_lines = vec![Line::from(vec![
        Span::styled(
            "Action Details: ",
            Style::default()
                .fg(Theme::TEXT_SECONDARY)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(&req.details, Style::default().fg(Theme::TEXT_PRIMARY)),
    ])];

    if let Some(target) = &req.target_path {
        header_lines.push(Line::from(vec![
            Span::styled(
                "Target Path:    ",
                Style::default()
                    .fg(Theme::TEXT_SECONDARY)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(target, Style::default().fg(Color::Cyan)),
        ]));
    }

    if let Some(cmd) = &req.command {
        header_lines.push(Line::from(vec![
            Span::styled(
                "Command to Run: ",
                Style::default()
                    .fg(Theme::TEXT_SECONDARY)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(cmd, Style::default().fg(Color::Yellow)),
        ]));
    }

    frame.render_widget(
        Paragraph::new(header_lines).wrap(Wrap { trim: true }),
        chunks[0],
    );

    // Preview area (diff or command output)
    if let Some(diff) = &req.diff_content {
        let diff_lines = render_diff_lines(diff);
        let preview_block = Block::default()
            .title(Span::styled(
                " Proposed Unified Diff ",
                Theme::header_title(),
            ))
            .borders(Borders::ALL)
            .border_style(Theme::border_subtle());

        let p = Paragraph::new(diff_lines)
            .block(preview_block)
            .wrap(Wrap { trim: false });
        frame.render_widget(p, chunks[1]);
    } else {
        let info_p = Paragraph::new(
            "No file modification diff. Tool requests direct system execution approval.",
        )
        .style(Style::default().fg(Theme::TEXT_SECONDARY))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Theme::border_subtle()),
        );
        frame.render_widget(info_p, chunks[1]);
    }

    // Action button hints
    let actions_p = Paragraph::new(Line::from(vec![
        Span::styled(
            " [y] Approve ",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            " [n] Reject ",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            " [Esc] Dismiss ",
            Style::default().fg(Theme::TEXT_SECONDARY),
        ),
    ]));
    frame.render_widget(actions_p, chunks[2]);
}
