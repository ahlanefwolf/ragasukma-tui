use super::*;

impl AgentWorkspaceState {
    pub fn check_and_update_tag_query(&mut self) {
        self.ensure_valid_cursor();
        if let Some(at_pos) = self.input_buffer[..self.cursor_pos].rfind('@') {
            let slice_between = &self.input_buffer[at_pos + 1..self.cursor_pos];
            // If user typed space, tab, newline after '@', or query is unreasonably long (e.g. pasted code/email)
            if slice_between.contains(' ')
                || slice_between.contains('\t')
                || slice_between.contains('\n')
                || slice_between.len() > 64
            {
                self.tag_query = None;
                return;
            }
            let query = slice_between.to_string();
            let query_lower = query.to_lowercase();

            if self.all_workspace_tag_items.is_empty() && !self.workspace_folders.is_empty() {
                self.refresh_workspace_tag_items();
            }

            let mut all_matches: Vec<TagItem> = self
                .all_workspace_tag_items
                .iter()
                .filter(|item| {
                    if query_lower.is_empty() {
                        true
                    } else {
                        item.name.to_lowercase().contains(&query_lower)
                            || item.path.to_lowercase().contains(&query_lower)
                    }
                })
                .cloned()
                .collect();

            // Prioritize items starting with query, then shorter paths
            all_matches.sort_by(|a, b| {
                let a_starts = a.name.to_lowercase().starts_with(&query_lower);
                let b_starts = b.name.to_lowercase().starts_with(&query_lower);
                match (a_starts, b_starts) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a
                        .path
                        .len()
                        .cmp(&b.path.len())
                        .then_with(|| a.name.cmp(&b.name)),
                }
            });

            let prev_selected = self
                .tag_query
                .as_ref()
                .map(|tq| tq.selected_idx)
                .unwrap_or(0);
            let mut tq = FileTagQuery::new(query, all_matches, at_pos);
            if !tq.matched_items.is_empty() {
                tq.selected_idx = prev_selected.min(tq.matched_items.len() - 1);
            }

            self.tag_query = Some(tq);
        } else {
            self.tag_query = None;
        }
    }

    pub fn select_tag_query_next(&mut self) {
        if let Some(ref mut tq) = self.tag_query {
            if tq.matched_items.is_empty() {
                return;
            }
            // Lazy load next batch when scrolling near or at the end
            if tq.selected_idx + 1 >= tq.matched_items.len() && tq.has_more() {
                tq.load_next_batch();
            }
            if tq.selected_idx + 1 < tq.matched_items.len() {
                tq.selected_idx += 1;
            }
        }
    }

    pub fn select_tag_query_prev(&mut self) {
        if let Some(ref mut tq) = self.tag_query
            && tq.selected_idx > 0
        {
            tq.selected_idx -= 1;
        }
    }

    pub fn apply_selected_tag(&mut self) {
        if let Some(ref tq) = self.tag_query
            && !tq.matched_items.is_empty()
        {
            let item = &tq.matched_items[tq.selected_idx.min(tq.matched_items.len() - 1)];
            let tag_insert = format!("@{} ", item.path);

            let start = tq.tag_start_idx;
            let end = self.cursor_pos;
            if start <= end && end <= self.input_buffer.len() {
                let before = &self.input_buffer[..start];
                let after = &self.input_buffer[end..];
                self.input_buffer = format!("{}{}{}", before, tag_insert, after);
                self.cursor_pos = start + tag_insert.len();
            }
        }
        self.tag_query = None;
    }

    pub fn close_tag_query(&mut self) {
        self.tag_query = None;
    }

    pub fn switch_sidebar_tab(&mut self) {
        self.sidebar_tab = match self.sidebar_tab {
            AgentSidebarTab::Tasks => AgentSidebarTab::Files,
            AgentSidebarTab::Files => AgentSidebarTab::Tasks,
        };
    }

    pub fn next_session(&mut self) {
        if !self.sessions.is_empty() {
            self.selected_session_idx = (self.selected_session_idx + 1) % self.sessions.len();
            self.active_session_id = self
                .sessions
                .get(self.selected_session_idx)
                .map(|s| s.id.clone());
            self.session_start_instant = std::time::Instant::now();
        }
    }

    pub fn prev_session(&mut self) {
        if !self.sessions.is_empty() {
            if self.selected_session_idx == 0 {
                self.selected_session_idx = self.sessions.len() - 1;
            } else {
                self.selected_session_idx -= 1;
            }
            self.active_session_id = self
                .sessions
                .get(self.selected_session_idx)
                .map(|s| s.id.clone());
            self.session_start_instant = std::time::Instant::now();
        }
    }

    pub fn create_new_task(&mut self, title: Option<String>) {
        let task_num = self.sessions.len() + 1;
        let id = format!(
            "task-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        );
        let new_task = TaskSessionItem {
            id: id.clone(),
            title: title.unwrap_or_else(|| format!("Task #{}: New Agent Task", task_num)),
            status: "ready".to_string(),
            model: Some(self.active_model.clone()),
            total_tokens_input: 0,
            total_tokens_output: 0,
            updated_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
            is_running: false,
            total_requests: 0,
        };
        self.sessions.push(new_task);
        self.selected_session_idx = self.sessions.len() - 1;
        self.active_session_id = Some(id);
        self.session_start_instant = std::time::Instant::now();
    }

    pub fn delete_selected_task(&mut self) {
        if !self.sessions.is_empty() && self.selected_session_idx < self.sessions.len() {
            self.sessions.remove(self.selected_session_idx);
            if self.selected_session_idx >= self.sessions.len() && !self.sessions.is_empty() {
                self.selected_session_idx = self.sessions.len() - 1;
            }
            self.active_session_id = self
                .sessions
                .get(self.selected_session_idx)
                .map(|s| s.id.clone());
            self.session_start_instant = std::time::Instant::now();
        }
    }
}
