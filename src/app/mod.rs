pub mod bridge;
pub mod controller;
pub mod state;
pub mod types;
pub mod update;

pub use state::App;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

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
                DiffLine {
                    line_type: DiffLineType::Header,
                    content: "@@ header @@".to_string(),
                },
                DiffLine {
                    line_type: DiffLineType::Added,
                    content: "+ new line".to_string(),
                },
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
        let mut app = App::new(std::path::PathBuf::from("/tmp"));
        app.agent.is_input_focused = false;

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
                stdout: None,
                stderr: None,
                exit_code: None,
                diff_preview: None,
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

        // Test input focused hotkeys: Ctrl+C, Esc
        app.agent.focus_input();
        app.agent.input_buffer = "hello world".to_string();

        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        let action = app.handle_key_event(ctrl_c).expect("action expected");
        assert_eq!(action, Action::Quit);

        let esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        let action = app.handle_key_event(esc).expect("action expected");
        assert_eq!(action, Action::AgentUnfocusInput);
        app.update(action);
        assert!(!app.agent.is_input_focused);
    }

    #[test]
    fn test_task_sessions_and_token_gauge() {
        let mut app = App::new(std::path::PathBuf::from("."));
        let breakdown = app.agent.calculate_token_breakdown();
        assert!(breakdown.remaining_tokens() <= breakdown.max_context_limit);
        assert!(!breakdown.is_danger());

        // Test create new task session
        let initial_count = app.agent.sessions.len();
        app.agent.create_new_task(Some("Refactor Agent View".to_string()));
        assert_eq!(app.agent.sessions.len(), initial_count + 1);
        assert_eq!(
            app.agent.sessions.last().map(|s| s.title.as_str()),
            Some("Refactor Agent View")
        );

        // Test switch sidebar tab
        assert_eq!(app.agent.sidebar_tab, crate::agent_view::types::AgentSidebarTab::Tasks);
        app.agent.switch_sidebar_tab();
        assert_eq!(app.agent.sidebar_tab, crate::agent_view::types::AgentSidebarTab::Files);

        // Test delete task session
        app.agent.delete_selected_task();
        assert_eq!(app.agent.sessions.len(), initial_count);
    }

    #[test]
    fn test_no_redundant_dead_or_conflicting_hotkeys() {
        let mut app = App::new(std::path::PathBuf::from("."));

        // 1. Text input in Agent: 'q' and '1'-'5' must type into input, NOT quit or switch tab
        app.agent.focus_input();
        assert!(app.agent.is_input_focused);

        let q_key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        let action_q = app.handle_key_event(q_key).expect("action expected");
        assert_eq!(action_q, Action::AgentInsertChar('q'));

        let digit_key = KeyEvent::new(KeyCode::Char('3'), KeyModifiers::NONE);
        let action_digit = app.handle_key_event(digit_key).expect("action expected");
        assert_eq!(action_digit, Action::AgentInsertChar('3'));

        // Unfocus input
        app.agent.unfocus_input();

        // 2. Bracket keys [ and ] safely switch tabs without conflict
        let bracket_right = KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE);
        let action_next = app.handle_key_event(bracket_right).expect("action expected");
        assert_eq!(action_next, Action::NextTab);
        app.update(action_next);
        assert_eq!(app.active_tab, 1);

        let bracket_left = KeyEvent::new(KeyCode::Char('['), KeyModifiers::NONE);
        let action_prev = app.handle_key_event(bracket_left).expect("action expected");
        assert_eq!(action_prev, Action::PrevTab);
        app.update(action_prev);
        assert_eq!(app.active_tab, 0);

        // 3. Tab 4 (Custom LLM Form): 'q', '1', Tab, BackTab must go to form, NOT global
        app.active_tab = 4;
        app.custom_llm.open_add_form();
        assert!(app.custom_llm.is_form_open);

        let tab_key = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
        let action_tab = app.handle_key_event(tab_key).expect("action expected");
        assert_eq!(action_tab, Action::FormNextField);

        let form_q = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        let action_form_q = app.handle_key_event(form_q).expect("action expected");
        assert_eq!(action_form_q, Action::FormChar('q'));

        let form_digit = KeyEvent::new(KeyCode::Char('8'), KeyModifiers::NONE);
        let action_form_digit = app.handle_key_event(form_digit).expect("action expected");
        assert_eq!(action_form_digit, Action::FormChar('8'));
    }

    #[test]
    fn test_multi_folder_workspace() {
        let mut app = App::new(std::path::PathBuf::from("."));
        assert_eq!(app.agent.workspace_folders.len(), 0);

        // Add valid folder (temp_dir)
        let temp_dir = std::env::temp_dir();
        let add_res = app.agent.add_workspace_folder(temp_dir.clone());
        assert!(add_res.is_ok());
        assert_eq!(app.agent.workspace_folders.len(), 1);

        // Prevent duplicate folder
        let dup_res = app.agent.add_workspace_folder(temp_dir);
        assert!(dup_res.is_err());

        // When added, the newly added folder becomes active
        assert_eq!(app.agent.active_folder_idx, 0);

        // Add a second distinct folder
        let second_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let add_second = app.agent.add_workspace_folder(second_dir);
        assert!(add_second.is_ok());
        assert_eq!(app.agent.workspace_folders.len(), 2);
        assert_eq!(app.agent.active_folder_idx, 1);

        // Select next folder cycles back to index 0
        app.agent.select_next_folder();
        assert_eq!(app.agent.active_folder_idx, 0);

        // Select prev folder cycles back to index 1
        app.agent.select_prev_folder();
        assert_eq!(app.agent.active_folder_idx, 1);

        // Modal open & close
        app.agent.open_add_folder_modal();
        assert!(app.agent.add_folder_modal.is_open);
        app.agent.add_folder_modal.input_path = "/nonexistent_folder_xyz_123".to_string();
        let submit_res = app.agent.submit_add_folder();
        assert!(submit_res.is_err());
        assert!(app.agent.add_folder_modal.validation_message.is_some());

        app.agent.close_add_folder_modal();
        assert!(!app.agent.add_folder_modal.is_open);

        // Remove folder
        let rem_res = app.agent.remove_workspace_folder(1);
        assert!(rem_res.is_ok());
        assert_eq!(app.agent.workspace_folders.len(), 1);

        // Cannot remove the only workspace folder
        let rem_last_res = app.agent.remove_workspace_folder(0);
        assert!(rem_last_res.is_err(), "Expected removing the only workspace folder to fail");
    }

    #[test]
    fn test_project_modal_delete_project() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut app = App::new(std::path::PathBuf::from("."));
        app.agent.projects = vec![
            crate::agent_view::types::ProjectItem {
                id: "p1".to_string(),
                name: "Project 1".to_string(),
                description: None,
                root_path: None,
            },
            crate::agent_view::types::ProjectItem {
                id: "p2".to_string(),
                name: "Project 2".to_string(),
                description: None,
                root_path: None,
            },
        ];
        app.agent.project_modal.is_open = true;
        app.agent.project_modal.selected_idx = 1;

        let d_key = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE);
        let action = app.handle_key_event(d_key).expect("action expected");
        assert_eq!(action, Action::AgentDeleteProject);

        app.update(action);
        assert!(app.status_message.is_some());
        assert_eq!(app.agent.projects.len(), 1);
        assert_eq!(app.agent.projects[0].id, "p1");
        assert_eq!(app.agent.project_modal.selected_idx, 0);
    }
}
