use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

use crate::agent_view::types::{ChatMessage, MessageRole, ToolExecutionStatus};
use crate::theme::Theme;

/// Standard Ratatui text formatting configuration for chat/tool messages.
pub struct MessageRenderConfig {
    pub tab_size: usize,
    pub max_tool_lines: usize,
}

impl Default for MessageRenderConfig {
    fn default() -> Self {
        Self {
            tab_size: 4,
            max_tool_lines: 40,
        }
    }
}

/// Standard string sanitization for Ratatui terminal:
/// 1. Converts tabs (`\t`) to a consistent number of spaces (prevents terminal cursor vs Ratatui cell desync).
/// 2. Strips carriage returns (`\r`) and non-printable control characters.
pub fn sanitize_terminal_line(input: &str, tab_size: usize) -> String {
    let spaces = " ".repeat(tab_size);
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\t' => out.push_str(&spaces),
            '\r' => {}
            ch if (ch as u32) < 32 && ch != '\n' => {}
            ch => out.push(ch),
        }
    }
    out
}

/// Formats raw timestamp string (Unix Epoch in seconds/milliseconds, "now", etc.)
/// into a compact local time format: `MM/DD HH:MM` (e.g. `04/04 14:03`).
pub fn format_message_timestamp(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return chrono::Local::now().format("%m/%d %H:%M").to_string();
    }

    if let Ok(ts) = trimmed.parse::<i64>() {
        // If unix epoch number: detect seconds vs milliseconds
        let dt = if ts > 100_000_000_000 {
            chrono::DateTime::from_timestamp_millis(ts)
        } else {
            chrono::DateTime::from_timestamp(ts, 0)
        };

        if let Some(utc) = dt {
            let local: chrono::DateTime<chrono::Local> = chrono::DateTime::from(utc);
            return local.format("%m/%d %H:%M").to_string();
        }
    }

    if trimmed.eq_ignore_ascii_case("now") {
        return chrono::Local::now().format("%m/%d %H:%M").to_string();
    }

    // If already formatted or other format, return as is
    trimmed.to_string()
}

/// Default helper to render `ChatMessage` with standard configuration.
pub fn render_message_standard<'a>(msg: &'a ChatMessage) -> Vec<Line<'a>> {
    let config = MessageRenderConfig::default();
    render_message_to_lines(msg, &config)
}

/// Renders a chat/tool message into a standardized and safe collection of Ratatui `Line`s.
pub fn render_message_to_lines<'a>(
    msg: &'a ChatMessage,
    config: &MessageRenderConfig,
) -> Vec<Line<'a>> {
    let mut lines = Vec::new();

    // 1. Render Message Header / Badge
    let (badge, badge_style) = match msg.role {
        MessageRole::User => (" You ", Theme::badge_info()),
        MessageRole::Assistant => (" Assistant ", Theme::badge_gold()),
        MessageRole::System => (" System ", Theme::badge_gray()),
        MessageRole::Tool => (" Tool Output ", Theme::badge_cyan()),
    };

    let formatted_ts = format_message_timestamp(&msg.timestamp);

    lines.push(Line::from(vec![
        Span::styled(badge, badge_style),
        Span::raw(" "),
        Span::styled(
            format!("[{}]", formatted_ts),
            Style::default().fg(Theme::TEXT_SECONDARY),
        ),
    ]));

    // 2. Render Reasoning (if present)
    if let Some(reasoning) = &msg.reasoning
        && !reasoning.is_empty()
    {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "  Thinking / Reasoning:",
                Style::default()
                    .fg(Theme::TEXT_MUTED)
                    .add_modifier(Modifier::ITALIC),
            ),
        ]));
        for raw_line in reasoning.lines() {
            let clean = sanitize_terminal_line(raw_line, config.tab_size);
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(
                    clean,
                    Style::default()
                        .fg(Theme::TEXT_MUTED)
                        .add_modifier(Modifier::ITALIC),
                ),
            ]));
        }
        lines.push(Line::from(""));
    }

    // 3. Render Message Content
    if msg.role == MessageRole::Tool {
        // Tool output is not displayed in full; only execution status (success or failure) is shown
        let is_failed = msg.content.starts_with("Error:")
            || msg.content.starts_with("Failed:")
            || msg.content.contains("\"is_error\": true")
            || msg.content.contains("\"is_error\":true")
            || msg.content.to_lowercase().starts_with("error");

        if is_failed {
            let error_summary = msg
                .content
                .lines()
                .next()
                .unwrap_or("Execution encountered an error");
            lines.push(Line::from(vec![
                Span::styled(
                    "    ✗ ",
                    Style::default()
                        .fg(Theme::STATUS_ERROR)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Tool execution failed",
                    Style::default()
                        .fg(Theme::STATUS_ERROR)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(": {}", error_summary),
                    Style::default().fg(Theme::TEXT_MUTED),
                ),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled(
                    "    ✓ ",
                    Style::default()
                        .fg(Theme::STATUS_SUCCESS)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Tool execution succeeded",
                    Style::default()
                        .fg(Theme::STATUS_SUCCESS)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
        }
    } else {
        // Standard message formatting with markdown code block support
        let mut in_code_block = false;

        for raw_line in msg.content.lines() {
            let clean = sanitize_terminal_line(raw_line, config.tab_size);
            let trimmed = clean.trim_start();

            // Check markdown code block ``` tag
            if trimmed.starts_with("```") {
                if in_code_block {
                    // Close code block
                    in_code_block = false;
                    lines.push(Line::from(vec![
                        Span::raw("    "),
                        Span::styled(
                            "╰──────────────────────────────────────────",
                            Theme::text_muted(),
                        ),
                    ]));
                } else {
                    // Open code block
                    in_code_block = true;
                    let code_lang = trimmed.trim_start_matches('`').trim();
                    let lang_display = if code_lang.is_empty() {
                        "code"
                    } else {
                        code_lang
                    };
                    lines.push(Line::from(vec![
                        Span::raw("    "),
                        Span::styled(
                            format!("╭─ [ {} ]", lang_display),
                            Style::default()
                                .fg(Theme::GOLD_PRIMARY)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]));
                }
                continue;
            }

            if in_code_block {
                lines.push(Line::from(vec![
                    Span::styled("    │ ", Theme::dim()),
                    Span::styled(clean, Style::default().fg(Theme::CYAN_ACCENT)),
                ]));
                continue;
            }

            let style = if trimmed.starts_with('#') {
                Style::default()
                    .fg(Theme::GOLD_PRIMARY)
                    .add_modifier(Modifier::BOLD)
            } else if trimmed.starts_with("> ") {
                Style::default()
                    .fg(Theme::TEXT_SECONDARY)
                    .add_modifier(Modifier::ITALIC)
            } else {
                Style::default().fg(Theme::TEXT_PRIMARY)
            };

            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(clean, style),
            ]));
        }

        // If stream is still inside code block without closing ```
        if in_code_block {
            lines.push(Line::from(vec![
                Span::styled("    │ ", Theme::dim()),
                Span::styled(
                    "▊ (streaming code...)",
                    Style::default()
                        .fg(Theme::GOLD_PRIMARY)
                        .add_modifier(Modifier::ITALIC),
                ),
            ]));
        }
    }

    // 4. Render Tool Calls (if present)
    for tc in &msg.tool_calls {
        let status_icon = match tc.status {
            ToolExecutionStatus::Running => "⏳ Running...",
            ToolExecutionStatus::Success => "✓ Completed",
            ToolExecutionStatus::Failed => "✗ Failed",
            ToolExecutionStatus::PendingApproval => "⚠ Pending Approval",
        };
        let status_color = match tc.status {
            ToolExecutionStatus::Running => Theme::CYAN_ACCENT,
            ToolExecutionStatus::Success => Theme::STATUS_SUCCESS,
            ToolExecutionStatus::Failed => Theme::STATUS_ERROR,
            ToolExecutionStatus::PendingApproval => Theme::STATUS_WARNING,
        };

        lines.push(Line::from(vec![
            Span::raw("   ["),
            Span::styled(
                &tc.name,
                Style::default()
                    .fg(Theme::GOLD_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("] "),
            Span::styled(status_icon, Style::default().fg(status_color)),
        ]));
    }

    // Spacer between messages
    lines.push(Line::from(""));
    lines
}

/// Wraps a collection of `Line`s into visual lines respecting `available_width`.
/// Each line is guaranteed to have visual width <= `available_width`, so that
/// `result.len()` maps 1:1 with physical terminal lines.
pub fn wrap_lines_to_width(lines: Vec<Line<'_>>, available_width: usize) -> Vec<Line<'static>> {
    if available_width == 0 {
        return lines
            .into_iter()
            .map(|l| {
                let static_spans: Vec<Span<'static>> = l
                    .spans
                    .into_iter()
                    .map(|s| Span::styled(s.content.into_owned(), s.style))
                    .collect();
                Line::from(static_spans)
            })
            .collect();
    }

    let mut result = Vec::with_capacity(lines.len() * 2);

    for line in lines {
        if line.width() <= available_width {
            let static_spans: Vec<Span<'static>> = line
                .spans
                .into_iter()
                .map(|s| Span::styled(s.content.into_owned(), s.style))
                .collect();
            result.push(Line::from(static_spans));
            continue;
        }

        let mut current_line_spans: Vec<Span<'static>> = Vec::new();
        let mut current_line_width = 0;

        for span in line.spans {
            let span_style = span.style;
            let text = span.content.into_owned();

            for word in text.split_inclusive(' ') {
                let word_span = Span::styled(word.to_string(), span_style);
                let word_width = word_span.width();

                if word_width == 0 {
                    continue;
                }

                if current_line_width + word_width <= available_width {
                    current_line_spans.push(word_span);
                    current_line_width += word_width;
                } else if word_width > available_width {
                    // Single word longer than screen width - split by character
                    if !current_line_spans.is_empty() {
                        result.push(Line::from(std::mem::take(&mut current_line_spans)));
                        current_line_width = 0;
                    }

                    let mut chunk = String::new();
                    let mut chunk_width = 0;
                    for ch in word.chars() {
                        let ch_span = Span::raw(ch.to_string());
                        let ch_w = ch_span.width();
                        if chunk_width + ch_w > available_width && !chunk.is_empty() {
                            result.push(Line::from(vec![Span::styled(
                                std::mem::take(&mut chunk),
                                span_style,
                            )]));
                            chunk_width = 0;
                        }
                        chunk.push(ch);
                        chunk_width += ch_w;
                    }
                    if !chunk.is_empty() {
                        current_line_spans.push(Span::styled(chunk, span_style));
                        current_line_width = chunk_width;
                    }
                } else {
                    // Move to new line
                    if !current_line_spans.is_empty() {
                        result.push(Line::from(std::mem::take(&mut current_line_spans)));
                    }
                    current_line_spans.push(word_span);
                    current_line_width = word_width;
                }
            }
        }

        if !current_line_spans.is_empty() {
            result.push(Line::from(current_line_spans));
        }
    }

    result
}

/// Computes visual line height from a slice of `Line`s taking `available_width` into account.
pub fn calculate_total_visual_lines(lines: &[Line], available_width: usize) -> usize {
    if available_width == 0 {
        return lines.len();
    }
    let cloned: Vec<Line> = lines
        .iter()
        .map(|l| {
            let spans: Vec<Span> = l
                .spans
                .iter()
                .map(|s| Span::styled(s.content.to_string(), s.style))
                .collect();
            Line::from(spans)
        })
        .collect();
    wrap_lines_to_width(cloned, available_width).len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_view::types::MessageRole;

    #[test]
    fn test_format_message_timestamp() {
        // Unix timestamp: 1793753006 -> valid "MM/DD HH:MM" pattern (11 characters)
        let formatted = format_message_timestamp("1793753006");
        assert_eq!(formatted.len(), 11);
        assert_eq!(&formatted[2..3], "/");
        assert_eq!(&formatted[5..6], " ");
        assert_eq!(&formatted[8..9], ":");

        let now_formatted = format_message_timestamp("now");
        assert_eq!(now_formatted.len(), 11);

        let passthrough = format_message_timestamp("Custom Timestamp");
        assert_eq!(passthrough, "Custom Timestamp");
    }

    #[test]
    fn test_sanitize_terminal_line_removes_tabs_and_control_chars() {
        let dirty = "line1\tvalue\r\nline2\x07bell";
        let sanitized = sanitize_terminal_line(dirty, 4);
        assert!(!sanitized.contains('\t'));
        assert!(!sanitized.contains('\r'));
        assert!(!sanitized.contains('\x07'));
        assert!(sanitized.contains("line1    value"));
        assert!(sanitized.contains("line2bell"));
    }

    #[test]
    fn test_calculate_total_visual_lines() {
        let lines = vec![
            Line::from("hello world"),
            Line::from(""),
            Line::from("this is a longer line to test wrapping logic"),
        ];
        let count_wide = calculate_total_visual_lines(&lines, 100);
        assert_eq!(count_wide, 3);

        let count_narrow = calculate_total_visual_lines(&lines, 10);
        assert_eq!(count_narrow, 9);
    }

    #[test]
    fn test_tool_message_truncation() {
        let mut content = String::new();
        for i in 1..=100 {
            content.push_str(&format!("line {}\n", i));
        }

        let msg_success = ChatMessage {
            role: MessageRole::Tool,
            sender_name: "tool".to_string(),
            content,
            timestamp: "12:00:00".to_string(),
            tool_calls: vec![],
            reasoning: None,
        };

        let config = MessageRenderConfig {
            tab_size: 4,
            max_tool_lines: 20,
        };

        let lines_success = render_message_to_lines(&msg_success, &config);
        // Header + 1 status line (Success) + 1 blank spacing line = 3 lines
        assert_eq!(lines_success.len(), 3);
        let joined_success = lines_success
            .iter()
            .map(|l| l.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(joined_success.contains("Tool execution succeeded"));
        assert!(!joined_success.contains("line 50"));

        let msg_failed = ChatMessage {
            role: MessageRole::Tool,
            sender_name: "tool".to_string(),
            content: "Error: file not found".to_string(),
            timestamp: "12:00:00".to_string(),
            tool_calls: vec![],
            reasoning: None,
        };

        let lines_failed = render_message_to_lines(&msg_failed, &config);
        assert_eq!(lines_failed.len(), 3);
        let joined_failed = lines_failed
            .iter()
            .map(|l| l.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(joined_failed.contains("Tool execution failed"));
        assert!(joined_failed.contains("file not found"));
    }
}
