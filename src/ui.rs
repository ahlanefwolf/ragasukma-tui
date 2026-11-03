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

    // Fill background with canvas color
    let bg_block = Block::default().style(Style::default().bg(Theme::BG_CANVAS));
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
        render_diff_preview_modal(frame, diff, size);
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

    // Right: Keybindings legend (context-sensitive)
    let key_spans = if app.show_help {
        vec![
            Span::styled(" [Esc/q/?] ", Theme::key_badge()),
            Span::raw("Close Help "),
        ]
    } else if app.active_tab == 0 {
        if app.agent.project_modal.is_open {
            vec![
                Span::styled(" [Enter] ", Theme::key_badge()),
                Span::raw("Switch "),
                Span::styled(" [j/k] ", Theme::key_badge()),
                Span::raw("Nav "),
                Span::styled(" [n] ", Theme::key_badge()),
                Span::raw("New Project "),
                Span::styled(" [Esc] ", Theme::key_badge()),
                Span::raw("Close "),
            ]
        } else if app.agent.create_project_modal.is_open {
            vec![
                Span::styled(" [Enter] ", Theme::key_badge()),
                Span::raw("Create "),
                Span::styled(" [Esc] ", Theme::key_badge()),
                Span::raw("Cancel "),
            ]
        } else if app.agent.add_folder_modal.is_open {
            vec![
                Span::styled(" [Enter] ", Theme::key_badge()),
                Span::raw("Add "),
                Span::styled(" [Tab/F2] ", Theme::key_badge()),
                Span::raw("Browse OS "),
                Span::styled(" [Esc] ", Theme::key_badge()),
                Span::raw("Cancel "),
            ]
        } else if app.agent.is_input_focused {
            vec![
                Span::styled(" [Enter] ", Theme::key_badge()),
                Span::raw("Send "),
                Span::styled(" [Esc] ", Theme::key_badge()),
                Span::raw("Unfocus "),
                Span::styled(" [Ctrl+U] ", Theme::key_badge()),
                Span::raw("Clear "),
                Span::styled(" [Ctrl+C] ", Theme::key_badge()),
                Span::raw("Quit "),
            ]
        } else if area.width < 90 {
            vec![
                Span::styled(" [i] ", Theme::key_badge()),
                Span::raw("Prompt "),
                Span::styled(" [j/k] ", Theme::key_badge()),
                Span::raw("Nav "),
                Span::styled(" [x] ", Theme::key_badge()),
                Span::raw("Tools "),
                Span::styled(" [?] ", Theme::key_badge()),
                Span::raw("Help "),
                Span::styled(" [q] ", Theme::key_badge()),
                Span::raw("Quit "),
            ]
        } else {
            match app.agent.focus_area {
                crate::agent_view::AgentFocusArea::Files => match app.agent.sidebar_tab {
                    crate::agent_view::types::AgentSidebarTab::Tasks => vec![
                        Span::styled(" [P] ", Theme::key_badge()),
                        Span::raw("Project "),
                        Span::styled(" [t] ", Theme::key_badge()),
                        Span::raw("Files "),
                        Span::styled(" [n] ", Theme::key_badge()),
                        Span::raw("New Task "),
                        Span::styled(" [j/k] ", Theme::key_badge()),
                        Span::raw("Select "),
                        Span::styled(" [d] ", Theme::key_badge()),
                        Span::raw("Del "),
                        Span::styled(" [Tab] ", Theme::key_badge()),
                        Span::raw("Focus "),
                        Span::styled(" [1-5] ", Theme::key_badge()),
                        Span::raw("Tabs "),
                        Span::styled(" [?] ", Theme::key_badge()),
                        Span::raw("Help "),
                        Span::styled(" [q] ", Theme::key_badge()),
                        Span::raw("Quit "),
                    ],
                    crate::agent_view::types::AgentSidebarTab::Files => vec![
                        Span::styled(" [P] ", Theme::key_badge()),
                        Span::raw("Project "),
                        Span::styled(" [t] ", Theme::key_badge()),
                        Span::raw("Tasks "),
                        Span::styled(" [j/k] ", Theme::key_badge()),
                        Span::raw("Tree "),
                        Span::styled(" [Enter] ", Theme::key_badge()),
                        Span::raw("Expand "),
                        Span::styled(" [Space] ", Theme::key_badge()),
                        Span::raw("@Mention "),
                        Span::styled(" [a] ", Theme::key_badge()),
                        Span::raw("Add Dir "),
                        Span::styled(" [f/F2] ", Theme::key_badge()),
                        Span::raw("Browse OS "),
                        Span::styled(" [o] ", Theme::key_badge()),
                        Span::raw("Open Mgr "),
                        Span::styled(" [d] ", Theme::key_badge()),
                        Span::raw("Remove "),
                        Span::styled(" [Tab] ", Theme::key_badge()),
                        Span::raw("Focus "),
                        Span::styled(" [1-5] ", Theme::key_badge()),
                        Span::raw("Tabs "),
                        Span::styled(" [?] ", Theme::key_badge()),
                        Span::raw("Help "),
                        Span::styled(" [q] ", Theme::key_badge()),
                        Span::raw("Quit "),
                    ],
                },
                crate::agent_view::AgentFocusArea::Chat => {
                    if app.agent.is_input_focused {
                        vec![
                            Span::styled(" [Enter] ", Theme::key_badge()),
                            Span::raw("Send "),
                            Span::styled(" [@] ", Theme::key_badge()),
                            Span::raw("Attach File "),
                            Span::styled(" [Esc] ", Theme::key_badge()),
                            Span::raw("Unfocus "),
                            Span::styled(" [?] ", Theme::key_badge()),
                            Span::raw("Help "),
                        ]
                    } else {
                        vec![
                            Span::styled(" [i/Enter] ", Theme::key_badge()),
                            Span::raw("Type "),
                            Span::styled(" [j/k] ", Theme::key_badge()),
                            Span::raw("Scroll "),
                            Span::styled(" [x] ", Theme::key_badge()),
                            Span::raw("Cards "),
                            Span::styled(" [t] ", Theme::key_badge()),
                            Span::raw("Filter "),
                            Span::styled(" [c] ", Theme::key_badge()),
                            Span::raw("Clear "),
                            Span::styled(" [Tab] ", Theme::key_badge()),
                            Span::raw("Focus "),
                            Span::styled(" [1-5] ", Theme::key_badge()),
                            Span::raw("Tabs "),
                            Span::styled(" [?] ", Theme::key_badge()),
                            Span::raw("Help "),
                            Span::styled(" [q] ", Theme::key_badge()),
                            Span::raw("Quit "),
                        ]
                    }
                }
                crate::agent_view::AgentFocusArea::Inspector => vec![
                    Span::styled(" [j/k] ", Theme::key_badge()),
                    Span::raw("Scroll "),
                    Span::styled(" [t] ", Theme::key_badge()),
                    Span::raw("Filter "),
                    Span::styled(" [Tab] ", Theme::key_badge()),
                    Span::raw("Focus "),
                    Span::styled(" [1-5] ", Theme::key_badge()),
                    Span::raw("Tabs "),
                    Span::styled(" [?] ", Theme::key_badge()),
                    Span::raw("Help "),
                    Span::styled(" [q] ", Theme::key_badge()),
                    Span::raw("Quit "),
                ],
            }
        }
    } else if app.active_tab == 4 {
        if app.custom_llm.delete_confirm_prefix.is_some() {
            vec![
                Span::styled(" [y/Enter] ", Theme::key_badge()),
                Span::raw("Confirm Delete "),
                Span::styled(" [n/Esc] ", Theme::key_badge()),
                Span::raw("Cancel "),
            ]
        } else if app.custom_llm.is_form_open {
            vec![
                Span::styled(" [Tab/↓] ", Theme::key_badge()),
                Span::raw("Next Field "),
                Span::styled(" [Space] ", Theme::key_badge()),
                Span::raw("Toggle "),
                Span::styled(" [Enter] ", Theme::key_badge()),
                Span::raw("Save "),
                Span::styled(" [Esc] ", Theme::key_badge()),
                Span::raw("Cancel "),
            ]
        } else {
            vec![
                Span::styled(" [j/k] ", Theme::key_badge()),
                Span::raw("Nav "),
                Span::styled(" [Space/Enter] ", Theme::key_badge()),
                Span::raw("Toggle "),
                Span::styled(" [a/n] ", Theme::key_badge()),
                Span::raw("Add "),
                Span::styled(" [e] ", Theme::key_badge()),
                Span::raw("Edit "),
                Span::styled(" [d] ", Theme::key_badge()),
                Span::raw("Del "),
                Span::styled(" [?] ", Theme::key_badge()),
                Span::raw("Help "),
                Span::styled(" [q] ", Theme::key_badge()),
                Span::raw("Quit "),
            ]
        }
    } else {
        let mut spans = vec![
            Span::styled(" [1-5] ", Theme::key_badge()),
            Span::raw("Tabs "),
            Span::styled(" [Tab] ", Theme::key_badge()),
            Span::raw("Nav "),
        ];

        if app.active_tab >= 1 && app.active_tab <= 3 {
            spans.push(Span::styled(" [a] ", Theme::key_badge()));
            spans.push(Span::raw("Add Acct "));
            spans.push(Span::styled(" [j/k] ", Theme::key_badge()));
            spans.push(Span::raw("Accts "));
            spans.push(Span::styled(" [Enter/Space] ", Theme::key_badge()));
            spans.push(Span::raw("Toggle/Active "));
            spans.push(Span::styled(" [r] ", Theme::key_badge()));
            spans.push(Span::raw("Refresh "));
            spans.push(Span::styled(" [d] ", Theme::key_badge()));
            spans.push(Span::raw("Del "));
        }

        spans.push(Span::styled(" [?] ", Theme::key_badge()));
        spans.push(Span::raw("Help "));
        spans.push(Span::styled(" [q] ", Theme::key_badge()));
        spans.push(Span::raw("Quit "));
        spans
    };

    let keys_line = Line::from(key_spans);
    frame.render_widget(
        Paragraph::new(keys_line).alignment(Alignment::Right),
        footer_chunks[1],
    );
}

fn render_help_modal(frame: &mut Frame, app: &App, area: Rect) {
    let popup_area = centered_rect(80, 80, area);

    frame.render_widget(Clear, popup_area);

    let (tab_name, panel_name) = match app.active_tab {
        0 => {
            let panel = if app.agent.pending_approval.is_some() {
                "Diff Review Modal"
            } else if app.agent.add_folder_modal.is_open {
                "Add Folder Modal"
            } else if app.agent.tag_query.is_some() {
                "Attach File Suggestion Popup"
            } else {
                match app.agent.focus_area {
                    crate::agent_view::types::AgentFocusArea::Files => match app.agent.sidebar_tab {
                        crate::agent_view::types::AgentSidebarTab::Tasks => "Sidebar › Tasks",
                        crate::agent_view::types::AgentSidebarTab::Files => "Sidebar › Files",
                    },
                    crate::agent_view::types::AgentFocusArea::Chat => {
                        if app.agent.is_input_focused {
                            "Chat › Message Input (Typing)"
                        } else {
                            "Chat › Message History"
                        }
                    }
                    crate::agent_view::types::AgentFocusArea::Inspector => "Inspector › System & Tokens",
                }
            };
            ("Tab 1: Agent Workspace", panel)
        }
        1 => ("Tab 2: Antigravity Accounts", "Account List"),
        2 => ("Tab 3: Codex Accounts", "Account List"),
        3 => ("Tab 4: Claude Accounts", "Account List"),
        4 => ("Tab 5: Custom LLM Providers", "Provider List"),
        _ => ("Help", "General"),
    };

    let title = format!(" ❓ Help & Shortcuts — {} [{}] ", tab_name, panel_name);
    let help_card = Block::default()
        .title(Span::styled(title, Theme::header_title()))
        .borders(Borders::ALL)
        .border_style(Theme::border_focused())
        .style(Theme::block_base());

    let mut help_text = vec![
        Line::from(vec![
            Span::styled(
                format!("🎯 Active Context ({}) Shortcuts:", panel_name),
                Style::default().fg(Theme::GOLD_PRIMARY).add_modifier(Modifier::BOLD),
            ),
        ]),
    ];

    if app.active_tab == 0 {
        if app.agent.pending_approval.is_some() {
            help_text.push(Line::from(vec![
                Span::styled("   y              : ", Theme::badge_cyan()),
                Span::raw("Approve pending tool diff / changes"),
            ]));
            help_text.push(Line::from(vec![
                Span::styled("   n / Esc        : ", Theme::badge_cyan()),
                Span::raw("Reject pending tool diff"),
            ]));
            help_text.push(Line::from(vec![
                Span::styled("   j / k (↓ / ↑)  : ", Theme::badge_cyan()),
                Span::raw("Scroll diff view up / down"),
            ]));
        } else if app.agent.add_folder_modal.is_open {
            help_text.push(Line::from(vec![
                Span::styled("   Enter          : ", Theme::badge_cyan()),
                Span::raw("Confirm and attach typed folder path to workspace"),
            ]));
            help_text.push(Line::from(vec![
                Span::styled("   Esc            : ", Theme::badge_cyan()),
                Span::raw("Cancel and close modal"),
            ]));
        } else if app.agent.tag_query.is_some() {
            help_text.push(Line::from(vec![
                Span::styled("   j / k (↓ / ↑)  : ", Theme::badge_cyan()),
                Span::raw("Navigate file suggestions"),
            ]));
            help_text.push(Line::from(vec![
                Span::styled("   Enter          : ", Theme::badge_cyan()),
                Span::raw("Insert selected @file mention into prompt"),
            ]));
            help_text.push(Line::from(vec![
                Span::styled("   Esc            : ", Theme::badge_cyan()),
                Span::raw("Dismiss file suggestion popup"),
            ]));
        } else {
            match app.agent.focus_area {
                crate::agent_view::types::AgentFocusArea::Files => match app.agent.sidebar_tab {
                    crate::agent_view::types::AgentSidebarTab::Tasks => {
                        help_text.push(Line::from(vec![
                            Span::styled("   t              : ", Theme::badge_cyan()),
                            Span::raw("Switch sidebar tab to Files view"),
                        ]));
                        help_text.push(Line::from(vec![
                            Span::styled("   n              : ", Theme::badge_cyan()),
                            Span::raw("Create a new task / chat session"),
                        ]));
                        help_text.push(Line::from(vec![
                            Span::styled("   j / k (↓ / ↑)  : ", Theme::badge_cyan()),
                            Span::raw("Select next / previous task session"),
                        ]));
                        help_text.push(Line::from(vec![
                            Span::styled("   d              : ", Theme::badge_cyan()),
                            Span::raw("Delete selected task session"),
                        ]));
                    }
                    crate::agent_view::types::AgentSidebarTab::Files => {
                        help_text.push(Line::from(vec![
                            Span::styled("   t              : ", Theme::badge_cyan()),
                            Span::raw("Switch sidebar tab to Tasks view"),
                        ]));
                        help_text.push(Line::from(vec![
                            Span::styled("   j / k (↓ / ↑)  : ", Theme::badge_cyan()),
                            Span::raw("Navigate directory tree items"),
                        ]));
                        help_text.push(Line::from(vec![
                            Span::styled("   Enter          : ", Theme::badge_cyan()),
                            Span::raw("Expand or collapse folder"),
                        ]));
                        help_text.push(Line::from(vec![
                            Span::styled("   Space          : ", Theme::badge_cyan()),
                            Span::raw("Insert selected file as @mention into chat prompt"),
                        ]));
                        help_text.push(Line::from(vec![
                            Span::styled("   r              : ", Theme::badge_cyan()),
                            Span::raw("Rescan and reload workspace files"),
                        ]));
                        help_text.push(Line::from(vec![
                            Span::styled("   a / +          : ", Theme::badge_cyan()),
                            Span::raw("Attach new folder to workspace"),
                        ]));
                        help_text.push(Line::from(vec![
                            Span::styled("   f / F2         : ", Theme::badge_cyan()),
                            Span::raw("Browse folder using native OS file picker"),
                        ]));
                        help_text.push(Line::from(vec![
                            Span::styled("   o / F3         : ", Theme::badge_cyan()),
                            Span::raw("Open active folder in OS file manager (GUI)"),
                        ]));
                        help_text.push(Line::from(vec![
                            Span::styled("   [ / ]          : ", Theme::badge_cyan()),
                            Span::raw("Switch active workspace folder"),
                        ]));
                        help_text.push(Line::from(vec![
                            Span::styled("   d / -          : ", Theme::badge_cyan()),
                            Span::raw("Remove active folder from workspace"),
                        ]));
                    }
                },
                crate::agent_view::types::AgentFocusArea::Chat => {
                    if app.agent.is_input_focused {
                        help_text.push(Line::from(vec![
                            Span::styled("   Enter          : ", Theme::badge_cyan()),
                            Span::raw("Send prompt instruction to agent"),
                        ]));
                        help_text.push(Line::from(vec![
                            Span::styled("   @              : ", Theme::badge_cyan()),
                            Span::raw("Open file suggestion popup to mention file"),
                        ]));
                        help_text.push(Line::from(vec![
                            Span::styled("   Ctrl+U         : ", Theme::badge_cyan()),
                            Span::raw("Clear input prompt"),
                        ]));
                        help_text.push(Line::from(vec![
                            Span::styled("   Esc            : ", Theme::badge_cyan()),
                            Span::raw("Exit input mode (return to navigation)"),
                        ]));
                    } else {
                        help_text.push(Line::from(vec![
                            Span::styled("   i / Enter      : ", Theme::badge_cyan()),
                            Span::raw("Focus prompt input to type instructions"),
                        ]));
                        help_text.push(Line::from(vec![
                            Span::styled("   j / k (↓ / ↑)  : ", Theme::badge_cyan()),
                            Span::raw("Scroll chat message history"),
                        ]));
                        help_text.push(Line::from(vec![
                            Span::styled("   x              : ", Theme::badge_cyan()),
                            Span::raw("Toggle tool call argument & output cards"),
                        ]));
                        help_text.push(Line::from(vec![
                            Span::styled("   t              : ", Theme::badge_cyan()),
                            Span::raw("Cycle tool call filters (All, Tools Only, Text Only)"),
                        ]));
                        help_text.push(Line::from(vec![
                            Span::styled("   c              : ", Theme::badge_cyan()),
                            Span::raw("Clear chat message history for active task"),
                        ]));
                    }
                }
                crate::agent_view::types::AgentFocusArea::Inspector => {
                    help_text.push(Line::from(vec![
                        Span::styled("   j / k (↓ / ↑)  : ", Theme::badge_cyan()),
                        Span::raw("Scroll token analytics and status logs"),
                    ]));
                    help_text.push(Line::from(vec![
                        Span::styled("   t              : ", Theme::badge_cyan()),
                        Span::raw("Cycle tool call filter mode"),
                    ]));
                }
            }
        }
    } else if app.active_tab >= 1 && app.active_tab <= 3 {
        help_text.push(Line::from(vec![
            Span::styled("   j / k (↓ / ↑)  : ", Theme::badge_cyan()),
            Span::raw("Navigate account list"),
        ]));
        help_text.push(Line::from(vec![
            Span::styled("   Enter          : ", Theme::badge_cyan()),
            Span::raw("Set selected account as active default"),
        ]));
        help_text.push(Line::from(vec![
            Span::styled("   Space / t      : ", Theme::badge_cyan()),
            Span::raw("Toggle account enable / disable state"),
        ]));
        help_text.push(Line::from(vec![
            Span::styled("   n / a          : ", Theme::badge_cyan()),
            Span::raw("Add new account / OAuth login"),
        ]));
        help_text.push(Line::from(vec![
            Span::styled("   d              : ", Theme::badge_cyan()),
            Span::raw("Delete selected account"),
        ]));
        help_text.push(Line::from(vec![
            Span::styled("   r              : ", Theme::badge_cyan()),
            Span::raw("Refresh account quotas and model limits"),
        ]));
    } else if app.active_tab == 4 {
        help_text.push(Line::from(vec![
            Span::styled("   j / k (↓ / ↑)  : ", Theme::badge_cyan()),
            Span::raw("Navigate custom LLM providers"),
        ]));
        help_text.push(Line::from(vec![
            Span::styled("   Space          : ", Theme::badge_cyan()),
            Span::raw("Toggle provider enable / disable state"),
        ]));
        help_text.push(Line::from(vec![
            Span::styled("   n              : ", Theme::badge_cyan()),
            Span::raw("Add a new custom provider"),
        ]));
        help_text.push(Line::from(vec![
            Span::styled("   e              : ", Theme::badge_cyan()),
            Span::raw("Edit selected provider configuration"),
        ]));
        help_text.push(Line::from(vec![
            Span::styled("   d              : ", Theme::badge_cyan()),
            Span::raw("Delete selected provider"),
        ]));
        help_text.push(Line::from(vec![
            Span::styled("   r              : ", Theme::badge_cyan()),
            Span::raw("Open available models list"),
        ]));
    }

    // Section 2: Global Navigation Controls
    help_text.push(Line::raw(""));
    help_text.push(Line::from(vec![
        Span::styled(
            "⚡ Global Navigation & App Controls:",
            Style::default().fg(Theme::CYAN_ACCENT).add_modifier(Modifier::BOLD),
        ),
    ]));
    help_text.push(Line::from(vec![
        Span::styled("   1 - 5          : ", Theme::badge_cyan()),
        Span::raw("Switch tabs (1: Agent, 2: Antigravity, 3: Codex, 4: Claude, 5: Custom LLM)"),
    ]));
    help_text.push(Line::from(vec![
        Span::styled("   Tab            : ", Theme::badge_cyan()),
        Span::raw("Cycle panel focus forward (Sidebar → Chat → Inspector)"),
    ]));
    help_text.push(Line::from(vec![
        Span::styled("   Shift+Tab      : ", Theme::badge_cyan()),
        Span::raw("Cycle panel focus backward"),
    ]));
    help_text.push(Line::from(vec![
        Span::styled("   ? / F1         : ", Theme::badge_cyan()),
        Span::raw("Open / Close this reactive help cheatsheet"),
    ]));
    help_text.push(Line::from(vec![
        Span::styled("   q / Esc        : ", Theme::badge_cyan()),
        Span::raw("Close popup modal or quit application"),
    ]));

    let help_widget = Paragraph::new(help_text)
        .block(help_card)
        .wrap(Wrap { trim: true });
    frame.render_widget(help_widget, popup_area);
}

fn render_confirm_dialog(frame: &mut Frame, dialog: &crate::app::ConfirmDialogState, area: Rect) {
    let popup_area = centered_rect(52, 28, area);
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(Span::styled(
            format!(" ⚠️ {} ", dialog.title),
            Style::default()
                .fg(Theme::STATUS_WARNING)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::STATUS_WARNING))
        .style(Theme::card_base());

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(" ", Style::default()),
            Span::styled(&dialog.prompt, Style::default().fg(Theme::TEXT_PRIMARY)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("   [ Y ] ", Style::default().fg(Theme::STATUS_SUCCESS).add_modifier(Modifier::BOLD)),
            Span::styled("Konfirmasi / Lanjutkan   ", Style::default().fg(Theme::TEXT_SECONDARY)),
            Span::styled("[ N / Esc ] ", Style::default().fg(Theme::STATUS_DANGER).add_modifier(Modifier::BOLD)),
            Span::styled("Batal", Style::default().fg(Theme::TEXT_SECONDARY)),
        ]),
    ];

    let widget = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
    frame.render_widget(widget, popup_area);
}

fn render_error_modal(frame: &mut Frame, error_msg: &str, area: Rect) {
    let popup_area = centered_rect(58, 30, area);
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(Span::styled(
            " ⛔ KESALAHAN SISTEM (SYSTEM ERROR) ",
            Style::default()
                .fg(Theme::STATUS_DANGER)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::STATUS_DANGER))
        .style(Theme::card_base());

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(" Error Detail: ", Style::default().fg(Theme::STATUS_WARNING).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled(format!("  {}", error_msg), Style::default().fg(Theme::TEXT_PRIMARY)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" [ Enter / Esc / Space / q ] ", Theme::key_badge()),
            Span::styled(" Tutup dialog", Style::default().fg(Theme::TEXT_MUTED)),
        ]),
    ];

    let widget = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: true });
    frame.render_widget(widget, popup_area);
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

fn render_diff_preview_modal(
    frame: &mut Frame,
    diff: &crate::app::DiffApprovalState,
    area: Rect,
) {
    let popup_area = centered_rect(80, 80, area);
    frame.render_widget(Clear, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header / File info
            Constraint::Min(5),    // Diff lines
            Constraint::Length(3), // Footer buttons
        ])
        .split(popup_area);

    // 1. Header Box
    let title = format!(" [⚡ DIFF APPROVAL] {} ", diff.tool_name);
    let header_text = vec![
        Line::from(vec![
            Span::styled("Target File: ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::styled(&diff.file_path, Style::default().fg(Theme::GOLD_PRIMARY).add_modifier(Modifier::BOLD)),
        ]),
    ];
    let header_widget = Paragraph::new(header_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Theme::TEXT_SECONDARY))
            .title(title),
    );
    frame.render_widget(header_widget, chunks[0]);

    // 2. Diff Lines Box
    let visible_height = (chunks[1].height as usize).saturating_sub(2);
    let total_lines = diff.diff_lines.len();
    let start_idx = diff.scroll_offset.min(total_lines.saturating_sub(1));
    let end_idx = (start_idx + visible_height).min(total_lines);

    let mut rendered_lines = Vec::new();
    for l in &diff.diff_lines[start_idx..end_idx] {
        let style = match l.line_type {
            crate::app::DiffLineType::Added => Style::default().fg(Theme::STATUS_SUCCESS),
            crate::app::DiffLineType::Removed => Style::default().fg(Theme::STATUS_DANGER),
            crate::app::DiffLineType::Header => Style::default().fg(Theme::GOLD_PRIMARY).add_modifier(Modifier::BOLD),
            crate::app::DiffLineType::Context => Style::default().fg(Theme::TEXT_MUTED),
        };
        rendered_lines.push(Line::from(Span::styled(&l.content, style)));
    }

    let scroll_info = format!(" Lines {}-{}/{} [↑/↓/PgUp/PgDn] ", start_idx + 1, end_idx, total_lines);
    let diff_widget = Paragraph::new(rendered_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Theme::BORDER_GOLD))
            .title(scroll_info),
    );
    frame.render_widget(diff_widget, chunks[1]);

    // 3. Footer Action Buttons
    let footer_text = vec![
        Line::from(vec![
            Span::styled(" [Y / Enter] ", Style::default().fg(Theme::TEXT_PRIMARY).bg(Theme::STATUS_SUCCESS).add_modifier(Modifier::BOLD)),
            Span::raw(" Approve & Apply Patch    "),
            Span::styled(" [N / Esc] ", Style::default().fg(Theme::TEXT_PRIMARY).bg(Theme::STATUS_DANGER).add_modifier(Modifier::BOLD)),
            Span::raw(" Reject Patch"),
        ]),
    ];
    let footer_widget = Paragraph::new(footer_text)
        .alignment(ratatui::layout::Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Theme::BORDER_GOLD)),
        );
    frame.render_widget(footer_widget, chunks[2]);
}
