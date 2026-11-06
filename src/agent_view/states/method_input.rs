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

    pub fn append_chunk(&mut self, chunk: &str) {
        self.auto_scroll = true;
        if let Some(last) = self
            .messages
            .last_mut()
            .filter(|m| m.role == MessageRole::Assistant)
        {
            last.content.push_str(chunk);
            return;
        }

        self.messages.push(ChatMessage {
            role: MessageRole::Assistant,
            sender_name: "Agent".to_string(),
            content: chunk.to_string(),
            timestamp: "now".to_string(),
            tool_calls: Vec::new(),
            reasoning: None,
        });
        self.enforce_message_limit();
    }

    pub fn append_thought(&mut self, thought: &str) {
        self.auto_scroll = true;
        if let Some(last) = self
            .messages
            .last_mut()
            .filter(|m| m.role == MessageRole::Assistant)
        {
            if let Some(ref mut r) = last.reasoning {
                r.push_str(thought);
            } else {
                last.reasoning = Some(thought.to_string());
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

    pub fn submit_prompt(&mut self) -> Option<String> {
        let trimmed = self.input_buffer.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            self.input_buffer.clear();
            self.cursor_pos = 0;
            self.tag_query = None;
            self.scroll_offset = 0;
            self.auto_scroll = true;
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
            }
            order.push(AgentFocusArea::Folders);
        }
        order.push(AgentFocusArea::Chat);
        order.push(AgentFocusArea::Input);
        if self.is_inspector_visible {
            order.push(AgentFocusArea::ModelTelemetry);
            order.push(AgentFocusArea::McpTools);
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
            }
            order.push(AgentFocusArea::Folders);
        }
        order.push(AgentFocusArea::Chat);
        order.push(AgentFocusArea::Input);
        if self.is_inspector_visible {
            order.push(AgentFocusArea::ModelTelemetry);
            order.push(AgentFocusArea::McpTools);
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
        if self.focus_area == AgentFocusArea::McpTools {
            self.inspector_scroll_offset = self.inspector_scroll_offset.saturating_sub(1);
        } else {
            let max = self.chat_max_scroll.get();
            self.scroll_offset = (self.scroll_offset.saturating_add(1)).min(max);
            self.auto_scroll = false;
        }
    }

    pub fn scroll_down(&mut self) {
        if self.focus_area == AgentFocusArea::McpTools {
            self.inspector_scroll_offset = self.inspector_scroll_offset.saturating_add(1);
        } else {
            let max = self.chat_max_scroll.get();
            let current = self.scroll_offset.min(max);
            self.scroll_offset = current.saturating_sub(1);
            if self.scroll_offset == 0 {
                self.auto_scroll = true;
            }
        }
    }

    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = self.chat_max_scroll.get();
        self.auto_scroll = false;
    }
}
