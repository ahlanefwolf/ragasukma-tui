use std::collections::HashMap;
use std::time::Instant;
use tokio::sync::mpsc::UnboundedSender;

use crate::agent_view::states::AgentWorkspaceState;
use crate::core_bridge::CoreBridgeCommand;
use crate::custom_llm_view::CustomLlmTabState;
use crate::event::AppEvent;
use crate::provider_views::ProviderTabState;

use super::types::{ConfirmDialogState, DiffApprovalState};

pub struct App {
    pub active_tab: usize,
    pub should_quit: bool,
    pub tick_count: u64,

    pub status_message: Option<String>,
    pub status_timestamp: Option<Instant>,

    pub agent: AgentWorkspaceState,
    pub antigravity: ProviderTabState,
    pub codex: ProviderTabState,
    pub claude: ProviderTabState,
    pub custom_llm: CustomLlmTabState,
    pub mcp_server: crate::mcp_server_view::McpServerTabState,

    pub confirm_dialog: Option<ConfirmDialogState>,
    pub diff_modal: Option<DiffApprovalState>,
    pub error_modal: Option<String>,
    pub needs_clear: bool,
    pub quota_last_fetched: HashMap<(String, String), Instant>,
    pub bridge_cmd_tx: Option<UnboundedSender<CoreBridgeCommand>>,
    pub event_tx: Option<UnboundedSender<AppEvent>>,
}

pub(crate) fn current_time_str() -> String {
    chrono::Local::now().format("%m/%d %H:%M").to_string()
}

impl App {
    pub fn new(_root_dir: std::path::PathBuf) -> Self {
        // Agent workspace tab - clean initial state, database is the source of truth
        let agent = AgentWorkspaceState::default();
        let antigravity = crate::provider_view::antigravity::load_antigravity_tab();
        let codex = crate::provider_view::codex::load_codex_tab();
        let claude = crate::provider_view::claude::load_claude_tab();
        let custom_llm = CustomLlmTabState::new();
        let mcp_server = crate::mcp_server_view::McpServerTabState::new();

        Self {
            active_tab: 0,
            should_quit: false,
            tick_count: 0,
            status_message: Some("Ready - Ragakusma Agent TUI".to_string()),
            status_timestamp: Some(Instant::now()),
            agent,
            antigravity,
            codex,
            claude,
            custom_llm,
            mcp_server,
            confirm_dialog: None,
            diff_modal: None,
            error_modal: None,
            needs_clear: false,
            quota_last_fetched: HashMap::new(),
            bridge_cmd_tx: None,
            event_tx: None,
        }
    }

    pub fn set_bridge_tx(&mut self, tx: UnboundedSender<CoreBridgeCommand>) {
        self.bridge_cmd_tx = Some(tx);
        self.request_all_active_quotas();
    }

    pub fn set_event_tx(&mut self, tx: UnboundedSender<AppEvent>) {
        self.event_tx = Some(tx);
    }

    pub fn set_status(&mut self, msg: String) {
        self.status_message = Some(msg);
        self.status_timestamp = Some(Instant::now());
    }

    pub fn on_tick(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);
        self.agent.tick_spinner();
        if let Some(ts) = self.status_timestamp
            && ts.elapsed().as_secs() > 4
        {
            self.status_message = None;
            self.status_timestamp = None;
        }
    }
}
