pub async fn exchange_antigravity_token(code: &str, redirect_uri: &str) -> Result<String, String> {
    let code = code.to_string();
    let redirect_uri = redirect_uri.to_string();

    tokio::task::spawn_blocking(move || {
        let mut provider =
            ragasukma_core::llm::providers::antigravity::AntigravityProvider::new();
        let account_id = provider
            .add_oauth_account(&code, &redirect_uri)
            .map_err(|e| e.to_string())?;

        let email = provider
            .accounts
            .iter()
            .find(|a| a.id == account_id)
            .and_then(|a| a.email.clone())
            .unwrap_or_else(|| account_id.clone());

        Ok::<String, String>(email)
    })
    .await
    .map_err(|e| format!("Join error: {}", e))?
}
