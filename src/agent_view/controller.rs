use crate::agent_view::states::AgentWorkspaceState;
use crate::agent_view::types::AgentFocusArea;
use crate::app::Action;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

impl AgentWorkspaceState {
    /// Controller key handler for the Agent workspace view.
    pub fn handle_key(&self, key: KeyEvent) -> Option<Action> {
        let code = key.code;

        // Modal: Pending Tool Approval / Diff Review
        if self.pending_approval.is_some() {
            return match code {
                KeyCode::Char('y') | KeyCode::Enter => Some(Action::DiffApprove),
                KeyCode::Char('n') | KeyCode::Esc => Some(Action::DiffReject),
                KeyCode::Up | KeyCode::Char('k') => Some(Action::DiffScrollUp),
                KeyCode::Down | KeyCode::Char('j') => Some(Action::DiffScrollDown),
                _ => None,
            };
        }

        // Modal: Add Folder to Workspace
        if self.add_folder_modal.is_open {
            return match code {
                KeyCode::Esc => Some(Action::AgentCloseAddFolderModal),
                KeyCode::Enter => Some(Action::AgentSubmitAddFolder),
                KeyCode::Tab => Some(Action::AgentBrowseFolderDialog),
                KeyCode::Backspace => Some(Action::AgentAddFolderBackspace),
                KeyCode::Char(c) => Some(Action::AgentAddFolderChar(c)),
                _ => None,
            };
        }

        // Modal: Project Switcher
        if self.project_modal.is_open {
            return match code {
                KeyCode::Esc => Some(Action::AgentCloseProjectModal),
                KeyCode::Down | KeyCode::Char('j') => Some(Action::AgentSelectNextProject),
                KeyCode::Up | KeyCode::Char('k') => Some(Action::AgentSelectPrevProject),
                KeyCode::Enter => Some(Action::AgentSwitchProject),
                KeyCode::Char('n') => Some(Action::AgentOpenCreateProjectModal),
                KeyCode::Char('d') => Some(Action::AgentDeleteProject),
                _ => None,
            };
        }

        // Modal: Create Project
        if self.create_project_modal.is_open {
            return match code {
                KeyCode::Esc => Some(Action::AgentCloseCreateProjectModal),
                KeyCode::Enter => Some(Action::AgentSubmitCreateProject),
                KeyCode::Backspace => Some(Action::AgentCreateProjectBackspace),
                KeyCode::Char(c) => Some(Action::AgentCreateProjectChar(c)),
                _ => None,
            };
        }

        // Modal: Select Model
        if self.model_modal.is_open {
            return match code {
                KeyCode::Esc => Some(Action::AgentCloseModelModal),
                KeyCode::Down | KeyCode::Tab | KeyCode::Char('j') => {
                    Some(Action::AgentSelectNextModalModel)
                }
                KeyCode::Up | KeyCode::BackTab | KeyCode::Char('k') => {
                    Some(Action::AgentSelectPrevModalModel)
                }
                KeyCode::Enter => Some(Action::AgentApplyModalModel),
                KeyCode::Backspace => Some(Action::AgentModelModalBackspace),
                KeyCode::Char(c) => Some(Action::AgentModelModalChar(c)),
                _ => None,
            };
        }

        // When the input textarea is focused
        if self.is_input_focused {
            if key.modifiers.contains(KeyModifiers::ALT) && code == KeyCode::Char('s') {
                if self.is_running {
                    return Some(Action::AgentCancelTurn);
                }
                return None;
            }

            // --- Newline: EXCLUSIVELY Alt+J. Nothing else may produce a newline. ---
            // Checked BEFORE the CONTROL shortcut block below so no other combo can
            // ever reach the newline path.
            {
                let shift = key.modifiers.contains(KeyModifiers::SHIFT);
                let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
                let alt = key.modifiers.contains(KeyModifiers::ALT);

                // Alt+J => the one and only newline.
                if alt
                    && !ctrl
                    && !shift
                    && matches!(code, KeyCode::Char('j') | KeyCode::Char('J'))
                {
                    return Some(Action::AgentInputNewline);
                }

                // Bare Enter = send. Any other Enter/modifier combo is a DEAD KEY:
                // swallowed so it triggers neither newline nor submit.
                if code == KeyCode::Enter {
                    if !shift && !ctrl && !alt {
                        return Some(Action::AgentSubmitPrompt);
                    }
                    return None;
                }
            }

            if key.modifiers.contains(KeyModifiers::CONTROL) {
                return match code {
                    KeyCode::Char('q') => Some(Action::Quit),
                    KeyCode::Char('a') => Some(Action::AgentInputCursorHome),
                    KeyCode::Char('e') => Some(Action::AgentInputCursorEnd),
                    _ => None,
                };
            }

            // Tag query navigation if active
            if self.tag_query.is_some() {
                match code {
                    KeyCode::Esc => return Some(Action::AgentCloseTagQuery),
                    KeyCode::Down => return Some(Action::AgentTagQueryNext),
                    KeyCode::Up => return Some(Action::AgentTagQueryPrev),
                    KeyCode::Tab | KeyCode::Enter => return Some(Action::AgentApplyTag),
                    _ => {}
                }
            }

            return match code {
                KeyCode::Esc => Some(Action::AgentUnfocusInput),
                KeyCode::Enter => Some(Action::AgentSubmitPrompt),
                KeyCode::Backspace => Some(Action::AgentBackspace),
                KeyCode::Delete => Some(Action::AgentInputDelete),
                KeyCode::Left => Some(Action::AgentInputCursorLeft),
                KeyCode::Right => Some(Action::AgentInputCursorRight),
                KeyCode::Up => Some(Action::AgentInputCursorUp),
                KeyCode::Down => Some(Action::AgentInputCursorDown),
                KeyCode::Home => Some(Action::AgentInputCursorHome),
                KeyCode::End => Some(Action::AgentInputCursorEnd),
                KeyCode::Tab => Some(Action::AgentFocusNextPanel),
                KeyCode::BackTab => Some(Action::AgentFocusPrevPanel),
                KeyCode::Char(c) => Some(Action::AgentInsertChar(c)),
                _ => None,
            };
        }

        // Global shortcuts visible in footer (1-5) when not typing in an input.
        // NOTE: bare 'q' is NOT a quit key. Only Ctrl+Q quits (handled above).
        match code {
            KeyCode::Char('1') => return Some(Action::SwitchTab(0)),
            KeyCode::Char('2') => return Some(Action::SwitchTab(1)),
            KeyCode::Char('3') => return Some(Action::SwitchTab(2)),
            KeyCode::Char('4') => return Some(Action::SwitchTab(3)),
            KeyCode::Char('5') => return Some(Action::SwitchTab(4)),
            KeyCode::Char('6') => return Some(Action::SwitchTab(5)),
            _ => {}
        }

        // Alt shortcuts
        if key.modifiers.contains(KeyModifiers::ALT) && code == KeyCode::Char('s') {
            if self.is_running {
                return Some(Action::AgentCancelTurn);
            }
            return None;
        }

        // Control shortcuts
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return match code {
                KeyCode::Char('q') => Some(Action::Quit),
                _ => None,
            };
        }

        // When browsing panels (Folders, Tasks, Files, Chat, Input, Inspector)
        match code {
            KeyCode::Char('i') => Some(Action::AgentFocusInput),
            KeyCode::Tab => Some(Action::AgentFocusNextPanel),
            KeyCode::BackTab => Some(Action::AgentFocusPrevPanel),
            KeyCode::Char('b') => Some(Action::AgentToggleSidebar),
            KeyCode::Char('m') => Some(Action::AgentOpenModelModal),
            KeyCode::Char('f') if self.focus_area == AgentFocusArea::ModelTelemetry => {
                Some(Action::AgentRefreshModels)
            }
            KeyCode::F(5) => Some(Action::AgentRefreshModels),
            KeyCode::Char('s')
                if matches!(
                    self.focus_area,
                    AgentFocusArea::Tasks | AgentFocusArea::Files | AgentFocusArea::McpTools
                ) =>
            {
                Some(Action::AgentSwitchSidebarTab)
            }
            KeyCode::Char('r') => match self.focus_area {
                AgentFocusArea::Files => Some(Action::AgentScanFiles),
                AgentFocusArea::McpTools => Some(Action::AgentRefreshMcpServers),
                AgentFocusArea::ModelTelemetry => Some(Action::AgentRefreshModels),
                _ => Some(Action::AgentToggleReasoning),
            },
            KeyCode::Char('t') if self.focus_area == AgentFocusArea::McpTools => {
                Some(Action::AgentCycleToolFilter)
            }
            KeyCode::Char('n') if self.focus_area == AgentFocusArea::Tasks => {
                Some(Action::AgentNewTask)
            }
            KeyCode::Char(' ') if self.focus_area == AgentFocusArea::McpTools => {
                Some(Action::AgentToggleMcpServer)
            }

            KeyCode::Down | KeyCode::Char('j') => match self.focus_area {
                AgentFocusArea::Tasks => Some(Action::AgentNextTask),
                AgentFocusArea::Files => Some(Action::AgentSelectFileNext),
                AgentFocusArea::Folders => Some(Action::AgentNextFolder),
                AgentFocusArea::Chat | AgentFocusArea::Input => Some(Action::AgentScrollDown),
                AgentFocusArea::McpTools => Some(Action::AgentNextMcpServer),
                AgentFocusArea::ModelTelemetry => None,
            },
            KeyCode::Up | KeyCode::Char('k') => match self.focus_area {
                AgentFocusArea::Tasks => Some(Action::AgentPrevTask),
                AgentFocusArea::Files => Some(Action::AgentSelectFilePrev),
                AgentFocusArea::Folders => Some(Action::AgentPrevFolder),
                AgentFocusArea::Chat | AgentFocusArea::Input => Some(Action::AgentScrollUp),
                AgentFocusArea::McpTools => Some(Action::AgentPrevMcpServer),
                AgentFocusArea::ModelTelemetry => None,
            },
            KeyCode::Enter => match self.focus_area {
                AgentFocusArea::Tasks => Some(Action::AgentSwitchTask),
                AgentFocusArea::Files => Some(Action::AgentToggleFileTreeExpand),
                AgentFocusArea::Input => Some(Action::AgentSubmitPrompt),
                AgentFocusArea::Chat => Some(Action::AgentFocusInput),
                AgentFocusArea::McpTools => Some(Action::AgentToggleMcpServer),
                _ => None,
            },
            KeyCode::Char('a')
                if self.focus_area == AgentFocusArea::Folders
                    || self.focus_area == AgentFocusArea::Files =>
            {
                Some(Action::AgentOpenAddFolderModal)
            }
            KeyCode::Char('o')
                if self.focus_area == AgentFocusArea::Folders
                    || self.focus_area == AgentFocusArea::Files =>
            {
                Some(Action::AgentOpenFolderInFileManager)
            }
            KeyCode::Char('d') => match self.focus_area {
                AgentFocusArea::Folders => Some(Action::AgentRemoveActiveFolder),
                AgentFocusArea::Tasks => Some(Action::AgentDeleteSelectedTask),
                _ => None,
            }
            KeyCode::Char('P') => Some(Action::AgentOpenProjectModal),
            _ => None,
        }
    }
}
