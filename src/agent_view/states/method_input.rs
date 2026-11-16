use super::*;

impl AgentWorkspaceState {
    pub fn compute_visual_lines(&self, wrap_width: usize) -> Vec<InputVisualLine> {
        let wrap = wrap_width.max(5);
        if self.input_buffer.is_empty() {
            return vec![InputVisualLine {
                start_byte: 0,
                end_byte: 0,
                is_paragraph_start: true,
            }];
        }

        let mut vlines = Vec::new();
        let mut phys_start = 0;
        let total_len = self.input_buffer.len();

        while phys_start <= total_len {
            let phys_end = match self.input_buffer[phys_start..].find('\n') {
                Some(idx) => phys_start + idx,
                None => total_len,
            };

            let phys_str = &self.input_buffer[phys_start..phys_end];
            let char_count = phys_str.chars().count();

            if char_count == 0 {
                vlines.push(InputVisualLine {
                    start_byte: phys_start,
                    end_byte: phys_end,
                    is_paragraph_start: true,
                });
            } else {
                let mut seg_start_byte = phys_start;
                let mut is_first = true;
                let mut current_seg_chars = 0;

                for (b_offset, _) in phys_str.char_indices() {
                    if current_seg_chars == wrap {
                        let seg_end_byte = phys_start + b_offset;
                        vlines.push(InputVisualLine {
                            start_byte: seg_start_byte,
                            end_byte: seg_end_byte,
                            is_paragraph_start: is_first,
                        });
                        is_first = false;
                        seg_start_byte = seg_end_byte;
                        current_seg_chars = 0;
                    }
                    current_seg_chars += 1;
                }

                // Push last segment of this physical line
                vlines.push(InputVisualLine {
                    start_byte: seg_start_byte,
                    end_byte: phys_end,
                    is_paragraph_start: is_first,
                });

                // If this physical line filled the width completely and cursor is at end,
                // wrap automatically to a new visual line below
                if char_count > 0 && char_count.is_multiple_of(wrap) && self.cursor_pos == phys_end
                {
                    vlines.push(InputVisualLine {
                        start_byte: phys_end,
                        end_byte: phys_end,
                        is_paragraph_start: false,
                    });
                }
            }

            if phys_end == total_len {
                break;
            }
            phys_start = phys_end + 1; // skip '\n'
        }

        if vlines.is_empty() {
            vlines.push(InputVisualLine {
                start_byte: 0,
                end_byte: 0,
                is_paragraph_start: true,
            });
        }

        vlines
    }

    pub fn get_cursor_visual_position(&self, wrap_width: usize) -> (usize, usize) {
        let vlines = self.compute_visual_lines(wrap_width);
        if vlines.is_empty() {
            return (0, 0);
        }
        let safe_pos = self.cursor_pos.min(self.input_buffer.len());

        for (idx, vl) in vlines.iter().enumerate() {
            // When cursor is at the right edge boundary of a wrapped line,
            // automatically prefer the start of the next line below (wrap downwards)
            if safe_pos == vl.end_byte
                && idx + 1 < vlines.len()
                && vlines[idx + 1].start_byte == vl.end_byte
            {
                continue;
            }

            if safe_pos >= vl.start_byte && safe_pos <= vl.end_byte {
                let col = self.input_buffer[vl.start_byte..safe_pos].chars().count();
                return (idx, col);
            }
        }

        let last_idx = vlines.len() - 1;
        let last_vl = &vlines[last_idx];
        let col = self.input_buffer[last_vl.start_byte..safe_pos.max(last_vl.start_byte)]
            .chars()
            .count();
        (last_idx, col)
    }

    pub fn get_cursor_row_col(&self) -> (usize, usize) {
        let wrap = self.input_wrap_width.get().max(5);
        self.get_cursor_visual_position(wrap)
    }

    pub fn adjust_input_scroll(&mut self) {
        let (row, _) = self.get_cursor_row_col();
        if row < self.input_scroll_line {
            self.input_scroll_line = row;
        } else if row >= self.input_scroll_line + 3 {
            self.input_scroll_line = row - 3 + 1;
        }
    }

    pub fn cursor_left(&mut self) {
        self.ensure_valid_cursor();
        if self.cursor_pos > 0 {
            let prev_len = self.input_buffer[..self.cursor_pos]
                .chars()
                .last()
                .map(|c| c.len_utf8())
                .unwrap_or(1);
            self.cursor_pos = self.cursor_pos.saturating_sub(prev_len);
            self.ensure_valid_cursor();
            self.adjust_input_scroll();
            self.check_and_update_tag_query();
        }
    }

    pub fn cursor_right(&mut self) {
        self.ensure_valid_cursor();
        if self.cursor_pos < self.input_buffer.len() {
            let next_len = self.input_buffer[self.cursor_pos..]
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(1);
            self.cursor_pos = (self.cursor_pos + next_len).min(self.input_buffer.len());
            self.ensure_valid_cursor();
            self.adjust_input_scroll();
            self.check_and_update_tag_query();
        }
    }

    pub fn cursor_home(&mut self) {
        self.ensure_valid_cursor();
        let wrap = self.input_wrap_width.get().max(5);
        let vlines = self.compute_visual_lines(wrap);
        let (row, _) = self.get_cursor_visual_position(wrap);
        if let Some(vl) = vlines.get(row) {
            self.cursor_pos = vl.start_byte;
        }
        self.ensure_valid_cursor();
        self.adjust_input_scroll();
        self.check_and_update_tag_query();
    }

    pub fn cursor_end(&mut self) {
        self.ensure_valid_cursor();
        let wrap = self.input_wrap_width.get().max(5);
        let vlines = self.compute_visual_lines(wrap);
        let (row, _) = self.get_cursor_visual_position(wrap);
        if let Some(vl) = vlines.get(row) {
            self.cursor_pos = vl.end_byte;
        }
        self.ensure_valid_cursor();
        self.adjust_input_scroll();
        self.check_and_update_tag_query();
    }

    pub fn cursor_up(&mut self) {
        self.ensure_valid_cursor();
        let wrap = self.input_wrap_width.get().max(5);
        let vlines = self.compute_visual_lines(wrap);
        let (row, col) = self.get_cursor_visual_position(wrap);
        if row == 0 {
            if self.history_prev() {
                return;
            }
            self.cursor_home();
            return;
        }

        if let Some(prev_vl) = vlines.get(row - 1) {
            let prev_str = &self.input_buffer[prev_vl.start_byte..prev_vl.end_byte];
            let prev_char_count = prev_str.chars().count();
            let target_col = col.min(prev_char_count);
            let col_byte_offset: usize = prev_str
                .chars()
                .take(target_col)
                .map(|c| c.len_utf8())
                .sum();
            self.cursor_pos = prev_vl.start_byte + col_byte_offset;
            self.ensure_valid_cursor();
            self.adjust_input_scroll();
            self.check_and_update_tag_query();
        }
    }

    pub fn cursor_down(&mut self) {
        self.ensure_valid_cursor();
        let wrap = self.input_wrap_width.get().max(5);
        let vlines = self.compute_visual_lines(wrap);
        let (row, col) = self.get_cursor_visual_position(wrap);
        if row + 1 >= vlines.len() {
            if self.history_index.is_some() && self.history_next() {
                return;
            }
            self.cursor_end();
            return;
        }

        if let Some(next_vl) = vlines.get(row + 1) {
            let next_str = &self.input_buffer[next_vl.start_byte..next_vl.end_byte];
            let next_char_count = next_str.chars().count();
            let target_col = col.min(next_char_count);
            let col_byte_offset: usize = next_str
                .chars()
                .take(target_col)
                .map(|c| c.len_utf8())
                .sum();
            self.cursor_pos = next_vl.start_byte + col_byte_offset;
            self.ensure_valid_cursor();
            self.adjust_input_scroll();
            self.check_and_update_tag_query();
        }
    }

    pub fn history_prev(&mut self) -> bool {
        if self.history.is_empty() {
            return false;
        }

        if self.history_index.is_none() {
            self.history_draft = self.input_buffer.clone();
        }

        let next_index = match self.history_index {
            None => self.history.len().saturating_sub(1),
            Some(idx) if idx > 0 => idx - 1,
            Some(_) => return false,
        };

        self.history_index = Some(next_index);
        if let Some(entry) = self.history.get(next_index) {
            self.input_buffer = entry.clone();
            self.cursor_pos = self.input_buffer.len();
            self.ensure_valid_cursor();
            self.adjust_input_scroll();
            self.check_and_update_tag_query();
            return true;
        }
        false
    }

    pub fn history_next(&mut self) -> bool {
        let Some(idx) = self.history_index else {
            return false;
        };

        if idx + 1 < self.history.len() {
            let next_index = idx + 1;
            self.history_index = Some(next_index);
            if let Some(entry) = self.history.get(next_index) {
                self.input_buffer = entry.clone();
                self.cursor_pos = self.input_buffer.len();
                self.ensure_valid_cursor();
                self.adjust_input_scroll();
                self.check_and_update_tag_query();
                return true;
            }
        } else {
            self.history_index = None;
            self.input_buffer = self.history_draft.clone();
            self.cursor_pos = self.input_buffer.len();
            self.ensure_valid_cursor();
            self.adjust_input_scroll();
            self.check_and_update_tag_query();
            return true;
        }
        false
    }

    pub fn insert_newline(&mut self) {
        self.insert_char('\n');
        self.adjust_input_scroll();
    }

    pub fn insert_char(&mut self, c: char) {
        self.ensure_valid_cursor();
        let prefix = &self.input_buffer[..self.cursor_pos];
        let suffix = &self.input_buffer[self.cursor_pos..];
        self.input_buffer = format!("{}{}{}", prefix, c, suffix);
        self.cursor_pos += c.len_utf8();
        self.adjust_input_scroll();
        self.check_and_update_tag_query();
    }

    pub fn sanitize_pasted_text(text: &str) -> String {
        // Strip trailing CRLF/LF that is commonly copied from terminal output/commands in Windows
        let text = text.trim_end_matches(&['\r', '\n'][..]);
        let text = text.replace('\r', "");
        let raw_lines: Vec<&str> = text.split('\n').collect();
        if raw_lines.len() <= 1 {
            return text.replace('\t', "    ");
        }

        let mut lines = Vec::new();
        let mut i = 0;
        while i < raw_lines.len() {
            let line = raw_lines[i];
            let trimmed_end = line.trim_end();
            if let Some(stripped) = trimmed_end.strip_suffix('\\') {
                // Shell line continuation: merge with next line
                let mut merged = stripped.trim_end().to_string();
                i += 1;
                while i < raw_lines.len() {
                    let next_line = raw_lines[i].trim();
                    if let Some(next_stripped) = next_line.strip_suffix('\\') {
                        merged.push(' ');
                        merged.push_str(next_stripped.trim_end());
                        i += 1;
                    } else {
                        if !next_line.is_empty() {
                            merged.push(' ');
                            merged.push_str(next_line);
                        }
                        break;
                    }
                }
                lines.push(merged);
            } else {
                lines.push(line.replace('\t', "    "));
            }
            i += 1;
        }

        lines.join("\n")
    }

    pub fn paste(&mut self, text: &str) {
        self.ensure_valid_cursor();
        let sanitized = Self::sanitize_pasted_text(text);
        let prefix = &self.input_buffer[..self.cursor_pos];
        let suffix = &self.input_buffer[self.cursor_pos..];
        self.input_buffer = format!("{}{}{}", prefix, sanitized, suffix);
        self.cursor_pos += sanitized.len();
        self.adjust_input_scroll();
        self.check_and_update_tag_query();
    }

    pub fn backspace(&mut self) {
        self.ensure_valid_cursor();
        if self.cursor_pos > 0 {
            let prev_len = self.input_buffer[..self.cursor_pos]
                .chars()
                .last()
                .map(|c| c.len_utf8())
                .unwrap_or(0);
            if prev_len > 0 && prev_len <= self.cursor_pos {
                let start = self.cursor_pos - prev_len;
                let end = self.cursor_pos;
                if self.input_buffer.is_char_boundary(start)
                    && self.input_buffer.is_char_boundary(end)
                {
                    self.input_buffer.replace_range(start..end, "");
                    self.cursor_pos = start;
                }
            }
            self.adjust_input_scroll();
            self.check_and_update_tag_query();
        }
    }

    pub fn delete_forward(&mut self) {
        self.ensure_valid_cursor();
        if self.cursor_pos < self.input_buffer.len() {
            let next_len = self.input_buffer[self.cursor_pos..]
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(0);
            if next_len > 0 {
                let start = self.cursor_pos;
                let end = self.cursor_pos + next_len;
                if self.input_buffer.is_char_boundary(start)
                    && self.input_buffer.is_char_boundary(end)
                {
                    self.input_buffer.replace_range(start..end, "");
                }
            }
            self.adjust_input_scroll();
            self.check_and_update_tag_query();
        }
    }

    /// Returns the index of the streaming assistant bubble that incoming deltas
    /// should target. Prefers the last assistant message that has no tool calls yet
    /// (the "current phase" bubble); if everything has tool calls, falls back to
    /// creating a fresh bubble. This prevents thoughts/content from leaking into a
    /// bubble that already produced finished tool cards.
    fn streaming_target_index(&self) -> Option<usize> {
        // Only the newest message can be the open phase. If it is an assistant
        // bubble without tool calls, stream into it; otherwise a new bubble is
        // created below instead of appending into closed/foreign bubbles.
        let idx = self.messages.len().checked_sub(1)?;
        let msg = &self.messages[idx];
        if msg.role == MessageRole::Assistant && msg.tool_calls.is_empty() {
            Some(idx)
        } else {
            None
        }
    }

    /// Push a fresh empty assistant bubble and return its index.
    fn push_assistant_bubble(&mut self) -> usize {
        self.messages.push(ChatMessage {
            role: MessageRole::Assistant,
            sender_name: "Agent".to_string(),
            content: String::new(),
            timestamp: "now".to_string(),
            tool_calls: Vec::new(),
            reasoning: None,
        });
        self.enforce_message_limit();
        self.messages.len() - 1
    }

    pub fn append_chunk(&mut self, chunk: &str) {
        // Only auto-follow when the user has not scrolled away during streaming.
        if self.follow_stream {
            self.auto_scroll = true;
        }
        let idx = match self.streaming_target_index() {
            Some(idx) => idx,
            None => self.push_assistant_bubble(),
        };
        if let Some(msg) = self.messages.get_mut(idx) {
            msg.content.push_str(chunk);
        }
    }

    pub fn append_thought(&mut self, thought: &str) {
        if self.follow_stream {
            self.auto_scroll = true;
        }
        let idx = match self.streaming_target_index() {
            Some(idx) => idx,
            None => self.push_assistant_bubble(),
        };
        if let Some(msg) = self.messages.get_mut(idx) {
            match &mut msg.reasoning {
                Some(r) => r.push_str(thought),
                None => msg.reasoning = Some(thought.to_string()),
            }
        }
    }

    pub fn finish_turn(&mut self) {
        self.is_running = false;
        self.active_turn_id = None;
        if let Some(active_sid) = &self.active_session_id
            && let Some(s) = self.sessions.iter_mut().find(|s| &s.id == active_sid)
        {
            s.is_running = false;
        }
    }

    pub fn fail_turn(&mut self, _err: String) {
        self.is_running = false;
        self.active_turn_id = None;
        if let Some(active_sid) = &self.active_session_id
            && let Some(s) = self.sessions.iter_mut().find(|s| &s.id == active_sid)
        {
            s.is_running = false;
        }
    }

    pub fn word_left(&mut self) {
        self.ensure_valid_cursor();
        if self.cursor_pos == 0 {
            return;
        }

        let prefix = &self.input_buffer[..self.cursor_pos];
        let mut chars = prefix.char_indices().rev().peekable();

        while let Some(&(_, c)) = chars.peek() {
            if c.is_whitespace() {
                chars.next();
            } else {
                break;
            }
        }

        let mut target_byte = 0;
        for (idx, c) in chars {
            if c.is_whitespace() {
                target_byte = idx + c.len_utf8();
                break;
            }
        }

        self.cursor_pos = target_byte;
        self.ensure_valid_cursor();
    }

    pub fn word_right(&mut self) {
        self.ensure_valid_cursor();
        if self.cursor_pos >= self.input_buffer.len() {
            return;
        }

        let suffix = &self.input_buffer[self.cursor_pos..];
        let mut chars = suffix.char_indices().peekable();

        while let Some(&(_, c)) = chars.peek() {
            if c.is_whitespace() {
                chars.next();
            } else {
                break;
            }
        }

        let mut target_offset = suffix.len();
        for (idx, c) in chars {
            if c.is_whitespace() {
                target_offset = idx;
                break;
            }
        }

        self.cursor_pos += target_offset;
        self.ensure_valid_cursor();
    }

    pub fn delete_word_backward(&mut self) {
        self.ensure_valid_cursor();
        if self.cursor_pos == 0 {
            return;
        }

        let prefix = &self.input_buffer[..self.cursor_pos];
        let mut chars = prefix.char_indices().rev().peekable();

        // Skip trailing spaces
        while let Some(&(_, c)) = chars.peek() {
            if c.is_whitespace() {
                chars.next();
            } else {
                break;
            }
        }

        // Consume non-whitespace word characters
        let mut target_byte = 0;
        for (idx, c) in chars {
            if c.is_whitespace() {
                target_byte = idx + c.len_utf8();
                break;
            }
        }

        if self.input_buffer.is_char_boundary(target_byte)
            && self.input_buffer.is_char_boundary(self.cursor_pos)
        {
            self.input_buffer.replace_range(target_byte..self.cursor_pos, "");
            self.cursor_pos = target_byte;
            self.adjust_input_scroll();
            self.check_and_update_tag_query();
        }
    }

    pub fn clear_to_start(&mut self) {
        self.ensure_valid_cursor();
        if self.cursor_pos > 0 && self.input_buffer.is_char_boundary(self.cursor_pos) {
            let line_start = self.input_buffer[..self.cursor_pos]
                .rfind('\n')
                .map(|idx| idx + 1)
                .unwrap_or(0);
            self.input_buffer.replace_range(line_start..self.cursor_pos, "");
            self.cursor_pos = line_start;
            self.adjust_input_scroll();
            self.check_and_update_tag_query();
        }
    }

    pub fn clear_to_end(&mut self) {
        self.ensure_valid_cursor();
        if self.cursor_pos < self.input_buffer.len()
            && self.input_buffer.is_char_boundary(self.cursor_pos)
        {
            let line_end = self.input_buffer[self.cursor_pos..]
                .find('\n')
                .map(|idx| self.cursor_pos + idx)
                .unwrap_or(self.input_buffer.len());
            self.input_buffer.replace_range(self.cursor_pos..line_end, "");
            self.adjust_input_scroll();
            self.check_and_update_tag_query();
        }
    }

    pub fn submit_prompt(&mut self) -> Option<String> {
        let trimmed = self.input_buffer.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            // Save prompt to history, deduplicating consecutive identical entries
            if self.history.last().map(|s| s.as_str()) != Some(&trimmed) {
                self.history.push(trimmed.clone());
            }
            self.history_index = None;
            self.history_draft.clear();

            self.input_buffer.clear();
            self.cursor_pos = 0;
            self.tag_query = None;
            self.scroll_offset = 0;
            self.auto_scroll = true;
            self.chat_visible_limit = crate::agent_view::ui::message_renderer::DEFAULT_CHAT_MESSAGE_LIMIT;
            Some(trimmed)
        }
    }

    pub fn focus_input(&mut self) {
        self.is_input_focused = true;
        self.focus_area = AgentFocusArea::Input;
    }

    pub fn unfocus_input(&mut self) {
        self.is_input_focused = false;
        if self.focus_area == AgentFocusArea::Input {
            self.focus_area = AgentFocusArea::Chat;
        }
    }

    pub fn cycle_focus_next(&mut self) {
        let mut order = Vec::new();
        if self.is_sidebar_visible {
            match self.sidebar_tab {
                AgentSidebarTab::Tasks => order.push(AgentFocusArea::Tasks),
                AgentSidebarTab::Files => order.push(AgentFocusArea::Files),
                AgentSidebarTab::Mcp => order.push(AgentFocusArea::McpTools),
            }
            order.push(AgentFocusArea::Folders);
        }
        order.push(AgentFocusArea::Chat);
        order.push(AgentFocusArea::Input);
        if self.is_inspector_visible {
            order.push(AgentFocusArea::ModelTelemetry);
            if !self.is_sidebar_visible || self.sidebar_tab != AgentSidebarTab::Mcp {
                order.push(AgentFocusArea::McpTools);
            }
        }

        if let Some(pos) = order.iter().position(|&f| f == self.focus_area) {
            let next_pos = (pos + 1) % order.len();
            self.focus_area = order[next_pos];
        } else {
            self.focus_area = order.first().copied().unwrap_or(AgentFocusArea::Chat);
        }

        self.is_input_focused = self.focus_area == AgentFocusArea::Input;
    }

    pub fn cycle_focus_prev(&mut self) {
        let mut order = Vec::new();
        if self.is_sidebar_visible {
            match self.sidebar_tab {
                AgentSidebarTab::Tasks => order.push(AgentFocusArea::Tasks),
                AgentSidebarTab::Files => order.push(AgentFocusArea::Files),
                AgentSidebarTab::Mcp => order.push(AgentFocusArea::McpTools),
            }
            order.push(AgentFocusArea::Folders);
        }
        order.push(AgentFocusArea::Chat);
        order.push(AgentFocusArea::Input);
        if self.is_inspector_visible {
            order.push(AgentFocusArea::ModelTelemetry);
            if !self.is_sidebar_visible || self.sidebar_tab != AgentSidebarTab::Mcp {
                order.push(AgentFocusArea::McpTools);
            }
        }

        if let Some(pos) = order.iter().position(|&f| f == self.focus_area) {
            let prev_pos = if pos == 0 { order.len() - 1 } else { pos - 1 };
            self.focus_area = order[prev_pos];
        } else {
            self.focus_area = order.first().copied().unwrap_or(AgentFocusArea::Chat);
        }

        self.is_input_focused = self.focus_area == AgentFocusArea::Input;
    }

    pub fn scroll_up(&mut self) {
        let max = self.chat_max_scroll.get();
        // If user reaches the top of the currently rendered messages, lazy load older messages (limit += 25)
        if self.scroll_offset >= max && self.chat_visible_limit < self.messages.len() {
            self.chat_visible_limit = (self.chat_visible_limit
                + crate::agent_view::ui::message_renderer::CHAT_LAZY_LOAD_STEP)
                .min(self.messages.len());
        }
        let max = self.chat_max_scroll.get();
        self.scroll_offset = (self.scroll_offset.saturating_add(1)).min(max);
        self.auto_scroll = false;
        // User intentionally scrolled up: stop auto-following the stream so new
        // deltas do not yank the viewport back to the bottom.
        self.follow_stream = false;
    }

    pub fn scroll_down(&mut self) {
        let max = self.chat_max_scroll.get();
        let current = self.scroll_offset.min(max);
        self.scroll_offset = current.saturating_sub(1);
        if self.scroll_offset == 0 {
            self.auto_scroll = true;
            // Back at the bottom: resume following the live stream.
            self.follow_stream = true;
        }
    }

    pub fn scroll_to_top(&mut self) {
        self.chat_visible_limit = self.messages.len();
        self.scroll_offset = self.chat_max_scroll.get();
        self.auto_scroll = false;
        self.follow_stream = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_append_thought_does_not_leak_into_phase_with_tool_calls() {
        let mut state = AgentWorkspaceState::default();
        state.start_turn();

        // Phase 1: streaming text + a finished tool call attached to it.
        state.append_chunk("Let me check that.");
        {
            let last = state.messages.last_mut().unwrap();
            last.tool_calls.push(crate::agent_view::types::ToolCallInfo {
                id: "t1".to_string(),
                name: "read_file".to_string(),
                args_summary: "{}".to_string(),
                output_summary: Some("ok".to_string()),
                status: crate::agent_view::types::ToolExecutionStatus::Success,
                duration_ms: None,
                stdout: None,
                stderr: None,
                exit_code: None,
                diff_preview: None,
            });
        }

        // Phase 2: more reasoning arrives after the tool finished. It must NOT be
        // appended to the bubble that already owns the tool card; a new bubble is
        // created instead.
        state.append_thought("Now thinking further.");

        assert_eq!(state.messages.len(), 2, "expected a fresh bubble for phase 2");
        assert!(state.messages[0].reasoning.is_none());
        assert_eq!(
            state.messages[1].reasoning.as_deref(),
            Some("Now thinking further.")
        );
    }

    #[test]
    fn test_stream_delta_respects_manual_scroll_away() {
        let mut state = AgentWorkspaceState::default();
        state.start_turn();
        state.append_chunk("hello");

        // User scrolls up mid-stream -> we must stop auto-following.
        state.scroll_up();
        assert!(!state.follow_stream);
        assert!(!state.auto_scroll);

        // Incoming delta must not force the viewport back to bottom.
        state.append_chunk(" world");
        assert!(!state.auto_scroll, "delta must not override manual scroll");
        assert_eq!(state.messages.last().unwrap().content, "hello world");

        // Returning to the bottom resumes following.
        state.scroll_to_bottom();
        assert!(state.follow_stream);
        state.append_chunk("!");
        assert!(state.auto_scroll);
    }

    #[test]
    fn test_prompt_history_submission_and_navigation() {
        let mut state = AgentWorkspaceState::default();

        state.input_buffer = "first prompt".to_string();
        state.cursor_pos = state.input_buffer.len();
        assert_eq!(state.submit_prompt(), Some("first prompt".to_string()));

        // Consecutive duplicate is ignored in history
        state.input_buffer = "first prompt".to_string();
        state.cursor_pos = state.input_buffer.len();
        assert_eq!(state.submit_prompt(), Some("first prompt".to_string()));
        assert_eq!(state.history.len(), 1);

        state.input_buffer = "second prompt".to_string();
        state.cursor_pos = state.input_buffer.len();
        assert_eq!(state.submit_prompt(), Some("second prompt".to_string()));
        assert_eq!(state.history.len(), 2);

        // User starts typing a draft prompt
        state.input_buffer = "my draft".to_string();
        state.cursor_pos = state.input_buffer.len();

        // Up arrow: recall second prompt (most recent)
        state.cursor_up();
        assert_eq!(state.input_buffer, "second prompt");
        assert_eq!(state.history_index, Some(1));

        // Up arrow again: recall first prompt
        state.cursor_up();
        assert_eq!(state.input_buffer, "first prompt");
        assert_eq!(state.history_index, Some(0));

        // Down arrow: recall second prompt
        state.cursor_down();
        assert_eq!(state.input_buffer, "second prompt");
        assert_eq!(state.history_index, Some(1));

        // Down arrow: restore draft
        state.cursor_down();
        assert_eq!(state.input_buffer, "my draft");
        assert_eq!(state.history_index, None);
    }

    #[test]
    fn test_readline_shortcuts_in_agent_input() {
        let mut state = AgentWorkspaceState::default();
        state.input_buffer = "hello brave new world".to_string();
        state.cursor_pos = state.input_buffer.len();

        // Ctrl+W delete word backward
        state.delete_word_backward();
        assert_eq!(state.input_buffer, "hello brave new ");
        assert_eq!(state.cursor_pos, state.input_buffer.len());

        // Ctrl+U clear to start of line
        state.clear_to_start();
        assert_eq!(state.input_buffer, "");
        assert_eq!(state.cursor_pos, 0);

        // Clear to end
        state.input_buffer = "abc def".to_string();
        state.cursor_pos = 3;
        state.clear_to_end();
        assert_eq!(state.input_buffer, "abc");
    }
}