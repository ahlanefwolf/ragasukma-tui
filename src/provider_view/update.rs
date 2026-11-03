use crate::app::types::Action;
use crate::core_bridge::CoreBridgeCommand;
use crate::provider_view::types::ProviderTabState;
use tokio::sync::mpsc::UnboundedSender;

pub fn handle_provider_action(
    active_tab: usize,
    antigravity: &mut ProviderTabState,
    codex: &mut ProviderTabState,
    claude: &mut ProviderTabState,
    action: &Action,
    bridge_cmd_tx: Option<&UnboundedSender<CoreBridgeCommand>>,
) -> Option<Option<String>> {
    match action {
        Action::ToggleAccountActive => {
            let status = match active_tab {
                1 => {
                    if let Some(status) = antigravity.toggle_selected_account() {
                        let status_label = if status { "Active" } else { "Inactive" };
                        Some(format!("Antigravity account status updated: {status_label}"))
                    } else {
                        None
                    }
                }
                2 => {
                    if let Some(status) = codex.toggle_selected_account() {
                        let status_label = if status { "Active" } else { "Inactive" };
                        Some(format!("Codex account status updated: {status_label}"))
                    } else {
                        None
                    }
                }
                3 => {
                    if let Some(status) = claude.toggle_selected_account() {
                        let status_label = if status { "Active" } else { "Inactive" };
                        Some(format!("Claude account status updated: {status_label}"))
                    } else {
                        None
                    }
                }
                _ => None,
            };
            Some(status)
        }
        Action::SetAccountActive => {
            let status = match active_tab {
                1 if antigravity.set_selected_account_active().is_some() => {
                    Some("Antigravity account set as Default".to_string())
                }
                2 if codex.set_selected_account_active().is_some() => {
                    Some("Codex account set as Default".to_string())
                }
                3 if claude.set_selected_account_active().is_some() => {
                    Some("Claude account set as Default".to_string())
                }
                _ => None,
            };
            Some(status)
        }
        Action::RefreshQuota => {
            if let Some(tx) = bridge_cmd_tx {
                let prov = match active_tab {
                    1 => "antigravity",
                    2 => "codex",
                    3 => "claude",
                    _ => "",
                };
                if !prov.is_empty() {
                    let _ = tx.send(CoreBridgeCommand::RefreshQuota {
                        provider: prov.to_string(),
                    });
                }
            }
            Some(Some("Refreshing provider model quotas...".to_string()))
        }
        Action::QuotaScrollDown => {
            match active_tab {
                1 => antigravity.scroll_quota_down(5),
                2 => codex.scroll_quota_down(5),
                3 => claude.scroll_quota_down(5),
                _ => {}
            }
            Some(None)
        }
        Action::QuotaScrollUp => {
            match active_tab {
                1 => antigravity.scroll_quota_up(),
                2 => codex.scroll_quota_up(),
                3 => claude.scroll_quota_up(),
                _ => {}
            }
            Some(None)
        }
        Action::ProviderToggleFocus => {
            let label = match active_tab {
                1 => {
                    antigravity.toggle_focus();
                    format!("Focus: {:?}", antigravity.focus_area)
                }
                2 => {
                    codex.toggle_focus();
                    format!("Focus: {:?}", codex.focus_area)
                }
                3 => {
                    claude.toggle_focus();
                    format!("Focus: {:?}", claude.focus_area)
                }
                _ => String::new(),
            };
            Some(Some(label))
        }
        Action::ConfirmDelete => {
            let status = match active_tab {
                1 => antigravity
                    .remove_selected_account()
                    .map(|name| format!("Antigravity account '{}' deleted successfully", name)),
                2 => codex
                    .remove_selected_account()
                    .map(|name| format!("Codex account '{}' deleted successfully", name)),
                3 => claude
                    .remove_selected_account()
                    .map(|name| format!("Claude account '{}' deleted successfully", name)),
                _ => None,
            };
            Some(status)
        }
        _ => None,
    }
}
