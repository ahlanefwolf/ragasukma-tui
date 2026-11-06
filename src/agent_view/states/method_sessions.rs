use super::*;

impl AgentWorkspaceState {
    pub fn calculate_token_breakdown(&self) -> TokenBreakdown {
        let chat_tokens = self.messages.iter().map(|m| m.content.len() / 4).sum();
        let session_tokens = self
            .sessions
            .iter()
            .map(|s| (s.total_tokens_input + s.total_tokens_output) as usize)
            .sum();

        let mut user_prompts = 0;
        let mut assistant_turns = 0;
        let mut msg_in_tokens = 0;
        let mut msg_out_tokens = 0;

        for msg in &self.messages {
            let tok = (msg.content.len() / 4).max(1);
            match msg.role {
                crate::agent_view::types::MessageRole::User => {
                    user_prompts += 1;
                    msg_in_tokens += tok;
                }
                crate::agent_view::types::MessageRole::Assistant => {
                    assistant_turns += 1;
                    msg_out_tokens += tok;
                }
                crate::agent_view::types::MessageRole::Tool => {
                    // Tool output is fed back as input context to the next model call
                    msg_in_tokens += tok;
                }
                _ => {}
            }
        }

        // Each tool call executed by the agent constitutes a full API round-trip cycle to the model.
        let total_tool_calls: usize = self.messages.iter().map(|m| m.tool_calls.len()).sum();

        // Calculate total LLM interaction turns (requests to model):
        // Each assistant response and tool execution represents a completed LLM API call cycle.
        // In addition, any ongoing turn (is_running) counts as an active live request (+1).
        // Determine current session either by active_session_id or selected_session_idx
        let cur_session = self
            .active_session_id
            .as_ref()
            .and_then(|aid| self.sessions.iter().find(|s| &s.id == aid))
            .or_else(|| self.sessions.get(self.selected_session_idx));

        let completed_requests = assistant_turns + total_tool_calls;
        let (stored_in, stored_out, stored_requests) = if let Some(s) = cur_session {
            (
                s.total_tokens_input as usize,
                s.total_tokens_output as usize,
                s.total_requests,
            )
        } else {
            (0, 0, 0)
        };

        let base_requests = completed_requests.max(stored_requests);
        let task_requests = if base_requests > 0 {
            base_requests
        } else {
            user_prompts
        };

        let task_input_tokens = stored_in.max(msg_in_tokens);
        let task_output_tokens = stored_out.max(msg_out_tokens);

        TokenBreakdown {
            system_tokens: 1500,
            chat_history_tokens: chat_tokens,
            session_tokens,
            completion_tokens: self.total_completion_tokens,
            max_context_limit: self.max_context_limit,
            task_input_tokens,
            task_output_tokens,
            task_requests,
        }
    }
}
