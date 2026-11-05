use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};

use super::state::App;
use super::types::Action;

impl App {
    pub fn is_text_input_active(&self) -> bool {
        (self.active_tab == 0
            && (self.agent.is_input_focused
                || self.agent.add_folder_modal.is_open
                || self.agent.create_project_modal.is_open
                || self.agent.pending_approval.is_some()
                || self.agent.project_modal.is_open))
            || (self.active_tab == 4 && self.custom_llm.is_form_open)
    }

    pub fn handle_key_event(&self, key: KeyEvent) -> Option<Action> {
        if key.kind == KeyEventKind::Release {
            return None;
        }

        let code = key.code;

        // 1. Modals have exclusive input handling
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

        if self.error_modal.is_some() {
            return match code {
                KeyCode::Esc => Some(Action::CloseModal),
                _ => None,
            };
        }

        // 2. Global exit: Ctrl+C always quits cleanly
        if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL)
            && (code == KeyCode::Char('c') || code == KeyCode::Char('C'))
        {
            return Some(Action::Quit);
        }

        // 3. If text input or form is actively focused, delegate directly to active tab view.
        // Never intercept typing characters (digits, q, ?, Tab, BackTab) as global shortcuts!
        if self.is_text_input_active() {
            return match self.active_tab {
                0 => self.agent.handle_key(key),
                4 => self.custom_llm.handle_key(key),
                _ => None,
            };
        }

        // 4. Delegate to the active tab view controller first.
        // If the active view handles this key (e.g. tree navigation, panel toggling, folder switching),
        // let the view have priority.
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

        // 5. Global fallback shortcuts (when unhandled by active view)
        match code {
            KeyCode::Char('1') => Some(Action::SwitchTab(0)),
            KeyCode::Char('2') => Some(Action::SwitchTab(1)),
            KeyCode::Char('3') => Some(Action::SwitchTab(2)),
            KeyCode::Char('4') => Some(Action::SwitchTab(3)),
            KeyCode::Char('5') => Some(Action::SwitchTab(4)),
            KeyCode::Char('[') => Some(Action::PrevTab),
            KeyCode::Char(']') => Some(Action::NextTab),
            KeyCode::Tab => Some(Action::NextTab),
            KeyCode::BackTab => Some(Action::PrevTab),
            KeyCode::Char('q') => Some(Action::Quit),
            _ => None,
        }
    }

    pub fn handle_paste_event(&self, text: String) -> Option<Action> {
        if self.active_tab == 0 && self.agent.is_input_focused {
            return Some(Action::AgentPaste(text));
        }
        if self.active_tab == 4 && self.custom_llm.is_form_open {
            return Some(Action::FormPaste(text));
        }
        None
    }
}