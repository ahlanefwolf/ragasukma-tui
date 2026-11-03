use std::time::Instant;
use tokio::sync::mpsc::UnboundedSender;
use crossterm::event::KeyCode;

use crate::agent_view::{AgentFocusArea, AgentWorkspaceState, ChatMessage, ChatSession, MessageRole, ToolCallInfo, ToolExecutionStatus};
use crate::core_bridge::{CoreBridgeCommand, CoreBridgeEvent};
use crate::custom_llm_view::CustomLlmTabState;
use crate::provider_views::ProviderTabState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLineType {
    Context,
    Added,
    Removed,
    Header,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    pub line_type: DiffLineType,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct DiffApprovalState {
    pub tool_call_id: String,
    pub tool_name: String,
    pub file_path: String,
    pub diff_lines: Vec<DiffLine>,
    pub scroll_offset: usize,
}

impl DiffApprovalState {
    pub fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }

    pub fn scroll_down(&mut self) {
        if self.scroll_offset + 1 < self.diff_lines.len() {
            self.scroll_offset += 1;
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConfirmDialogState {
    pub title: String,
    pub prompt: String,
    pub on_confirm_action: Box<Action>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Quit,
    NextTab,
    PrevTab,
    SwitchTab(usize),
    ToggleHelp,
    CloseModal,
    SetStatus(String),

    FocusNext,
    FocusPrev,
    Select,

    ConfirmOk,
    ConfirmCancel,
    ShowError(String),
    ConfirmDelete,
    CancelDelete,

    StartAntigravityOAuth,
    StartCodexOAuth,

    AgentInsertChar(char),
    AgentBackspace,
    AgentClearInput,
    AgentSubmitPrompt,
    AgentClearChat,
    AgentFocusInput,
    AgentUnfocusInput,
    AgentFocusSessions,
    AgentFocusFiles,
    AgentFocusChat,
    AgentFocusNextPanel,
    AgentFocusPrevPanel,
    AgentSelectSessionNext,
    AgentSelectSessionPrev,
    AgentSwitchSession,
    AgentCreateNewSession,
    AgentDeleteSession,
    AgentSelectFileNext,
    AgentSelectFilePrev,
    AgentInsertSelectedFile,
    AgentScanFiles,
    AgentScrollUp,
    AgentScrollDown,
    AgentToggleToolExpand,

    FormChar(char),
    FormBackspace,
    FormNextField,
    FormPrevField,
    FormSubmit,
    CancelForm,

    ShowDiffPreview {
        file_path: String,
        tool_name: String,
        diff_lines: Vec<DiffLine>,
    },
    DiffScrollUp,
    DiffScrollDown,
    DiffApprove,
    DiffReject,
}

pub struct App {
    pub active_tab: usize,
    pub should_quit: bool,
    pub tick_count: u64,
    pub show_help: bool,

    pub status_message: Option<String>,
    pub status_timestamp: Option<Instant>,

    pub agent: AgentWorkspaceState,
    pub antigravity: ProviderTabState,
    pub codex: ProviderTabState,
    pub claude: ProviderTabState,
    pub custom_llm: CustomLlmTabState,

    pub confirm_dialog: Option<ConfirmDialogState>,
    pub diff_modal: Option<DiffApprovalState>,
    pub error_modal: Option<String>,

    pub bridge_cmd_tx: Option<UnboundedSender<CoreBridgeCommand>>,
}

fn current_time_str() -> String {
    let now = std::time::SystemTime::now();
    let d = now.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let secs = d.as_secs();
    let h = (secs / 3600) % 24;
    let m = (secs / 60) % 60;
    let s = secs % 60;
    format!("{:02}:{:02}:{:02}", h, m, s)
}

impl App {
    pub fn new(_root_dir: std::path::PathBuf) -> Self {
        let agent = AgentWorkspaceState::sample();
        let antigravity = ProviderTabState::load_antigravity();
        let codex = ProviderTabState::load_codex();
        let claude = ProviderTabState::load_claude();
        let custom_llm = CustomLlmTabState::new();

        Self {
            active_tab: 0,
            should_quit: false,
            tick_count: 0,
            show_help: false,
            status_message: Some("Ready - Ragakusma Agent TUI".to_string()),
            status_timestamp: Some(Instant::now()),
            agent,
            antigravity,
            codex,
            claude,
            custom_llm,
            confirm_dialog: None,
            diff_modal: None,
            error_modal: None,
            bridge_cmd_tx: None,
        }
    }

    pub fn set_bridge_tx(&mut self, tx: UnboundedSender<CoreBridgeCommand>) {
        self.bridge_cmd_tx = Some(tx);
    }

    pub fn set_status(&mut self, msg: String) {
        self.status_message = Some(msg);
        self.status_timestamp = Some(Instant::now());
    }

    pub fn on_tick(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);
        if let Some(ts) = self.status_timestamp
            && ts.elapsed().as_secs() > 4 {
                self.status_message = None;
                self.status_timestamp = None;
            }
    }

    pub fn update(&mut self, action: Action) {
        match action {
            Action::Quit => self.should_quit = true,
            Action::NextTab => {
                self.active_tab = (self.active_tab + 1) % 5;
            }
            Action::PrevTab => {
                self.active_tab = if self.active_tab == 0 { 4 } else { self.active_tab - 1 };
            }
            Action::SwitchTab(tab) => {
                if tab < 5 {
                    self.active_tab = tab;
                }
            }
            Action::ToggleHelp => {
                self.show_help = !self.show_help;
            }
            Action::CloseModal => {
                self.show_help = false;
                self.confirm_dialog = None;
                self.diff_modal = None;
                self.error_modal = None;
            }
            Action::SetStatus(msg) => {
                self.set_status(msg);
            }
            Action::FocusNext => match self.active_tab {
                0 => self.agent.cycle_focus_next(),
                1 => self.antigravity.next_account(),
                2 => self.codex.next_account(),
                3 => self.claude.next_account(),
                4 => self.custom_llm.select_next(),
                _ => {}
            },
            Action::FocusPrev => match self.active_tab {
                0 => self.agent.cycle_focus_prev(),
                1 => self.antigravity.prev_account(),
                2 => self.codex.prev_account(),
                3 => self.claude.prev_account(),
                4 => self.custom_llm.select_prev(),
                _ => {}
            },
            Action::Select => match self.active_tab {
                4 => {
                    self.custom_llm.toggle_enabled();
                }
                0
                    if self.agent.focus_area == AgentFocusArea::Files => {
                        self.agent.insert_selected_file_to_input();
                    }
                _ => {}
            },
            Action::ConfirmOk => {
                if let Some(dialog) = self.confirm_dialog.take() {
                    self.update(*dialog.on_confirm_action);
                }
            }
            Action::ConfirmCancel => {
                self.confirm_dialog = None;
            }
            Action::ShowError(err) => {
                self.error_modal = Some(err);
            }
            Action::ConfirmDelete => {
                if self.active_tab == 4 {
                    self.custom_llm.confirm_delete();
                }
            }
            Action::CancelDelete => {
                self.confirm_dialog = None;
            }

            Action::StartAntigravityOAuth => {
                let state = format!("ragasukma_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs());
                let redirect_uri = "http://localhost:51121/oauth/callback";
                let auth_url = ragasukma_core::llm::providers::antigravity::oauth::build_auth_url(redirect_uri, &state);
                let _ = std::process::Command::new("xdg-open").arg(&auth_url).spawn();
                self.set_status(format!("OAuth URL dibuka: {}", auth_url));
            }
            Action::StartCodexOAuth => {
                let (code_verifier, code_challenge) = ragasukma_core::llm::providers::codex::oauth::generate_pkce();
                let state = format!("ragasukma_codex_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs());
                ragasukma_core::llm::providers::codex::oauth::store_oauth_state(&state, &code_verifier);
                let redirect_uri = "http://localhost:51122/oauth/callback";
                let auth_url = ragasukma_core::llm::providers::codex::oauth::build_auth_url(redirect_uri, &state, &code_challenge);
                let _ = std::process::Command::new("xdg-open").arg(&auth_url).spawn();
                self.set_status(format!("OAuth URL dibuka: {}", auth_url));
            }

            Action::AgentInsertChar(c) => self.agent.insert_char(c),
            Action::AgentBackspace => self.agent.backspace(),
            Action::AgentClearInput => self.agent.clear_input(),
            Action::AgentSubmitPrompt => {
                if let Some(prompt) = self.agent.submit_prompt() {
                    if let Some(tx) = &self.bridge_cmd_tx {
                        let _ = tx.send(CoreBridgeCommand::ExecuteTurn {
                            prompt: prompt.clone(),
                            model_override: None,
                        });
                    }
                    self.agent.messages.push(ChatMessage {
                        role: MessageRole::User,
                        sender_name: "You".to_string(),
                        content: prompt,
                        timestamp: current_time_str(),
                        tool_calls: Vec::new(),
                        reasoning: None,
                    });
                }
            }
            Action::AgentClearChat => {
                self.agent.clear_chat();
            }
            Action::AgentFocusInput => self.agent.focus_input(),
            Action::AgentUnfocusInput => self.agent.unfocus_input(),
            Action::AgentFocusSessions => self.agent.focus_area = AgentFocusArea::Sessions,
            Action::AgentFocusFiles => self.agent.focus_area = AgentFocusArea::Files,
            Action::AgentFocusChat => self.agent.focus_area = AgentFocusArea::Chat,
            Action::AgentFocusNextPanel => self.agent.cycle_focus_next(),
            Action::AgentFocusPrevPanel => self.agent.cycle_focus_prev(),
            Action::AgentSelectSessionNext => self.agent.select_session_next(),
            Action::AgentSelectSessionPrev => self.agent.select_session_prev(),
            Action::AgentSwitchSession => {
                if let Some(sid) = self.agent.switch_to_selected_session() {
                    if let Some(tx) = &self.bridge_cmd_tx {
                        let _ = tx.send(CoreBridgeCommand::SwitchSession { session_id: sid.clone() });
                    }
                    self.set_status(format!("Beralih ke sesi: {sid}"));
                }
            }
            Action::AgentCreateNewSession => {
                let sid = self.agent.create_new_session();
                if let Some(tx) = &self.bridge_cmd_tx {
                    let _ = tx.send(CoreBridgeCommand::SwitchSession { session_id: sid });
                }
                self.set_status("Sesi baru dibuat".to_string());
            }
            Action::AgentDeleteSession => {
                if let Some(sid) = self.agent.delete_selected_session() {
                    if let Some(tx) = &self.bridge_cmd_tx {
                        let _ = tx.send(CoreBridgeCommand::DeleteSession { session_id: sid });
                    }
                    self.set_status("Sesi berhasil dihapus".to_string());
                }
            }
            Action::AgentSelectFileNext => self.agent.select_file_next(),
            Action::AgentSelectFilePrev => self.agent.select_file_prev(),
            Action::AgentInsertSelectedFile => self.agent.insert_selected_file_to_input(),
            Action::AgentScanFiles => self.agent.scan_workspace_files(),
            Action::AgentScrollUp => self.agent.scroll_up(),
            Action::AgentScrollDown => self.agent.scroll_down(),
            Action::AgentToggleToolExpand => self.agent.toggle_tool_expand(),

            Action::FormChar(c) => self.custom_llm.form_input(c),
            Action::FormBackspace => self.custom_llm.form_backspace(),
            Action::FormNextField => self.custom_llm.form_next_field(),
            Action::FormPrevField => self.custom_llm.form_prev_field(),
            Action::FormSubmit => {
                let _ = self.custom_llm.form_submit();
            }
            Action::CancelForm => self.custom_llm.close_form(),

            Action::ShowDiffPreview { file_path, tool_name, diff_lines } => {
                self.diff_modal = Some(DiffApprovalState {
                    tool_call_id: "diff_preview".to_string(),
                    tool_name,
                    file_path,
                    diff_lines,
                    scroll_offset: 0,
                });
            }
            Action::DiffScrollUp => {
                if let Some(dm) = &mut self.diff_modal {
                    dm.scroll_up();
                }
            }
            Action::DiffScrollDown => {
                if let Some(dm) = &mut self.diff_modal {
                    dm.scroll_down();
                }
            }
            Action::DiffApprove => {
                self.diff_modal = None;
                self.set_status("Patch/Diff Disetujui".to_string());
            }
            Action::DiffReject => {
                self.diff_modal = None;
                self.set_status("Patch/Diff Ditolak".to_string());
            }
        }
    }

    pub fn handle_bridge_event(&mut self, ev: CoreBridgeEvent) {
        match ev {
            CoreBridgeEvent::Connected { session_id, .. } => {
                self.set_status(format!("Terhubung ke session {}", session_id));
            }
            CoreBridgeEvent::SessionsLoaded { sessions, active_id } => {
                self.agent.sessions = sessions
                    .into_iter()
                    .map(|s| ChatSession {
                        id: s.0,
                        title: s.1,
                        created_at: current_time_str(),
                        messages: Vec::new(),
                    })
                    .collect();
                if let Some(idx) = self.agent.sessions.iter().position(|s| s.id == active_id) {
                    self.agent.selected_session_idx = idx;
                }
            }
            CoreBridgeEvent::SessionCreated { id, title } => {
                self.agent.sessions.push(ChatSession {
                    id,
                    title,
                    created_at: current_time_str(),
                    messages: Vec::new(),
                });
                self.agent.selected_session_idx = self.agent.sessions.len().saturating_sub(1);
            }
            CoreBridgeEvent::SessionDeleted { id } => {
                self.agent.sessions.retain(|s| s.id != id);
                if self.agent.selected_session_idx >= self.agent.sessions.len() {
                    self.agent.selected_session_idx = self.agent.sessions.len().saturating_sub(1);
                }
            }
            CoreBridgeEvent::HistoryLoaded { messages, .. } => {
                self.agent.messages = messages
                    .into_iter()
                    .map(|m| ChatMessage {
                        role: match m.role.as_str() {
                            "user" => MessageRole::User,
                            "assistant" => MessageRole::Assistant,
                            _ => MessageRole::System,
                        },
                        sender_name: m.role,
                        content: m.content.unwrap_or_default(),
                        timestamp: current_time_str(),
                        tool_calls: Vec::new(),
                        reasoning: None,
                    })
                    .collect();
            }
            CoreBridgeEvent::StatusChange(msg) => {
                self.set_status(msg);
            }
            CoreBridgeEvent::Delta(delta) => {
                if let Some(last) = self.agent.messages.last_mut()
                    && last.role == MessageRole::Assistant {
                        last.content.push_str(&delta);
                        return;
                    }
                self.agent.messages.push(ChatMessage {
                    role: MessageRole::Assistant,
                    sender_name: "Agent".to_string(),
                    content: delta,
                    timestamp: current_time_str(),
                    tool_calls: Vec::new(),
                    reasoning: None,
                });
            }
            CoreBridgeEvent::ThoughtDelta(thought) => {
                if let Some(last) = self.agent.messages.last_mut()
                    && last.role == MessageRole::Assistant {
                        let r = last.reasoning.get_or_insert_with(String::new);
                        r.push_str(&thought);
                    }
            }
            CoreBridgeEvent::StepInfo { action, details, .. } => {
                let det_str = details.unwrap_or_default();
                self.set_status(format!("[Step] {} - {}", action, det_str));
            }
            CoreBridgeEvent::ToolCall { name, args } => {
                if let Some(last) = self.agent.messages.last_mut() {
                    last.tool_calls.push(ToolCallInfo {
                        id: format!("call_{}", last.tool_calls.len() + 1),
                        name,
                        args_summary: args,
                        output_summary: None,
                        status: ToolExecutionStatus::Success,
                        duration_ms: None,
                    });
                }
            }
            CoreBridgeEvent::TurnCompleted(msg) => {
                self.set_status(format!("Selesai: {}", msg));
            }
            CoreBridgeEvent::QuotaUpdated { provider, total_tokens, .. } => {
                self.set_status(format!("Quota updated: {} total: {}", provider, total_tokens));
            }
            CoreBridgeEvent::Error(err) => {
                self.error_modal = Some(err);
            }
        }
    }

    pub fn handle_key_event(&self, key: crossterm::event::KeyEvent) -> Option<Action> {
        let code = key.code;

        if self.diff_modal.is_some() {
            return match code {
                KeyCode::Esc => Some(Action::CloseModal),
                KeyCode::Up | KeyCode::Char('k') => Some(Action::DiffScrollUp),
                KeyCode::Down | KeyCode::Char('j') => Some(Action::DiffScrollDown),
                KeyCode::Char('y') | KeyCode::Enter => Some(Action::DiffApprove),
                KeyCode::Char('n') => Some(Action::DiffReject),
                _ => None,
            };
        }

        if self.confirm_dialog.is_some() {
            return match code {
                KeyCode::Esc | KeyCode::Char('n') => Some(Action::ConfirmCancel),
                KeyCode::Enter | KeyCode::Char('y') => Some(Action::ConfirmOk),
                _ => None,
            };
        }

        if self.error_modal.is_some() || self.show_help {
            return match code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => Some(Action::CloseModal),
                _ => None,
            };
        }

        if let KeyCode::Char(c @ '1'..='5') = code {
            let tab_idx = (c as usize) - ('1' as usize);
            return Some(Action::SwitchTab(tab_idx));
        }

        match code {
            KeyCode::Tab => return Some(Action::NextTab),
            KeyCode::BackTab => return Some(Action::PrevTab),
            _ => {}
        }

        // Delegate to active view controller
        let view_action = match self.active_tab {
            0 => self.agent.handle_key(key),
            1 => self.antigravity.handle_key(key),
            2 => self.codex.handle_key(key),
            3 => self.claude.handle_key(key),
            4 => self.custom_llm.handle_key(key),
            _ => None,
        };

        if view_action.is_some() {
            return view_action;
        }

        // Fallback global keys
        match code {
            KeyCode::Char('q') => Some(Action::Quit),
            KeyCode::Char('?') => Some(Action::ToggleHelp),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_creation_and_tab_switch() {
        let mut app = App::new(std::path::PathBuf::from("/tmp"));
        assert_eq!(app.active_tab, 0);
        app.update(Action::NextTab);
        assert_eq!(app.active_tab, 1);
        app.update(Action::PrevTab);
        assert_eq!(app.active_tab, 0);
    }

    #[test]
    fn test_diff_approval_state() {
        let mut diff = DiffApprovalState {
            tool_call_id: "test".to_string(),
            tool_name: "patch".to_string(),
            file_path: "src/main.rs".to_string(),
            diff_lines: vec![
                DiffLine { line_type: DiffLineType::Header, content: "@@ header @@".to_string() },
                DiffLine { line_type: DiffLineType::Added, content: "+ new line".to_string() },
            ],
            scroll_offset: 0,
        };
        diff.scroll_down();
        assert_eq!(diff.scroll_offset, 1);
        diff.scroll_up();
        assert_eq!(diff.scroll_offset, 0);
    }

    #[test]
    fn test_footer_hotkeys_navigation_and_tab_switching() {
        use crossterm::event::{KeyEvent, KeyModifiers};

        let mut app = App::new(std::path::PathBuf::from("/tmp"));

        // Test 1-5 jump tab
        for (i, ch) in ['1', '2', '3', '4', '5'].iter().enumerate() {
            let key = KeyEvent::new(KeyCode::Char(*ch), KeyModifiers::NONE);
            let action = app.handle_key_event(key).expect("action expected");
            assert_eq!(action, Action::SwitchTab(i));
            app.update(action);
            assert_eq!(app.active_tab, i);
        }

        // Back to tab 0
        app.update(Action::SwitchTab(0));
        assert_eq!(app.active_tab, 0);

        // Test 'n' creates new session
        let n_key = KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE);
        let action = app.handle_key_event(n_key).expect("action expected");
        assert_eq!(action, Action::AgentCreateNewSession);
        app.update(action);
        assert!(!app.agent.sessions.is_empty());

        // Setup message with tool call to test 'x' toggle
        app.agent.messages.push(crate::agent_view::ChatMessage {
            role: crate::agent_view::MessageRole::Assistant,
            sender_name: "Assistant".to_string(),
            content: "Testing tool".to_string(),
            timestamp: "00:00:00".to_string(),
            tool_calls: vec![crate::agent_view::ToolCallInfo {
                id: "call_1".to_string(),
                name: "workspace_read_file".to_string(),
                args_summary: "{}".to_string(),
                output_summary: None,
                status: crate::agent_view::ToolExecutionStatus::Success,
                duration_ms: Some(10),
            }],
            reasoning: None,
        });

        // Test 'x' toggles tool detail expand
        let x_key = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE);
        let action = app.handle_key_event(x_key).expect("action expected");
        assert_eq!(action, Action::AgentToggleToolExpand);
        let initial_expand = app.agent.expanded_tool_idx;
        app.update(action);
        assert_ne!(app.agent.expanded_tool_idx, initial_expand);

        // Test input focused hotkeys: Ctrl+U, Ctrl+C, Esc
        app.agent.focus_input();
        app.agent.input_buffer = "hello world".to_string();

        let ctrl_u = KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL);
        let action = app.handle_key_event(ctrl_u).expect("action expected");
        assert_eq!(action, Action::AgentClearInput);
        app.update(action);
        assert!(app.agent.input_buffer.is_empty());

        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        let action = app.handle_key_event(ctrl_c).expect("action expected");
        assert_eq!(action, Action::Quit);

        let esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        let action = app.handle_key_event(esc).expect("action expected");
        assert_eq!(action, Action::AgentUnfocusInput);
        app.update(action);
        assert!(!app.agent.is_input_focused);
    }
}
