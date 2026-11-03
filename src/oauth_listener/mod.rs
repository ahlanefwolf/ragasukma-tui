pub mod antigravity;
pub mod browser;
pub mod codex;
pub mod server;

pub use browser::open_browser_oauth;

use tokio::sync::mpsc;
use crate::event::AppEvent;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OAuthProviderKind {
    Antigravity,
    Codex,
}

pub fn spawn_oauth_listener(
    kind: OAuthProviderKind,
    port: u16,
    expected_state: String,
    event_tx: mpsc::UnboundedSender<AppEvent>,
) {
    tokio::spawn(async move {
        let (stream, callback_data) = match server::listen_for_oauth_callback(port, &expected_state).await {
            Ok(res) => res,
            Err(e) => {
                let prov = match kind {
                    OAuthProviderKind::Antigravity => "antigravity",
                    OAuthProviderKind::Codex => "codex",
                };
                let _ = event_tx.send(AppEvent::OAuthFailed {
                    provider: prov.to_string(),
                    error: e,
                });
                return;
            }
        };

        let redirect_uri = match kind {
            OAuthProviderKind::Antigravity => format!("http://localhost:{}/oauth/callback", port),
            OAuthProviderKind::Codex => {
                ragasukma_core::llm::providers::codex::constants::CODEX_REDIRECT_URI.to_string()
            }
        };
        let exchange_res = match kind {
            OAuthProviderKind::Antigravity => {
                antigravity::exchange_antigravity_token(&callback_data.code, &redirect_uri).await
            }
            OAuthProviderKind::Codex => {
                codex::exchange_codex_token(&callback_data.code, &redirect_uri, &expected_state).await
            }
        };

        match exchange_res {
            Ok(email) => {
                server::respond_success(stream, &email).await;
                let prov = match kind {
                    OAuthProviderKind::Antigravity => "antigravity",
                    OAuthProviderKind::Codex => "codex",
                };
                let _ = event_tx.send(AppEvent::OAuthSuccess {
                    provider: prov.to_string(),
                    email,
                });
            }
            Err(err) => {
                server::respond_error(stream, &err).await;
                let prov = match kind {
                    OAuthProviderKind::Antigravity => "antigravity",
                    OAuthProviderKind::Codex => "codex",
                };
                let _ = event_tx.send(AppEvent::OAuthFailed {
                    provider: prov.to_string(),
                    error: err,
                });
            }
        }
    });
}
