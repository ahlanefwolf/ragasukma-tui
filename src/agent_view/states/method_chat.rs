use super::*;

impl AgentWorkspaceState {
    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
        self.auto_scroll = true;
    }

    pub fn toggle_tool_expand(&mut self) {
        if self.expanded_tool_idx.is_some() {
            self.expanded_tool_idx = None;
        } else {
            self.expanded_tool_idx = Some(0);
        }
    }

    pub fn toggle_tool_card(&mut self, idx: usize) {
        if self.expanded_tools.contains(&idx) {
            self.expanded_tools.remove(&idx);
        } else {
            self.expanded_tools.insert(idx);
        }
    }

    pub fn toggle_reasoning(&mut self) {
        self.reasoning_expanded = !self.reasoning_expanded;
    }

    pub fn toggle_sidebar(&mut self) {
        self.is_sidebar_visible = !self.is_sidebar_visible;
        if !self.is_sidebar_visible && self.focus_area == AgentFocusArea::Files {
            self.focus_area = AgentFocusArea::Chat;
        }
    }

    pub fn refresh_available_models(&mut self) -> usize {
        let fetched = crate::agent_view::models::fetch_core_available_models();
        let count = fetched.len();
        if !fetched.is_empty() {
            if !fetched.contains(&self.active_model)
                && let Some(first) = fetched.first()
            {
                self.active_model = first.clone();
            }
            self.available_models = fetched;
        }
        count
    }

    pub fn cycle_model(&mut self) {
        if self.available_models.is_empty() {
            return;
        }
        if let Some(pos) = self
            .available_models
            .iter()
            .position(|m| m == &self.active_model)
        {
            let next = (pos + 1) % self.available_models.len();
            self.active_model = self.available_models[next].clone();
        } else {
            self.active_model = self.available_models[0].clone();
        }
    }

    pub fn filtered_modal_models(&self) -> Vec<String> {
        let q = self.model_modal.search_query.trim().to_lowercase();
        if q.is_empty() {
            self.available_models.clone()
        } else {
            self.available_models
                .iter()
                .filter(|m| m.to_lowercase().contains(&q))
                .cloned()
                .collect()
        }
    }

    pub fn open_model_modal(&mut self) {
        self.model_modal.is_open = true;
        self.model_modal.search_query.clear();
        let filtered = self.filtered_modal_models();
        if let Some(idx) = filtered.iter().position(|m| m == &self.active_model) {
            self.model_modal.selected_idx = idx;
        } else {
            self.model_modal.selected_idx = 0;
        }
    }

    pub fn close_model_modal(&mut self) {
        self.model_modal.is_open = false;
        self.model_modal.search_query.clear();
    }

    pub fn model_modal_push_char(&mut self, c: char) {
        self.model_modal.search_query.push(c);
        let filtered = self.filtered_modal_models();
        if self.model_modal.selected_idx >= filtered.len() {
            self.model_modal.selected_idx = 0;
        }
    }

    pub fn model_modal_backspace(&mut self) {
        self.model_modal.search_query.pop();
        let filtered = self.filtered_modal_models();
        if self.model_modal.selected_idx >= filtered.len() {
            self.model_modal.selected_idx = 0;
        }
    }

    pub fn select_next_modal_model(&mut self) {
        let filtered = self.filtered_modal_models();
        if !filtered.is_empty() {
            self.model_modal.selected_idx = (self.model_modal.selected_idx + 1) % filtered.len();
        }
    }

    pub fn select_prev_modal_model(&mut self) {
        let filtered = self.filtered_modal_models();
        if !filtered.is_empty() {
            if self.model_modal.selected_idx == 0 {
                self.model_modal.selected_idx = filtered.len() - 1;
            } else {
                self.model_modal.selected_idx -= 1;
            }
        }
    }

    pub fn apply_modal_model(&mut self) {
        let filtered = self.filtered_modal_models();
        if let Some(m) = filtered.get(self.model_modal.selected_idx) {
            self.active_model = m.clone();
        }
        self.model_modal.is_open = false;
        self.model_modal.search_query.clear();
    }

    pub fn start_turn(&mut self) {
        self.is_running = true;
        self.scroll_offset = 0;
        self.auto_scroll = true;
        if let Some(active_sid) = &self.active_session_id
            && let Some(s) = self.sessions.iter_mut().find(|s| &s.id == active_sid)
        {
            s.is_running = true;
        }
    }

    pub fn cancel_turn(&mut self) {
        self.is_running = false;
        self.active_turn_id = None;
        if let Some(active_sid) = &self.active_session_id
            && let Some(s) = self.sessions.iter_mut().find(|s| &s.id == active_sid)
        {
            s.is_running = false;
        }
    }

    pub fn tick_spinner(&mut self) {
        self.spinner_tick = (self.spinner_tick + 1) % 10;
    }

    pub fn spinner_char(&self) -> &'static str {
        const FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        FRAMES[self.spinner_tick % FRAMES.len()]
    }
}
