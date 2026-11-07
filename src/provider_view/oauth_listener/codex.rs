pub async fn exchange_codex_token(
    code: &str,
    redirect_uri: &str,
    expected_state: &str,
) -> Result<String, String> {
    let code = code.to_string();
    let redirect_uri = redirect_uri.to_string();
    let state = expected_state.to_string();

    tokio::task::spawn_blocking(move || {
        let code_verifier = ragasukma_core::llm::providers::codex::oauth::take_oauth_state(&state)
            .ok_or_else(|| "PKCE code verifier not found or expired".to_string())?;

        let mut provider = ragasukma_core::llm::providers::codex::CodexProvider::new();

        let account = ragasukma_core::llm::providers::codex::oauth::complete_oauth_flow(
            &provider.client,
            &code,
            &redirect_uri,
            &code_verifier,
        )
        .map_err(|e| e.to_string())?;

        let id = provider
            .add_oauth_account(account)
            .map_err(|e| e.to_string())?;

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
