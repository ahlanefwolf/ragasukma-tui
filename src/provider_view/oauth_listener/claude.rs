use anyhow::{Context, Result};
use std::time::Duration;

pub async fn exchange_claude_token(
    code: &str,
    redirect_uri: &str,
    state: &str,
) -> Result<String, String> {
    let code = code.to_string();
    let redirect_uri = redirect_uri.to_string();
    let state = state.to_string();

    let account = tokio::task::spawn_blocking(move || {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .context("Failed to initialize HTTP client")?;

        let verifier = ragasukma_core::llm::providers::claude::oauth::take_oauth_state(&state)
            .context("OAuth verifier not found or expired")?;
        ragasukma_core::llm::providers::claude::oauth::complete_oauth_flow(
            &client,
            &code,
            &redirect_uri,
            &verifier,
            &state,
        )
    })
    .await
    .map_err(|e| format!("Task join error: {e}"))?
    .map_err(|e| format!("Claude OAuth error: {e:#}"))?;

    let email = account.email.clone().unwrap_or_else(|| "claude_user".to_string());

    tokio::task::spawn_blocking(move || -> Result<(), String> {
        let mut provider = ragasukma_core::llm::providers::claude::ClaudeProvider::new();
        provider
            .add_oauth_account(account)
            .map_err(|e| format!("Failed to save Claude account: {e}"))?;
        Ok(())
    })
    .await
    .map_err(|e| format!("Task join error: {e}"))??;

    Ok(email)
}
