pub mod controller;
pub mod state;
pub mod types;
pub mod ui;

pub use state::AgentWorkspaceState;
pub use types::{
    AgentFocusArea, ChatMessage, ChatSession, MessageRole, ToolCallInfo, ToolExecutionStatus,
};
pub use ui::render_agent_tab;
