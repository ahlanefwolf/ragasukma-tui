use super::*;

pub(super) fn collect_recursive_tag_items(
    root: &Path,
    current: &Path,
    depth: usize,
    is_multi_root: bool,
    root_name: &str,
    out: &mut Vec<TagItem>,
) {
    if depth > 12 {
        return;
    }

    let entries = match std::fs::read_dir(current) {
        Ok(e) => e,
        Err(_) => return,
    };

    let mut dirs = Vec::new();
    let mut files = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if name.starts_with('.')
            || name == "target"
            || name == "node_modules"
            || name == ".git"
            || name == ".cargo"
        {
            continue;
        }

        if path.is_dir() {
            dirs.push((name, path));
        } else {
            files.push((name, path));
        }
    }

    dirs.sort_by(|a, b| a.0.cmp(&b.0));
    files.sort_by(|a, b| a.0.cmp(&b.0));

    for (name, path) in dirs {
        let rel_path = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let clean_path = if is_multi_root {
            format!("{}/{}/", root_name, rel_path)
        } else {
            format!("{}/", rel_path)
        };

        out.push(TagItem {
            path: clean_path,
            name,
            is_dir: true,
        });

        collect_recursive_tag_items(root, &path, depth + 1, is_multi_root, root_name, out);
    }

    for (name, path) in files {
        let rel_path = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let clean_path = if is_multi_root {
            format!("{}/{}", root_name, rel_path)
        } else {
            rel_path
        };

        out.push(TagItem {
            path: clean_path,
            name,
            is_dir: false,
        });
    }
}

pub(super) fn collect_visible_children(
    dir: &Path,
    depth: usize,
    expanded_dirs: &HashSet<String>,
    nodes: &mut Vec<WorkspaceFileNode>,
    parent_path: Option<String>,
) {
    if depth > 5 {
        return;
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(read_dir) => read_dir,
        Err(_) => return,
    };

    let mut dirs = Vec::new();
    let mut files = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if name.starts_with('.') || name == "target" || name == "node_modules" {
            continue;
        }

        if path.is_dir() {
            dirs.push((name, path));
        } else {
            files.push((name, path));
        }
    }

    dirs.sort_by(|a, b| a.0.cmp(&b.0));
    files.sort_by(|a, b| a.0.cmp(&b.0));

    for (name, path) in dirs {
        let path_str = path.to_string_lossy().to_string();
        let is_expanded = expanded_dirs.contains(&path_str);
        nodes.push(WorkspaceFileNode {
            path: path_str.clone(),
            name,
            is_dir: true,
            depth,
            is_expanded,
            parent_path: parent_path.clone(),
        });

        if is_expanded {
            collect_visible_children(&path, depth + 1, expanded_dirs, nodes, Some(path_str));
        }
    }

    for (name, path) in files {
        nodes.push(WorkspaceFileNode {
            path: path.to_string_lossy().to_string(),
            name,
            is_dir: false,
            depth,
            is_expanded: false,
            parent_path: parent_path.clone(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_limit_eviction() {
        let mut state = AgentWorkspaceState::default();
        for i in 0..(MAX_CHAT_MESSAGES + 50) {
            state.push_message(ChatMessage {
                role: MessageRole::User,
                sender_name: "User".to_string(),
                content: format!("Message {}", i),
                timestamp: "now".to_string(),
                tool_calls: Vec::new(),
                reasoning: None,
            });
        }
        assert_eq!(state.messages.len(), MAX_CHAT_MESSAGES);
        assert_eq!(state.messages[0].content, "Message 50");
        assert_eq!(
            state.messages[MAX_CHAT_MESSAGES - 1].content,
            format!("Message {}", MAX_CHAT_MESSAGES + 49)
        );
    }

    #[test]
    fn test_bracketed_paste_multiline() {
        let mut state = AgentWorkspaceState::default();
        state.insert_char('a');
        let pasted_text = "line 1\r\nline 2\nline 3";
        state.paste(pasted_text);
        assert_eq!(state.input_buffer, "aline 1\nline 2\nline 3");
        assert_eq!(state.cursor_pos, "aline 1\nline 2\nline 3".len());

        // Backspace should delete the last character safely
        state.backspace();
        assert_eq!(state.input_buffer, "aline 1\nline 2\nline ");
        assert_eq!(state.cursor_pos, "aline 1\nline 2\nline ".len());
    }

    #[test]
    fn test_paste_multiline_bash_command_and_backspace() {
        let mut state = AgentWorkspaceState::default();
        let cmd = "cargo clippy --bin ragasukma -- \\\n  -W dead_code \\\n  -W unused \\\n  -W clippy::redundant_clone \\\n  -W clippy::await_holding_lock \\\n  -W clippy::mutex_atomic \\\n  -W clippy::rc_buffer \\\n  -W clippy::panic \\\n  -W clippy::unwrap_used";
        state.paste(cmd);
        let expected = "cargo clippy --bin ragasukma -- -W dead_code -W unused -W clippy::redundant_clone -W clippy::await_holding_lock -W clippy::mutex_atomic -W clippy::rc_buffer -W clippy::panic -W clippy::unwrap_used";
        assert_eq!(state.input_buffer, expected);
        assert_eq!(state.cursor_pos, expected.len());

        // Backspacing 'd' from unwrap_used
        state.backspace();
        assert_eq!(
            state.input_buffer,
            "cargo clippy --bin ragasukma -- -W dead_code -W unused -W clippy::redundant_clone -W clippy::await_holding_lock -W clippy::mutex_atomic -W clippy::rc_buffer -W clippy::panic -W clippy::unwrap_use"
        );
        assert_eq!(state.cursor_pos, expected.len() - 1);
    }

    #[test]
    fn test_paste_with_at_symbol_no_freeze() {
        let mut state = AgentWorkspaceState::default();
        state.paste("const fn = @decorator\nfunction test() {}");
        assert!(state.tag_query.is_none());
        state.backspace();
        assert_eq!(
            state.input_buffer,
            "const fn = @decorator\nfunction test() {"
        );
    }

    #[test]
    fn test_2d_cursor_navigation_and_scroll() {
        let mut state = AgentWorkspaceState::default();
        // Insert 5 lines
        state.paste("line 1\nline 2\nline 3\nline 4\nline 5");
        assert_eq!(state.get_cursor_row_col(), (4, 6)); // Line 4, col 6 ("line 5")
        // Scroll line must be 2 (showing lines 2, 3, 4 in 3-line window)
        assert_eq!(state.input_scroll_line, 2);

        // Move cursor UP
        state.cursor_up();
        assert_eq!(state.get_cursor_row_col(), (3, 6));

        state.cursor_up();
        assert_eq!(state.get_cursor_row_col(), (2, 6));

        state.cursor_up();
        assert_eq!(state.get_cursor_row_col(), (1, 6));
        // Viewport should scroll up to show line 1
        assert_eq!(state.input_scroll_line, 1);

        state.cursor_up();
        assert_eq!(state.get_cursor_row_col(), (0, 6));
        assert_eq!(state.input_scroll_line, 0);

        // Home and End
        state.cursor_home();
        assert_eq!(state.get_cursor_row_col(), (0, 0));

        state.cursor_end();
        assert_eq!(state.get_cursor_row_col(), (0, 6));

        // Move cursor DOWN
        state.cursor_down();
        assert_eq!(state.get_cursor_row_col(), (1, 6));

        // Insert newline
        state.insert_newline();
        assert_eq!(state.get_cursor_row_col(), (2, 0));
    }

    #[test]
    fn test_auto_wrap_to_newline_on_right_edge() {
        let mut state = AgentWorkspaceState::default();
        // Set wrap width to 10
        state.input_wrap_width.set(10);

        // Type 10 characters "0123456789"
        for c in "0123456789".chars() {
            state.insert_char(c);
        }
        // At 10 chars, it hits the right edge boundary and automatically wraps downwards to next line (line 1, col 0)
        assert_eq!(state.get_cursor_row_col(), (1, 0));

        // Type the 11th character
        state.insert_char('A');
        assert_eq!(state.get_cursor_row_col(), (1, 1));
        assert_eq!(state.input_buffer, "0123456789A");

        // Cursor UP moves back to first line
        state.cursor_up();
        assert_eq!(state.get_cursor_row_col(), (0, 1));

        // Cursor DOWN moves back to second line
        state.cursor_down();
        assert_eq!(state.get_cursor_row_col(), (1, 1));
    }

    #[test]
    fn test_tag_query_navigation_and_selection() {
        let mut state = AgentWorkspaceState::default();
        state.is_input_focused = true;
        state.all_workspace_tag_items = vec![
            TagItem {
                name: "file1.rs".into(),
                path: "src/file1.rs".into(),
                is_dir: false,
            },
            TagItem {
                name: "file2.rs".into(),
                path: "src/file2.rs".into(),
                is_dir: false,
            },
        ];

        // Trigger tag query
        state.input_buffer = "check @file".to_string();
        state.cursor_pos = state.input_buffer.len();
        state.check_and_update_tag_query();

        assert!(state.tag_query.is_some());
        let tq = state.tag_query.as_ref().unwrap();
        assert_eq!(tq.query, "file");
        assert_eq!(tq.matched_items.len(), 2);
        assert_eq!(tq.selected_idx, 0);

        // Next selection
        state.select_tag_query_next();
        assert_eq!(state.tag_query.as_ref().unwrap().selected_idx, 1);

        // Prev selection
        state.select_tag_query_prev();
        assert_eq!(state.tag_query.as_ref().unwrap().selected_idx, 0);

        // Apply selected tag
        state.apply_selected_tag();
        assert!(state.tag_query.is_none());
        assert_eq!(state.input_buffer, "check @src/file1.rs ");
    }
}
