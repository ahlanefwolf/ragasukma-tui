pub fn format_custom_model_id(prefix: &str, id: &str) -> String {
    let prefix = prefix.trim();
    let id = id.trim();
    if id
        .split_once('/')
        .is_some_and(|(existing_prefix, _)| existing_prefix.eq_ignore_ascii_case(prefix))
    {
        id.to_string()
    } else {
        format!("{prefix}/{id}")
    }
}

pub fn fetch_models() -> Vec<(String, String)> {
    let accounts = ragasukma_core::llm::providers::custom::load_accounts();
    let client = match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(client) => client,
        Err(_) => return Vec::new(),
    };
    let mut models = Vec::new();
    for account in accounts.into_iter().filter(|account| account.enabled) {
        let base = account.base_url.trim().trim_end_matches('/');
        let url = if base.ends_with("/v1") {
            format!("{base}/models")
        } else {
            format!("{base}/v1/models")
        };
        let mut request = client.get(url);
        if !account.api_key.trim().is_empty() {
            request = request.bearer_auth(account.api_key.trim());
        }
        let Ok(response) = request.send() else {
            continue;
        };
        let Ok(body) = response.error_for_status() else {
            continue;
        };
        let Ok(value) = body.json::<serde_json::Value>() else {
            continue;
        };
        if let Some(items) = value.get("data").and_then(serde_json::Value::as_array) {
            for item in items {
                if let Some(id) = item.get("id").and_then(serde_json::Value::as_str) {
                    let id = id.trim();
                    if id.is_empty() {
                        continue;
                    }
                    let model_id = format_custom_model_id(&account.prefix, id);
                    models.push((model_id, account.name.clone()));
                }
            }
        }
    }
    models
}
