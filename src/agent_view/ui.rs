use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use super::state::AgentWorkspaceState;
use super::types::{AgentFocusArea, MessageRole, ToolExecutionStatus};
use crate::theme::Theme;

pub fn render_agent_tab(frame: &mut Frame, area: Rect, state: &AgentWorkspaceState) {
    if !state.is_sidebar_visible && !state.is_inspector_visible {
        render_center_chat(frame, area, state);
    } else if !state.is_sidebar_visible {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(area);
        render_center_chat(frame, chunks[0], state);
        render_right_inspector(frame, chunks[1], state);
    } else if !state.is_inspector_visible {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
            .split(area);
        render_left_sidebar(frame, chunks[0], state);
        render_center_chat(frame, chunks[1], state);
    } else {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(22),
                Constraint::Percentage(56),
                Constraint::Percentage(22),
            ])
            .split(area);
        render_left_sidebar(frame, chunks[0], state);
        render_center_chat(frame, chunks[1], state);
        render_right_inspector(frame, chunks[2], state);
    }

    if let Some(ref tq) = state.tag_query {
        render_tag_query_popup(frame, area, tq);
    }

    if state.add_folder_modal.is_open {
        render_add_folder_modal(frame, area, &state.add_folder_modal);
    }
}

fn render_left_sidebar(frame: &mut Frame, area: Rect, state: &AgentWorkspaceState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(8),
            Constraint::Percentage(35),
        ])
        .split(area);

    let is_focused = state.focus_area == AgentFocusArea::Files;

    // Header Workspace Folders Bar (1:1 with web agent multi-folder header)
    let folder_block = Block::default()
        .title(Span::styled(
            format!(" 🗂️ Folders [{}] (a: +Folder, d: -Del) ", state.workspace_folders.len()),
            Theme::badge_gold(),
        ))
        .borders(Borders::ALL)
        .border_style(if is_focused {
            Theme::border_focused()
        } else {
            Theme::border_subtle()
        })
        .style(Theme::block_base());

    let mut folder_spans = Vec::new();
    for (idx, folder) in state.workspace_folders.iter().enumerate() {
        let is_active = idx == state.active_folder_idx;
        let name = folder
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| folder.to_string_lossy().to_string());

        if is_active {
            folder_spans.push(Span::styled(
                format!(" [{}] ", name),
                Style::default().fg(Theme::GOLD_PRIMARY).add_modifier(Modifier::BOLD),
            ));
        } else {
            folder_spans.push(Span::styled(
                format!(" {} ", name),
                Style::default().fg(Theme::TEXT_MUTED),
            ));
        }
    }
    if folder_spans.is_empty() {
        folder_spans.push(Span::styled(" (No attached folders) ", Style::default().fg(Theme::TEXT_MUTED)));
    }
    frame.render_widget(Paragraph::new(Line::from(folder_spans)).block(folder_block), chunks[0]);

    let border_style = if is_focused {
        Theme::border_focused()
    } else {
        Theme::border_subtle()
    };

    let title = if is_focused {
        Span::styled(" 📁 Berkas Workspace [Fokus] ", Theme::badge_gold())
    } else {
        Span::styled(" 📁 Berkas Workspace ", Theme::header_title())
    };

    let file_items: Vec<ListItem> = if state.workspace_files.is_empty() {
        vec![ListItem::new(Line::from(vec![Span::styled(
            " (Tekan r untuk memuat berkas)",
            Style::default().fg(Theme::TEXT_MUTED),
        )]))]
    } else {
        state
            .workspace_files
            .iter()
            .enumerate()
            .map(|(idx, f)| {
                let is_cursor = idx == state.selected_file_idx && is_focused;
                let indent = "  ".repeat(f.depth);
                let icon = if f.is_dir {
                    if f.is_expanded { "📂 " } else { "📁 " }
                } else {
                    let ext = f.name.rsplit('.').next().unwrap_or("");
                    match ext {
                        "rs" => "🦀 ",
                        "toml" => "📦 ",
                        "json" => "🏷️ ",
                        "md" => "📝 ",
                        _ => "📄 ",
                    }
                };

                let cursor_str = if is_cursor { "▶ " } else { "  " };
                let text_style = if is_cursor {
                    Style::default().fg(Theme::GOLD_PRIMARY).add_modifier(Modifier::BOLD)
                } else if f.is_dir {
                    Style::default().fg(Theme::CYAN_ACCENT).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Theme::TEXT_PRIMARY)
                };

                ListItem::new(Line::from(vec![
                    Span::styled(cursor_str, Style::default().fg(Theme::GOLD_PRIMARY)),
                    Span::raw(indent),
                    Span::raw(icon),
                    Span::styled(&f.name, text_style),
                ]))
            })
            .collect()
    };

    let block_files = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(Theme::block_base());
    frame.render_widget(List::new(file_items).block(block_files), chunks[1]);

    // Context Files Pinned
    let context_items: Vec<ListItem> = if state.context_files.is_empty() {
        vec![ListItem::new(Line::from(vec![Span::styled(
            " (Belum ada konteks berkas)",
            Style::default().fg(Theme::TEXT_MUTED),
        )]))]
    } else {
        state
            .context_files
            .iter()
            .enumerate()
            .map(|(idx, cf)| {
                let is_cursor = idx == state.selected_context_file_idx && is_focused;
                let pin_icon = if cf.is_pinned { "📌 " } else { "📎 " };
                let cursor_str = if is_cursor { "▶ " } else { "  " };

                let style = if is_cursor {
                    Style::default().fg(Theme::GOLD_PRIMARY).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Theme::TEXT_SECONDARY)
                };

                ListItem::new(Line::from(vec![
                    Span::styled(cursor_str, Style::default().fg(Theme::GOLD_PRIMARY)),
                    Span::raw(pin_icon),
                    Span::styled(&cf.name, style),
                    Span::styled(
                        format!(" ({}t)", cf.token_count),
                        Style::default().fg(Theme::TEXT_MUTED),
                    ),
                ]))
            })
            .collect()
    };

    let block_context = Block::default()
        .title(Span::styled(" 📌 Konteks Tersemat ", Theme::header_title()))
        .borders(Borders::ALL)
        .border_style(Theme::border_subtle())
        .style(Theme::block_base());
    frame.render_widget(List::new(context_items).block(block_context), chunks[1]);
}

fn render_center_chat(frame: &mut Frame, area: Rect, state: &AgentWorkspaceState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(6), Constraint::Length(3)])
        .split(area);

    let is_chat_focused = state.focus_area == AgentFocusArea::Chat;

    let mut chat_lines = Vec::new();
    if state.messages.is_empty() {
        chat_lines.push(Line::from(vec![
            Span::styled("Selamat datang di ", Style::default().fg(Theme::TEXT_SECONDARY)),
            Span::styled("Ragasukma AI Agent", Style::default().fg(Theme::GOLD_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::styled("! Ketik pesan Anda di bawah atau gunakan @ untuk menyematkan berkas.", Style::default().fg(Theme::TEXT_SECONDARY)),
        ]));
    } else {
        for msg in &state.messages {
            let (badge, badge_style) = match msg.role {
                MessageRole::User => (" Anda ", Theme::badge_info()),
                MessageRole::Assistant => (" Asisten ", Theme::badge_gold()),
                MessageRole::System => (" Sistem ", Theme::badge_gray()),
                MessageRole::Tool => (" Alat ", Theme::badge_cyan()),
            };

            chat_lines.push(Line::from(vec![
                Span::styled(badge, badge_style),
                Span::raw(" "),
                Span::styled(
                    format!("[{}]", msg.timestamp),
                    Style::default().fg(Theme::TEXT_MUTED),
                ),
            ]));

            for line in msg.content.lines() {
                chat_lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(line, Style::default().fg(Theme::TEXT_PRIMARY)),
                ]));
            }

            for tc in &msg.tool_calls {
                let status_icon = match tc.status {
                    ToolExecutionStatus::Running => "⏳ Sedang Berjalan",
                    ToolExecutionStatus::Success => "✓ Selesai",
                    ToolExecutionStatus::Failed => "✗ Gagal",
                    ToolExecutionStatus::PendingApproval => "⚠️ Menunggu Persetujuan",
                };
                let status_color = match tc.status {
                    ToolExecutionStatus::Running => Theme::CYAN_ACCENT,
                    ToolExecutionStatus::Success => Theme::STATUS_SUCCESS,
                    ToolExecutionStatus::Failed => Theme::STATUS_ERROR,
                    ToolExecutionStatus::PendingApproval => Theme::STATUS_WARNING,
                };

                chat_lines.push(Line::from(vec![
                    Span::raw("  🛠️ ["),
                    Span::styled(&tc.name, Style::default().fg(Theme::GOLD_PRIMARY).add_modifier(Modifier::BOLD)),
                    Span::raw("] "),
                    Span::styled(status_icon, Style::default().fg(status_color)),
                ]));
            }
            chat_lines.push(Line::from(""));
        }
    }

    let chat_block = Block::default()
        .title(Span::styled(" 💬 Percakapan Agent ", Theme::header_title()))
        .borders(Borders::ALL)
        .border_style(if is_chat_focused && !state.is_input_focused {
            Theme::border_focused()
        } else {
            Theme::border_subtle()
        })
        .style(Theme::block_base());

    let p = Paragraph::new(chat_lines)
        .block(chat_block)
        .wrap(Wrap { trim: false })
        .scroll((state.scroll_offset as u16, 0));
    frame.render_widget(p, chunks[0]);

    // Input Bar
    let input_border = if state.is_input_focused {
        Theme::border_focused()
    } else {
        Theme::border_subtle()
    };

    let input_title = if state.is_input_focused {
        Span::styled(" Masukkan Pesan (Enter: kirim, @: panggil berkas, Esc: lepas) ", Theme::badge_gold())
    } else {
        Span::styled(" Masukkan Pesan (Tekan i / Enter untuk mengetik) ", Theme::badge_gray())
    };

    let prompt_prefix = Span::styled("❯ ", Style::default().fg(Theme::GOLD_PRIMARY).add_modifier(Modifier::BOLD));
    let input_text = Span::styled(&state.input_buffer, Style::default().fg(Theme::TEXT_PRIMARY));

    let input_block = Block::default()
        .title(input_title)
        .borders(Borders::ALL)
        .border_style(input_border)
        .style(Theme::block_base());

    let input_p = Paragraph::new(Line::from(vec![prompt_prefix, input_text]))
        .block(input_block);
    frame.render_widget(input_p, chunks[1]);
}

fn render_right_inspector(frame: &mut Frame, area: Rect, state: &AgentWorkspaceState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(7), Constraint::Min(8)])
        .split(area);

    let is_focused = state.focus_area == AgentFocusArea::Inspector;

    let breakdown = state.calculate_token_breakdown();
    let usage_pct = (breakdown.usage_ratio() * 100.0) as usize;

    let model_block = Block::default()
        .title(Span::styled(" 🧠 Telemetri Model ", Theme::header_title()))
        .borders(Borders::ALL)
        .border_style(if is_focused {
            Theme::border_focused()
        } else {
            Theme::border_subtle()
        })
        .style(Theme::block_base());

    let lines_model = vec![
        Line::from(vec![
            Span::styled("Model Aktif : ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(&state.active_model, Style::default().fg(Theme::GOLD_PRIMARY).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("Penggunaan  : ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                format!("{} / {} ({}%)", breakdown.total_used(), breakdown.max_context_limit, usage_pct),
                Style::default().fg(if breakdown.is_danger() {
                    Theme::STATUS_ERROR
                } else if breakdown.is_warning() {
                    Theme::STATUS_WARNING
                } else {
                    Theme::CYAN_ACCENT
                }),
            ),
        ]),
        Line::from(vec![
            Span::styled("Sisa Kuota  : ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(
                format!("{} token", breakdown.remaining_tokens()),
                Style::default().fg(Theme::TEXT_PRIMARY),
            ),
        ]),
    ];
    frame.render_widget(Paragraph::new(lines_model).block(model_block), chunks[0]);

    let block_tools = Block::default()
        .title(Span::styled(" 🛠️ Status Alat MCP ", Theme::header_title()))
        .borders(Borders::ALL)
        .border_style(Theme::border_subtle())
        .style(Theme::block_base());

    let tools_lines = vec![
        Line::from(vec![
            Span::styled("● ", Theme::status_success()),
            Span::styled("workspace_read_file", Style::default().fg(Theme::TEXT_PRIMARY)),
        ]),
        Line::from(vec![
            Span::styled("● ", Theme::status_success()),
            Span::styled("workspace_write_file", Style::default().fg(Theme::TEXT_PRIMARY)),
        ]),
        Line::from(vec![
            Span::styled("● ", Theme::status_success()),
            Span::styled("workspace_edit_file", Style::default().fg(Theme::TEXT_PRIMARY)),
        ]),
        Line::from(vec![
            Span::styled("● ", Theme::status_success()),
            Span::styled("workspace_grep", Style::default().fg(Theme::TEXT_PRIMARY)),
        ]),
        Line::from(vec![
            Span::styled("● ", Theme::status_success()),
            Span::styled("workspace_run_cmd", Style::default().fg(Theme::TEXT_PRIMARY)),
        ]),
        Line::from(vec![
            Span::styled("● ", Theme::status_success()),
            Span::styled("quota_fetch_all", Style::default().fg(Theme::TEXT_PRIMARY)),
        ]),
    ];
    frame.render_widget(Paragraph::new(tools_lines).block(block_tools), chunks[1]);
}

fn render_tag_query_popup(frame: &mut Frame, area: Rect, tq: &super::types::FileTagQuery) {
    let popup_area = Rect {
        x: area.x + 10,
        y: area.y + area.height.saturating_sub(12),
        width: area.width.saturating_sub(20).min(50),
        height: 8.min(area.height),
    };

    frame.render_widget(Clear, popup_area);

    let items: Vec<ListItem> = tq
        .matched_files
        .iter()
        .enumerate()
        .map(|(idx, file)| {
            let is_sel = idx == tq.selected_idx;
            let prefix = if is_sel { "▶ " } else { "  " };
            let style = if is_sel {
                Style::default().fg(Theme::GOLD_PRIMARY).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Theme::TEXT_PRIMARY)
            };
            ListItem::new(Line::from(vec![
                Span::styled(prefix, Style::default().fg(Theme::GOLD_PRIMARY)),
                Span::styled(file, style),
            ]))
        })
        .collect();

    let block = Block::default()
        .title(Span::styled(" @ Sertakan Berkas (↑/↓: pilih, Enter: sisip, Esc: batal) ", Theme::badge_gold()))
        .borders(Borders::ALL)
        .border_style(Theme::border_focused())
        .style(Theme::block_surface());

    frame.render_widget(List::new(items).block(block), popup_area);
}

fn render_add_folder_modal(
    frame: &mut Frame,
    area: Rect,
    modal_state: &super::types::AddFolderModalState,
) {
    let width = 64.min(area.width.saturating_sub(4));
    let height = 10.min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let popup_area = Rect {
        x: area.x + x,
        y: area.y + y,
        width,
        height,
    };

    frame.render_widget(Clear, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(2),
            Constraint::Length(2),
        ])
        .margin(1)
        .split(popup_area);

    let block = Block::default()
        .title(Span::styled(
            " ➕ Tambah Folder ke Workspace (Attached Folders) ",
            Theme::badge_gold(),
        ))
        .borders(Borders::ALL)
        .border_style(Theme::border_focused())
        .style(Theme::block_surface());
    frame.render_widget(block, popup_area);

    let input_block = Block::default()
        .title(Span::styled(
            " Path Folder (misal: /path/to/project atau ~/repo) ",
            Style::default().fg(Theme::GOLD_PRIMARY).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Theme::border_focused());
    let input_line = Line::from(vec![
        Span::styled("📂 ", Style::default().fg(Theme::GOLD_PRIMARY)),
        Span::styled(&modal_state.input_path, Style::default().fg(Theme::TEXT_PRIMARY)),
        Span::styled("█", Style::default().fg(Theme::GOLD_PRIMARY)),
    ]);
    frame.render_widget(Paragraph::new(input_line).block(input_block), chunks[0]);

    let feedback_line = if let Some((is_valid, ref msg)) = modal_state.validation_message {
        let style = if is_valid {
            Style::default().fg(Theme::STATUS_SUCCESS)
        } else {
            Style::default().fg(Theme::STATUS_ERROR)
        };
        Line::from(vec![Span::styled(format!("⚠️ {}", msg), style)])
    } else {
        Line::from(vec![Span::styled(
            "Ketik path lokal lalu tekan [Enter] untuk menambahkan",
            Style::default().fg(Theme::TEXT_MUTED),
        )])
    };
    frame.render_widget(Paragraph::new(feedback_line), chunks[1]);

    let footer_line = Line::from(vec![
        Span::styled(" [Enter] ", Theme::key_badge()),
        Span::styled("Tambahkan   ", Style::default().fg(Theme::TEXT_SECONDARY)),
        Span::styled(" [Esc] ", Theme::key_badge()),
        Span::styled("Batal", Style::default().fg(Theme::TEXT_SECONDARY)),
    ]);
    frame.render_widget(Paragraph::new(footer_line), chunks[2]);
}
