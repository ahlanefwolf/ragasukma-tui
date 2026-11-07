pub async fn exchange_claude_token(
    code: &str,
    redirect_uri: &str,
    state: &str,
) -> Result<String, String> {
    let code = code.to_string();
    let redirect_uri = redirect_uri.to_string();
    let state = state.to_string();

    tokio::task::spawn_blocking(move || {
        let code_verifier = ragasukma_core::llm::providers::claude::oauth::take_oauth_state(&state)
            .ok_or_else(|| "Claude OAuth state or verifier expired".to_string())?;

        let mut provider = ragasukma_core::llm::providers::claude::ClaudeProvider::new();

        let account = ragasukma_core::llm::providers::claude::oauth::complete_oauth_flow(
            &provider.client,
            &code,
            &redirect_uri,
            &code_verifier,
            &state,
        )
        .map_err(|e| format!("Claude OAuth error: {e:#}"))?;

        let id = provider
            .add_oauth_account(account)
            .map_err(|e| format!("Failed to save Claude account: {e}"))?;

        let email = provider
            .accounts
            .iter()
            .find(|a| a.id == id)
            .and_then(|a| a.email.clone())
            .unwrap_or(id);

        Ok::<String, String>(email)
    })
    .await
    .map_err(|e| format!("Join error: {}", e))?
}
