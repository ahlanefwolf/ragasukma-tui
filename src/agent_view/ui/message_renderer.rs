use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

use crate::agent_view::types::{ChatMessage, MessageRole, ToolExecutionStatus};
use crate::theme::Theme;

/// Default number of chat messages to render initially (lazy load window limit).
pub const DEFAULT_CHAT_MESSAGE_LIMIT: usize = 25;

/// Number of older messages to load when scrolling up to the top.
pub const CHAT_LAZY_LOAD_STEP: usize = 25;

/// Returns a sliced window of messages for lazy rendering and the count of hidden older messages.
pub fn get_lazy_message_window(
    messages: &[ChatMessage],
    limit: usize,
) -> (&[ChatMessage], usize) {
    let effective_limit = limit.max(1);
    if messages.len() > effective_limit {
        let hidden = messages.len() - effective_limit;
        (&messages[hidden..], hidden)
    } else {
        (messages, 0)
    }
}

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
        render_tool_output_message(
            &effective_content,
            config.tab_size,
            config.tool_expanded,
            config.max_tool_lines,
            &mut lines,
        );
    } else {
        // Standard message formatting with markdown code block & inline support
        let mut in_code_block = false;
        let mut code_lang = String::new();

        for raw_line in effective_content.lines() {
            let clean = sanitize_terminal_line(raw_line, config.tab_size);
            let trimmed = clean.trim_start();

            // Check markdown code block ``` tag
            if trimmed.starts_with("```") {
                if in_code_block {
                    // Close code block
                    in_code_block = false;
                    code_lang.clear();
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
                    code_lang = trimmed.trim_start_matches('`').trim().to_string();
                    let lang_display = if code_lang.is_empty() {
                        "code"
                    } else {
                        code_lang.as_str()
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
                let is_diff = code_lang.eq_ignore_ascii_case("diff")
                    || code_lang.eq_ignore_ascii_case("patch");
                let code_style = if is_diff {
                    if clean.starts_with('+') && !clean.starts_with("+++") {
                        Style::default().fg(Theme::STATUS_SUCCESS)
                    } else if clean.starts_with('-') && !clean.starts_with("---") {
                        Style::default().fg(Theme::STATUS_ERROR)
                    } else if clean.starts_with("@@") {
                        Style::default().fg(Theme::CYAN_ACCENT)
                    } else {
                        Style::default().fg(Theme::TEXT_SECONDARY)
                    }
                } else {
                    Style::default().fg(Theme::CYAN_ACCENT)
                };
                lines.push(Line::from(vec![
                    Span::styled("    │ ", Theme::dim()),
                    Span::styled(clean, code_style),
                ]));
                continue;
            }

            if trimmed.starts_with('#') {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        clean,
                        Style::default()
                            .fg(Theme::GOLD_PRIMARY)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
            } else if trimmed.starts_with("> ") {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        clean,
                        Style::default()
                            .fg(Theme::TEXT_SECONDARY)
                            .add_modifier(Modifier::ITALIC),
                    ),
                ]));
            } else if let Some(rest) = trimmed.strip_prefix("- ").or_else(|| trimmed.strip_prefix("* ")) {
                let indent_len = clean.len().saturating_sub(trimmed.len());
                let mut line_spans = vec![
                    Span::raw("  ".to_string() + &" ".repeat(indent_len)),
                    Span::styled("• ", Style::default().fg(Theme::GOLD_PRIMARY)),
                ];
                line_spans.extend(parse_inline_markdown_spans(
                    rest,
                    Style::default().fg(Theme::TEXT_PRIMARY),
                ));
                lines.push(Line::from(line_spans));
            } else {
                let mut line_spans = vec![Span::raw("  ")];
                line_spans.extend(parse_inline_markdown_spans(
                    &clean,
                    Style::default().fg(Theme::TEXT_PRIMARY),
                ));
                lines.push(Line::from(line_spans));
            }
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

    // 4. Render Tool Calls (smart summary + unified git diff for edit_file + expandable details)
    for tc in &msg.tool_calls {
        render_tool_call_entry(tc, config, &mut lines);
    }

    // Spacer between messages
    lines.push(Line::from(""));
    lines
}

/// Parses inline markdown (`**bold**` and `` `code` ``) into styled Ratatui `Span`s.
fn parse_inline_markdown_spans(text: &str, base_style: Style) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut rest = text;

    while !rest.is_empty() {
        let next_code = rest.find('`');
        let next_bold = rest.find("**");

        enum InlineToken {
            Code(usize),
            Bold(usize),
            Plain,
        }

        let token = match (next_code, next_bold) {
            (Some(c), Some(b)) => {
                if c < b {
                    InlineToken::Code(c)
                } else {
                    InlineToken::Bold(b)
                }
            }
            (Some(c), None) => InlineToken::Code(c),
            (None, Some(b)) => InlineToken::Bold(b),
            (None, None) => InlineToken::Plain,
        };

        match token {
            InlineToken::Code(c) => {
                if c > 0 {
                    spans.push(Span::styled(rest[..c].to_string(), base_style));
                }
                let after = &rest[c + 1..];
                if let Some(end) = after.find('`') {
                    spans.push(Span::styled(
                        after[..end].to_string(),
                        Style::default().fg(Theme::CYAN_ACCENT),
                    ));
                    rest = &after[end + 1..];
                } else {
                    spans.push(Span::styled(rest[c..].to_string(), base_style));
                    break;
                }
            }
            InlineToken::Bold(b) => {
                if b > 0 {
                    spans.push(Span::styled(rest[..b].to_string(), base_style));
                }
                let after = &rest[b + 2..];
                if let Some(end) = after.find("**") {
                    spans.push(Span::styled(
                        after[..end].to_string(),
                        base_style.add_modifier(Modifier::BOLD),
                    ));
                    rest = &after[end + 2..];
                } else {
                    spans.push(Span::styled(rest[b..].to_string(), base_style));
                    break;
                }
            }
            InlineToken::Plain => {
                spans.push(Span::styled(rest.to_string(), base_style));
                break;
            }
        }
    }

    if spans.is_empty() {
        spans.push(Span::styled(String::new(), base_style));
    }
    spans
}

/// Renders unified diff lines (`-` red, `+` green) for `old_text` vs `new_text`, capped by `max_lines`.
fn render_unified_diff_lines(
    old_text: &str,
    new_text: &str,
    tab_size: usize,
    max_lines: usize,
    lines: &mut Vec<Line<'static>>,
) {
    let old_lines: Vec<&str> = old_text.lines().collect();
    let new_lines: Vec<&str> = new_text.lines().collect();
    let mut rendered = 0usize;
    let limit = max_lines.max(4);

    // Trim common prefix & suffix so large edits focus on actual changed lines
    let mut prefix = 0;
    while prefix < old_lines.len()
        && prefix < new_lines.len()
        && old_lines[prefix] == new_lines[prefix]
    {
        prefix += 1;
    }

    let mut old_end = old_lines.len();
    let mut new_end = new_lines.len();
    while old_end > prefix
        && new_end > prefix
        && old_lines[old_end - 1] == new_lines[new_end - 1]
    {
        old_end -= 1;
        new_end -= 1;
    }

    let ctx_start = prefix.saturating_sub(2);
    for line in &old_lines[ctx_start..prefix] {
        if rendered >= limit {
            break;
        }
        lines.push(Line::from(vec![
            Span::styled("    │ ", Theme::dim()),
            Span::styled(
                format!("  {}", sanitize_terminal_line(line, tab_size)),
                Style::default().fg(Theme::TEXT_MUTED),
            ),
        ]));
        rendered += 1;
    }

    for line in &old_lines[prefix..old_end] {
        if rendered >= limit {
            break;
        }
        lines.push(Line::from(vec![
            Span::styled("    │ ", Theme::dim()),
            Span::styled(
                format!("- {}", sanitize_terminal_line(line, tab_size)),
                Style::default().fg(Theme::STATUS_ERROR),
            ),
        ]));
        rendered += 1;
    }

    for line in &new_lines[prefix..new_end] {
        if rendered >= limit {
            break;
        }
        lines.push(Line::from(vec![
            Span::styled("    │ ", Theme::dim()),
            Span::styled(
                format!("+ {}", sanitize_terminal_line(line, tab_size)),
                Style::default().fg(Theme::STATUS_SUCCESS),
            ),
        ]));
        rendered += 1;
    }

    let total_diff = (prefix - ctx_start) + (old_end - prefix) + (new_end - prefix);
    if total_diff > limit {
        lines.push(Line::from(vec![
            Span::styled("    │ ", Theme::dim()),
            Span::styled(
                format!("... (+{} more diff lines)", total_diff - limit),
                Style::default()
                    .fg(Theme::TEXT_MUTED)
                    .add_modifier(Modifier::ITALIC),
            ),
        ]));
    }
}

fn short_file_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

fn format_tool_call_summary(name: &str, args_json: Option<&serde_json::Value>) -> Option<String> {
    let obj = args_json?.as_object()?;
    let file_path = obj
        .get("file_path")
        .or_else(|| obj.get("filepath"))
        .or_else(|| obj.get("path"))
        .and_then(|v| v.as_str());

    match name {
        "workspace_read_file" => file_path.map(|p| format!("Reading {}", short_file_name(p))),
        "workspace_write_file" => file_path.map(|p| {
            let chars = obj
                .get("content")
                .and_then(|v| v.as_str())
                .map(|s| format!(" ({} chars)", s.len()))
                .unwrap_or_default();
            format!("Writing {}{}", short_file_name(p), chars)
        }),
        "workspace_edit_file" => file_path.map(|p| {
            let old_l = obj
                .get("old_text")
                .and_then(|v| v.as_str())
                .map(|s| s.lines().count());
            let new_l = obj
                .get("new_text")
                .and_then(|v| v.as_str())
                .map(|s| s.lines().count());
            match (old_l, new_l) {
                (Some(o), Some(n)) => format!("Editing {} ({} → {} lines)", short_file_name(p), o, n),
                _ => format!("Editing {}", short_file_name(p)),
            }
        }),
        "workspace_run_command" => obj
            .get("command")
            .and_then(|v| v.as_str())
            .map(|cmd| format!("$ {}", cmd.lines().next().unwrap_or(cmd))),
        "workspace_grep" => obj
            .get("query")
            .and_then(|v| v.as_str())
            .map(|q| format!("grep \"{}\"", q)),
        "workspace_list_dir" => file_path.map(|p| format!("ls {}", p)),
        _ => None,
    }
}

fn render_tool_call_entry(
    tc: &crate::agent_view::types::ToolCallInfo,
    config: &MessageRenderConfig,
    lines: &mut Vec<Line<'static>>,
) {
    let (status_icon, status_color) = match tc.status {
        ToolExecutionStatus::Running => ("⏳ Running", Theme::CYAN_ACCENT),
        ToolExecutionStatus::Success => ("✓ Done", Theme::STATUS_SUCCESS),
        ToolExecutionStatus::Failed => ("✗ Failed", Theme::STATUS_ERROR),
        ToolExecutionStatus::PendingApproval => ("! Pending Approval", Theme::STATUS_WARNING),
    };

    let parsed_args = serde_json::from_str::<serde_json::Value>(tc.args_summary.trim()).ok();
    let smart_summary = format_tool_call_summary(&tc.name, parsed_args.as_ref());

    let mut spans = vec![
        Span::styled("    ⚙ ", Style::default().fg(Theme::CYAN_ACCENT)),
        Span::styled(
            tc.name.clone(),
            Style::default()
                .fg(Theme::GOLD_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ),
    ];

    if let Some(summary) = smart_summary {
        spans.push(Span::raw(": "));
        spans.push(Span::styled(
            sanitize_terminal_line(&summary, config.tab_size),
            Style::default().fg(Theme::TEXT_SECONDARY),
        ));
    } else {
        let summary = tc.args_summary.trim();
        if !summary.is_empty() {
            let clean_args = sanitize_terminal_line(&summary.replace(['\r', '\n'], " "), config.tab_size);
            let truncated = if clean_args.chars().count() > 100 && !config.tool_expanded {
                let s: String = clean_args.chars().take(97).collect();
                format!("{}...", s)
            } else {
                clean_args
            };
            spans.push(Span::raw(": "));
            spans.push(Span::styled(
                truncated,
                Style::default().fg(Theme::TEXT_SECONDARY),
            ));
        }
    }

    spans.push(Span::raw(" ("));
    spans.push(Span::styled(
        status_icon,
        Style::default()
            .fg(status_color)
            .add_modifier(Modifier::BOLD),
    ));
    if let Some(ms) = tc.duration_ms {
        spans.push(Span::styled(
            format!(", {}ms", ms),
            Style::default().fg(Theme::TEXT_MUTED),
        ));
    }
    spans.push(Span::raw(")"));
    lines.push(Line::from(spans));

    // Render unified diff for workspace_edit_file if old_text & new_text exist
    if let Some(obj) = parsed_args.as_ref().and_then(|v| v.as_object())
        && let (Some(old_t), Some(new_t)) = (
            obj.get("old_text").and_then(|v| v.as_str()),
            obj.get("new_text").and_then(|v| v.as_str()),
        )
    {
        let diff_limit = if config.tool_expanded {
            config.max_tool_lines
        } else {
            12
        };
        render_unified_diff_lines(old_t, new_t, config.tab_size, diff_limit, lines);
    }

    // If tool is expanded, show extra details (diff_preview, stdout, stderr, output_summary)
    if config.tool_expanded {
        if let Some(diff) = &tc.diff_preview {
            for (idx, dline) in diff.lines().enumerate() {
                if idx >= config.max_tool_lines {
                    lines.push(Line::from(vec![
                        Span::styled("    │ ", Theme::dim()),
                        Span::styled("... (diff truncated)", Theme::text_muted()),
                    ]));
                    break;
                }
                let style = if dline.starts_with('+') && !dline.starts_with("+++") {
                    Style::default().fg(Theme::STATUS_SUCCESS)
                } else if dline.starts_with('-') && !dline.starts_with("---") {
                    Style::default().fg(Theme::STATUS_ERROR)
                } else {
                    Style::default().fg(Theme::TEXT_SECONDARY)
                };
                lines.push(Line::from(vec![
                    Span::styled("    │ ", Theme::dim()),
                    Span::styled(sanitize_terminal_line(dline, config.tab_size), style),
                ]));
            }
        }
        if let Some(out) = tc.stdout.as_deref().or(tc.output_summary.as_deref()) {
            for (idx, oline) in out.lines().enumerate() {
                if idx >= config.max_tool_lines {
                    lines.push(Line::from(vec![
                        Span::styled("    │ ", Theme::dim()),
                        Span::styled("... (output truncated)", Theme::text_muted()),
                    ]));
                    break;
                }
                lines.push(Line::from(vec![
                    Span::styled("    │ ", Theme::dim()),
                    Span::styled(
                        sanitize_terminal_line(oline, config.tab_size),
                        Style::default().fg(Theme::TEXT_SECONDARY),
                    ),
                ]));
            }
        }
        if let Some(err) = &tc.stderr
            && !err.trim().is_empty()
        {
            for (idx, eline) in err.lines().enumerate() {
                if idx >= config.max_tool_lines {
                    break;
                }
                lines.push(Line::from(vec![
                    Span::styled("    │ ", Theme::dim()),
                    Span::styled(
                        sanitize_terminal_line(eline, config.tab_size),
                        Style::default().fg(Theme::STATUS_ERROR),
                    ),
                ]));
            }
        }
    }
}

fn render_tool_output_message(
    effective_content: &str,
    tab_size: usize,
    tool_expanded: bool,
    max_tool_lines: usize,
    lines: &mut Vec<Line<'static>>,
) {
    let trimmed = effective_content.trim();
    let parsed = serde_json::from_str::<serde_json::Value>(trimmed).ok();

    let is_failed = trimmed.starts_with("Error:")
        || trimmed.starts_with("Failed:")
        || trimmed.contains("\"is_error\": true")
        || trimmed.contains("\"is_error\":true")
        || trimmed.to_lowercase().starts_with("error")
        || parsed
            .as_ref()
            .and_then(|v| v.get("status"))
            .and_then(|s| s.as_str())
            == Some("error");

    let mut summary_text: Option<String> = None;
    let mut detail_lines: Vec<String> = Vec::new();

    if let Some(val) = parsed.as_ref()
        && let Some(obj) = val.as_object()
    {
        let file_path = obj
            .get("file_path")
            .or_else(|| obj.get("path"))
            .and_then(|v| v.as_str());

        if obj.get("status").and_then(|s| s.as_str()) == Some("ok")
            && let Some(fp) = file_path
            && (obj.contains_key("bytes_written") || obj.contains_key("new_text_len"))
        {
            let bytes = obj.get("bytes_written").and_then(|v| v.as_u64()).unwrap_or(0);
            let label = if obj.contains_key("new_text_len") || obj.contains_key("old_text_len") {
                "Edited file"
            } else {
                "Updated file"
            };
            summary_text = Some(format!("{}: {} ({} bytes)", label, short_file_name(fp), bytes));
        } else if let Some(exit_code) = obj.get("exit_code").and_then(|v| v.as_i64()) {
            summary_text = Some(format!("Command exited with code {}", exit_code));
            if let Some(stdout) = obj.get("stdout").and_then(|v| v.as_str()) {
                detail_lines.extend(stdout.lines().map(|l| l.to_string()));
            }
            if let Some(stderr) = obj.get("stderr").and_then(|v| v.as_str())
                && !stderr.trim().is_empty()
            {
                detail_lines.extend(stderr.lines().map(|l| format!("stderr: {}", l)));
            }
        } else if let Some(entries) = obj.get("entries").and_then(|v| v.as_array()) {
            summary_text = Some(format!("Listed {} items", entries.len()));
        } else if let Some(err) = obj.get("error").and_then(|v| v.as_str()) {
            summary_text = Some(err.to_string());
        }
    }

    if is_failed {
        let err_msg = summary_text
            .unwrap_or_else(|| trimmed.lines().next().unwrap_or("Execution error").to_string());
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
                format!(": {}", sanitize_terminal_line(&err_msg, tab_size)),
                Style::default().fg(Theme::TEXT_MUTED),
            ),
        ]));
    } else {
        let ok_msg = summary_text.unwrap_or_else(|| "Tool execution completed".to_string());
        lines.push(Line::from(vec![
            Span::styled(
                "    ✓ ",
                Style::default()
                    .fg(Theme::STATUS_SUCCESS)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                sanitize_terminal_line(&ok_msg, tab_size),
                Style::default()
                    .fg(Theme::STATUS_SUCCESS)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
    }

    if tool_expanded {
        let lines_to_show: Vec<String> = if !detail_lines.is_empty() {
            detail_lines
        } else {
            trimmed.lines().map(|l| l.to_string()).collect()
        };

        let limit = max_tool_lines.max(1);
        for (idx, raw) in lines_to_show.iter().enumerate() {
            if idx >= limit {
                lines.push(Line::from(vec![
                    Span::styled("    │ ", Theme::dim()),
                    Span::styled(
                        format!("... (+{} more lines)", lines_to_show.len() - limit),
                        Theme::text_muted(),
                    ),
                ]));
                break;
            }
            lines.push(Line::from(vec![
                Span::styled("    │ ", Theme::dim()),
                Span::styled(
                    sanitize_terminal_line(raw, tab_size),
                    Style::default().fg(Theme::TEXT_SECONDARY),
                ),
            ]));
        }
    }
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

    #[test]
    fn test_inline_markdown_and_bullet_list() {
        let msg = ChatMessage {
            role: MessageRole::Assistant,
            sender_name: "Assistant".into(),
            content: "- **Bold item** with `inline_code`\nNormal **text**".into(),
            timestamp: "now".into(),
            tool_calls: Vec::new(),
            reasoning: None,
        };
        let lines = render_message_standard(&msg);
        let rendered: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
        assert!(rendered.iter().any(|s| s.contains("• Bold item with inline_code")));
        assert!(rendered.iter().any(|s| s.contains("Normal text")));
    }

    #[test]
    fn test_workspace_edit_file_unified_diff_and_tool_expand() {
        let tc = crate::agent_view::types::ToolCallInfo {
            id: "tc_1".into(),
            name: "workspace_edit_file".into(),
            args_summary: r#"{"file_path":"src/main.rs","old_text":"let a = 1;\nlet b = 2;","new_text":"let a = 1;\nlet b = 3;"}"#.into(),
            output_summary: None,
            status: ToolExecutionStatus::Success,
            duration_ms: Some(12),
            stdout: None,
            stderr: None,
            exit_code: None,
            diff_preview: None,
        };
        let msg = ChatMessage {
            role: MessageRole::Assistant,
            sender_name: "Assistant".into(),
            content: String::new(),
            timestamp: "now".into(),
            tool_calls: vec![tc],
            reasoning: None,
        };
        let lines = render_message_standard(&msg);
        let rendered: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
        assert!(rendered.iter().any(|s| s.contains("Editing main.rs (2 → 2 lines)")));
        assert!(rendered.iter().any(|s| s.contains("- let b = 2;")));
        assert!(rendered.iter().any(|s| s.contains("+ let b = 3;")));

        let tool_msg = ChatMessage {
            role: MessageRole::Tool,
            sender_name: "Tool".into(),
            content: r#"{"status":"ok","file_path":"src/main.rs","bytes_written":42,"new_text_len":10}"#.into(),
            timestamp: "now".into(),
            tool_calls: Vec::new(),
            reasoning: None,
        };
        let collapsed = render_message_to_lines(
            &tool_msg,
            &MessageRenderConfig {
                tool_expanded: false,
                ..Default::default()
            },
        );
        assert!(collapsed.iter().any(|l| l.to_string().contains("Edited file: main.rs (42 bytes)")));
        assert!(!collapsed.iter().any(|l| l.to_string().contains("bytes_written")));

        let expanded = render_message_to_lines(
            &tool_msg,
            &MessageRenderConfig {
                tool_expanded: true,
                max_tool_lines: 5,
                ..Default::default()
            },
        );
        assert!(expanded.iter().any(|l| l.to_string().contains("bytes_written")));
    }
}
