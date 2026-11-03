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
                        let status_label = if status { "Aktif" } else { "Nonaktif" };
                        Some(format!("Status akun Antigravity diubah: {status_label}"))
                    } else {
                        None
                    }
                }
                2 => {
                    if let Some(status) = codex.toggle_selected_account() {
                        let status_label = if status { "Aktif" } else { "Nonaktif" };
                        Some(format!("Status akun Codex diubah: {status_label}"))
                    } else {
                        None
                    }
                }
                3 => {
                    if let Some(status) = claude.toggle_selected_account() {
                        let status_label = if status { "Aktif" } else { "Nonaktif" };
                        Some(format!("Status akun Claude diubah: {status_label}"))
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
                    Some("Akun Antigravity disetel sebagai Default".to_string())
                }
                2 if codex.set_selected_account_active().is_some() => {
                    Some("Akun Codex disetel sebagai Default".to_string())
                }
                3 if claude.set_selected_account_active().is_some() => {
                    Some("Akun Claude disetel sebagai Default".to_string())
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
            Some(Some("Menyegarkan kuota model provider...".to_string()))
        }
        Action::ConfirmDelete => {
            let status = match active_tab {
                1 => antigravity
                    .remove_selected_account()
                    .map(|name| format!("Akun Antigravity '{}' berhasil dihapus", name)),
                2 => codex
                    .remove_selected_account()
                    .map(|name| format!("Akun Codex '{}' berhasil dihapus", name)),
                3 => claude
                    .remove_selected_account()
                    .map(|name| format!("Akun Claude '{}' berhasil dihapus", name)),
                _ => None,
            };
            Some(status)
        }
        _ => None,
    }
}
