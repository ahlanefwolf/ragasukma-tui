use ragasukma_core::llm::client::LlmClient;

/// Returns fallback default models if no accounts are connected yet or core is unreachable
pub fn get_default_models() -> Vec<String> {
    vec![
        "cc/claude-3-5-sonnet-20241022".to_string(),
        "cc/claude-3-5-haiku-20241022".to_string(),
        "cx/gpt-4o".to_string(),
        "cx/gpt-4o-mini".to_string(),
        "ag/gemini-1.5-pro".to_string(),
        "ag/gemini-1.5-flash".to_string(),
    ]
}

/// Fetches available models dynamically from ragasukma-core providers
/// Runs on a dedicated OS thread to prevent blocking `reqwest` runtime drop panics inside Tokio async context.
pub fn fetch_core_available_models() -> Vec<String> {
    let join_handle = std::thread::spawn(fetch_models_internal);
    join_handle.join().unwrap_or_else(|_| get_default_models())
}

fn with_prefix(prefix: &str, model_id: &str) -> String {
    if model_id.starts_with(prefix) {
        model_id.to_string()
    } else {
        format!("{prefix}{model_id}")
    }
}

fn fetch_models_internal() -> Vec<String> {
    let mut collected = Vec::new();
    let mut client = LlmClient::new();

    // 1. Antigravity models (prefix: ag/)
    if let Ok(antigravity_models) = client.antigravity.get_dynamic_available_models() {
        for m in antigravity_models {
            let model_with_prefix = with_prefix("ag/", &m.model_id);
            if !collected.contains(&model_with_prefix) {
                collected.push(model_with_prefix);
            }
        }
    }

    // 2. Codex models (prefix: cx/)
    if let Ok(codex_models) = client.codex.get_dynamic_available_models() {
        for m in codex_models {
            let model_with_prefix = with_prefix("cx/", &m.model_id);
            if !collected.contains(&model_with_prefix) {
                collected.push(model_with_prefix);
            }
        }
    }

    // 3. Claude models (prefix: cc/)
    if let Ok(claude_models) = client.claude.get_dynamic_available_models() {
        for m in claude_models {
            let model_with_prefix = with_prefix("cc/", &m.model_id);
            if !collected.contains(&model_with_prefix) {
                collected.push(model_with_prefix);
            }
        }
    }

    // 4. Custom LLM models
    let custom_models = ragasukma_core::llm::providers::custom::fetch_models();
    for (model_id, _) in custom_models {
        if !collected.contains(&model_id) {
            collected.push(model_id);
        }
    }

    if collected.is_empty() {
        get_default_models()
    } else {
        collected
    }
}
