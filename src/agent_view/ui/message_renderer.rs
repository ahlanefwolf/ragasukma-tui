use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

use crate::agent_view::types::{ChatMessage, MessageRole, ToolExecutionStatus};
use crate::theme::Theme;

/// Standard Ratatui text formatting configuration for chat/tool messages.
#[derive(Debug, Clone)]
pub struct MessageRenderConfig {
    pub tab_size: usize,
    pub max_tool_lines: usize,
    pub reasoning_expanded: bool,
    pub tool_expanded: bool,
}

impl Default for MessageRenderConfig {
    fn default() -> Self {
        Self {
            tab_size: 4,
            max_tool_lines: 40,
            reasoning_expanded: true,
            tool_expanded: false,
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

    // 2. Resolve Reasoning and Content (supporting both explicit reasoning field and inline <think> tags)
    let (effective_reasoning, effective_content): (Option<std::borrow::Cow<'a, str>>, std::borrow::Cow<'a, str>) =
        if let Some(r) = &msg.reasoning {
            (Some(std::borrow::Cow::Borrowed(r.as_str())), std::borrow::Cow::Borrowed(msg.content.as_str()))
        } else if let Some(start_idx) = msg.content.find("<think>") {
            let after_think = &msg.content[start_idx + 7..];
            if let Some(end_idx) = after_think.find("</think>") {
                let r = after_think[..end_idx].trim();
                let before = &msg.content[..start_idx];
                let after = &after_think[end_idx + 8..];
                let clean_content = format!("{}{}", before, after).trim().to_string();
                (Some(std::borrow::Cow::Owned(r.to_string())), std::borrow::Cow::Owned(clean_content))
            } else {
                // Incomplete think block (e.g. while still streaming thinking phase)
                let r = after_think.trim();
                (Some(std::borrow::Cow::Owned(r.to_string())), std::borrow::Cow::Borrowed(&msg.content[..start_idx]))
            }
        } else {
            (None, std::borrow::Cow::Borrowed(msg.content.as_str()))
        };

    // Render Reasoning (supporting collapsed and expanded views)
    if let Some(reasoning) = effective_reasoning.as_deref()
        && !reasoning.is_empty()
    {
        if !config.reasoning_expanded {
            lines.push(Line::from(vec![
                Span::styled("    │ ", Theme::dim()),
                Span::styled(
                    "Thinking Process (collapsed)",
                    Style::default()
                        .fg(Theme::TEXT_MUTED)
                        .add_modifier(Modifier::ITALIC),
                ),
            ]));
            lines.push(Line::from(""));
        } else {
            lines.push(Line::from(vec![
                Span::styled("    │ ", Theme::dim()),
                Span::styled(
                    "Thinking Process",
                    Style::default()
                        .fg(Theme::TEXT_MUTED)
                        .add_modifier(Modifier::ITALIC | Modifier::BOLD),
                ),
            ]));
            for raw_line in reasoning.lines() {
                let clean = sanitize_terminal_line(raw_line, config.tab_size);
                lines.push(Line::from(vec![
                    Span::styled("    │ ", Theme::dim()),
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
    }

    // 3. Render Message Content
    if msg.role == MessageRole::Tool {
        let is_failed = effective_content.starts_with("Error:")
            || effective_content.starts_with("Failed:")
            || effective_content.contains("\"is_error\": true")
            || effective_content.contains("\"is_error\":true")
            || effective_content.to_lowercase().starts_with("error");

        let (status_icon, status_text, status_style) = if is_failed {
            (
                "    ✗ ",
                "Tool execution failed",
                Style::default()
                    .fg(Theme::STATUS_ERROR)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            (
                "    ✓ ",
                "Tool execution completed",
                Style::default()
                    .fg(Theme::STATUS_SUCCESS)
                    .add_modifier(Modifier::BOLD),
            )
        };

        if is_failed {
            let error_summary = effective_content
                .lines()
                .next()
                .unwrap_or("Execution encountered an error");
            lines.push(Line::from(vec![
                Span::styled(status_icon, status_style),
                Span::styled(status_text, status_style),
                Span::styled(
                    format!(": {}", error_summary),
                    Style::default().fg(Theme::TEXT_MUTED),
                ),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled(status_icon, status_style),
                Span::styled(status_text, status_style),
            ]));
        }
    } else {
        // Standard message formatting with markdown code block support
        let mut in_code_block = false;

        for raw_line in effective_content.lines() {
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

    // 4. Render Tool Calls (if present: show command summary without raw output payload)
    for tc in &msg.tool_calls {
        let (status_icon, status_color) = match tc.status {
            ToolExecutionStatus::Running => ("⏳ Running", Theme::CYAN_ACCENT),
            ToolExecutionStatus::Success => ("✓ Done", Theme::STATUS_SUCCESS),
            ToolExecutionStatus::Failed => ("✗ Failed", Theme::STATUS_ERROR),
            ToolExecutionStatus::PendingApproval => ("! Pending Approval", Theme::STATUS_WARNING),
        };

        let mut spans = vec![
            Span::styled("    ⚙ ", Style::default().fg(Theme::CYAN_ACCENT)),
            Span::styled(
                &tc.name,
                Style::default()
                    .fg(Theme::GOLD_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
        ];

        let summary = tc.args_summary.trim();
        if !summary.is_empty() {
            // Clean up JSON or single-line formatted command
            let clean_args = summary.replace(['\r', '\n'], " ");
            spans.push(Span::raw(": "));
            spans.push(Span::styled(
                clean_args,
                Style::default().fg(Theme::TEXT_SECONDARY),
            ));
        }

        spans.push(Span::raw(" ("));
        spans.push(Span::styled(
            status_icon,
            Style::default()
                .fg(status_color)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(")"));

        lines.push(Line::from(spans));
    }

    // Spacer between messages
    lines.push(Line::from(""));
    lines
}

/// Helper to detect if a span content represents indentation or gutter padding.
fn is_indent_or_gutter(content: &str) -> bool {
    let trimmed = content.trim_start();
    trimmed.is_empty() || trimmed == "│ " || trimmed == "│"
}

/// Wraps a collection of `Line`s into visual lines respecting `available_width`.
/// - Fast path for lines that already fit within `available_width`.
/// - Continuation line gutter/indent retention: code block gutters (`│ `) and indentations
///   are seamlessly preserved across wrapped lines.
/// - Character and word wrapping with zero unnecessary allocations.
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

    let mut result = Vec::with_capacity(lines.len());

    for line in lines {
        // Fast path: line fits within terminal width as is
        if line.width() <= available_width {
            let static_spans: Vec<Span<'static>> = line
                .spans
                .into_iter()
                .map(|s| Span::styled(s.content.into_owned(), s.style))
                .collect();
            result.push(Line::from(static_spans));
            continue;
        }

        // Determine continuation prefix (such as gutter `│ ` or indentation)
        let continuation_prefix: Option<Span<'static>> = if line.spans.len() >= 2 {
            let first = &line.spans[0];
            if is_indent_or_gutter(&first.content) {
                Some(Span::styled(first.content.to_string(), first.style))
            } else {
                None
            }
        } else {
            None
        };

        let prefix_width = continuation_prefix.as_ref().map(|p| p.width()).unwrap_or(0);
        let can_apply_prefix = prefix_width > 0 && prefix_width + 4 <= available_width;

        let mut current_line_spans: Vec<Span<'static>> = Vec::new();
        let mut current_line_width = 0;

        for (span_idx, span) in line.spans.into_iter().enumerate() {
            let span_style = span.style;
            let text = span.content;

            if span_idx == 0 && can_apply_prefix {
                current_line_spans.push(Span::styled(text.into_owned(), span_style));
                current_line_width += prefix_width;
                continue;
            }

            for word in text.split_inclusive(' ') {
                // Zero-allocation word width check using borrowed span
                let word_width = Span::raw(word).width();

                if word_width == 0 {
                    continue;
                }

                if current_line_width + word_width <= available_width {
                    current_line_spans.push(Span::styled(word.to_string(), span_style));
                    current_line_width += word_width;
                } else {
                    // Check if word fits on a fresh line
                    let fresh_budget = if can_apply_prefix {
                        available_width.saturating_sub(prefix_width)
                    } else {
                        available_width
                    };

                    if word_width > fresh_budget {
                        // Single word exceeds line width budget: split character-by-character
                        if !current_line_spans.is_empty() {
                            result.push(Line::from(std::mem::take(&mut current_line_spans)));
                            current_line_width = 0;
                        }

                        if can_apply_prefix
                            && let Some(ref p) = continuation_prefix
                        {
                            current_line_spans.push(p.clone());
                            current_line_width = prefix_width;
                        }

                        let mut chunk = String::new();
                        let mut chunk_width = 0;
                        let target_budget = available_width.saturating_sub(current_line_width);

                        for ch in word.chars() {
                            let mut buf = [0u8; 4];
                            let ch_str: &str = ch.encode_utf8(&mut buf);
                            let ch_w = Span::raw(ch_str).width();

                            if chunk_width + ch_w > target_budget && !chunk.is_empty() {
                                current_line_spans.push(Span::styled(
                                    std::mem::take(&mut chunk),
                                    span_style,
                                ));
                                result.push(Line::from(std::mem::take(&mut current_line_spans)));
                                current_line_width = 0;

                                if can_apply_prefix
                                    && let Some(ref p) = continuation_prefix
                                {
                                    current_line_spans.push(p.clone());
                                    current_line_width = prefix_width;
                                }
                                chunk_width = 0;
                            }
                            chunk.push(ch);
                            chunk_width += ch_w;
                        }

                        if !chunk.is_empty() {
                            current_line_spans.push(Span::styled(chunk, span_style));
                            current_line_width += chunk_width;
                        }
                    } else {
                        // Word fits on next line: flush current line and start new one
                        if !current_line_spans.is_empty() {
                            result.push(Line::from(std::mem::take(&mut current_line_spans)));
                            current_line_width = 0;
                        }

                        if can_apply_prefix
                            && let Some(ref p) = continuation_prefix
                        {
                            current_line_spans.push(p.clone());
                            current_line_width = prefix_width;
                        }

                        current_line_spans.push(Span::styled(word.to_string(), span_style));
                        current_line_width += word_width;
                    }
                }
            }
        }

        if !current_line_spans.is_empty() {
            // Avoid pushing solitary continuation prefix if nothing else was added
            if !(can_apply_prefix && current_line_spans.len() == 1) {
                result.push(Line::from(current_line_spans));
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_terminal_line() {
        assert_eq!(sanitize_terminal_line("hello\tworld", 4), "hello    world");
        assert_eq!(sanitize_terminal_line("line1\r\nline2", 4), "line1\nline2");
        assert_eq!(sanitize_terminal_line("test\x00\x07bell", 4), "testbell");
    }

    #[test]
    fn test_format_message_timestamp() {
        let ts = format_message_timestamp("now");
        assert!(!ts.is_empty());

        let epoch_s = "1700000000";
        let formatted_s = format_message_timestamp(epoch_s);
        assert!(!formatted_s.is_empty());

        let epoch_ms = "1700000000000";
        let formatted_ms = format_message_timestamp(epoch_ms);
        assert!(!formatted_ms.is_empty());
    }

    #[test]
    fn test_wrap_lines_to_width() {
        let lines = vec![Line::from("hello world this is a test line")];
        let wrapped = wrap_lines_to_width(lines, 12);
        for line in &wrapped {
            assert!(line.width() <= 12);
        }
        assert!(wrapped.len() > 1);
    }

    #[test]
    fn test_wrap_lines_code_block_gutter_preservation() {
        let lines = vec![Line::from(vec![
            Span::raw("    │ "),
            Span::raw("const a_very_long_variable_name_that_needs_wrapping = 42;"),
        ])];
        let wrapped = wrap_lines_to_width(lines, 30);
        assert!(wrapped.len() >= 2);
        for line in &wrapped {
            assert!(line.width() <= 30);
            assert!(
                line.spans[0].content.starts_with("    │ "),
                "Continuation line should retain the gutter prefix"
            );
        }
    }

    #[test]
    fn test_tool_and_reasoning_expansion() {
        let mut msg = ChatMessage {
            role: MessageRole::Assistant,
            sender_name: "Assistant".into(),
            content: "Hello world".into(),
            timestamp: "now".into(),
            tool_calls: Vec::new(),
            reasoning: None,
        };
        msg.reasoning = Some("Detailed step 1\nDetailed step 2".into());

        // Collapsed reasoning
        let config_collapsed = MessageRenderConfig {
            reasoning_expanded: false,
            ..Default::default()
        };
        let lines_collapsed = render_message_to_lines(&msg, &config_collapsed);
        assert!(lines_collapsed
            .iter()
            .any(|l| l.to_string().contains("collapsed")));

        // Expanded reasoning
        let config_expanded = MessageRenderConfig {
            reasoning_expanded: true,
            ..Default::default()
        };
        let lines_expanded = render_message_to_lines(&msg, &config_expanded);
        assert!(lines_expanded
            .iter()
            .any(|l| l.to_string().contains("Detailed step 1")));
    }
}
