use std::fs;
use std::path::{Path, PathBuf};

pub use ragasukma_core::llm::providers::antigravity::types::AntigravityAccount;
pub use ragasukma_core::llm::providers::claude::types::ClaudeAccount;
pub use ragasukma_core::llm::providers::codex::types::CodexAccount;
pub use ragasukma_core::llm::providers::custom::CustomLlmAccount;

/// Returns the primary assets/config directory.
/// - In development mode: checks local `./assets` first so developers can mock files locally.
/// - In release mode: directly targets the shared OS configuration directory (`~/.config/ragasukma` / `%APPDATA%\ragasukma`),
///   shared seamlessly between `@ragasukma` (GUI) and `@ragasukma-tui`.
pub fn get_assets_dir() -> PathBuf {
    #[cfg(debug_assertions)]
    {
        // 1. Current working dir / assets (development mode)
        let cwd_assets = PathBuf::from("assets");
        if cwd_assets.exists() && cwd_assets.is_dir() {
            return cwd_assets;
        }

        // 2. Relative to executable (development target/debug/)
        if let Ok(exe_path) = std::env::current_exe()
            && let Some(exe_dir) = exe_path.parent()
        {
            let exe_assets = exe_dir.join("assets");
            if exe_assets.exists() && exe_assets.is_dir() {
                return exe_assets;
            }
            let target_parent_assets = exe_dir.join("../../assets");
            if target_parent_assets.exists() && target_parent_assets.is_dir() {
                return target_parent_assets;
            }
        }
    }

    // Production release or fallback: Always shared user configuration directory
    ragasukma_core::config::get_app_config_dir()
}

/// Locate an account file:
/// Checks shared config directory first in release, or assets dir in debug.
pub fn find_account_file(filename: &str) -> PathBuf {
    let config_file = ragasukma_core::config::get_app_config_dir().join(filename);
    if config_file.exists() {
        return config_file;
    }

    #[cfg(debug_assertions)]
    {
        let assets_file = get_assets_dir().join(filename);
        if assets_file.exists() {
            return assets_file;
        }
    }

    config_file
}

/// Ensures required asset directories and template JSON files exist.
/// Auto-creates missing account JSONs in the shared config directory if they do not exist anywhere.
pub fn ensure_assets_exist() {
    let config_dir = ragasukma_core::config::get_app_config_dir();
    let _ = fs::create_dir_all(&config_dir);
    let assets_dir = get_assets_dir();

    #[cfg(debug_assertions)]
    {
        if assets_dir != config_dir {
            let _ = fs::create_dir_all(&assets_dir);
        }
    }

    // 1. Antigravity accounts
    ensure_file_with_template(
        "antigravity_accounts.json",
        &assets_dir,
        &config_dir,
        r#"[
  {
    "id": "ag-acc-default-1",
    "email": "user@gmail.com",
    "refresh_token": "",
    "access_token": "",
    "expires_at": 0,
    "project_id": "ragasukma-default",
    "tier_id": "free-tier",
    "rate_limited_until": 0,
    "model_rate_limits": {},
    "consecutive_errors": 0,
    "is_active": true
  }
]
"#,
    );

    // 2. Codex accounts
    ensure_file_with_template(
        "codex_accounts.json",
        &assets_dir,
        &config_dir,
        r#"[
  {
    "id": "codex-acc-default-1",
    "email": "user@openai.com",
    "api_key": "",
    "organization": "",
    "tier": "pay-as-you-go",
    "is_active": true
  }
]
"#,
    );

    // 3. Claude accounts
    ensure_file_with_template(
        "claude_accounts.json",
        &assets_dir,
        &config_dir,
        r#"[
  {
    "id": "claude-acc-default-1",
    "email": "user@anthropic.com",
    "session_key": "",
    "tier": "free",
    "is_active": true
  }
]
"#,
    );

    // 4. Custom LLM / API accounts
    ensure_file_with_template(
        "api_llm_accounts.json",
        &assets_dir,
        &config_dir,
        r#"[
  {
    "prefix": "groq",
    "name": "Groq Cloud LPU",
    "base_url": "https://api.groq.com/openai/v1",
    "api_key": "",
    "enabled": false
  },
  {
    "prefix": "deepseek",
    "name": "DeepSeek API Platform",
    "base_url": "https://api.deepseek.com/v1",
    "api_key": "",
    "enabled": false
  },
  {
    "prefix": "local",
    "name": "Local Ollama / vLLM",
    "base_url": "http://localhost:11434/v1",
    "api_key": "",
    "enabled": true
  }
]
"#,
    );
}

fn ensure_file_with_template(
    filename: &str,
    assets_dir: &Path,
    config_dir: &Path,
    default_content: &str,
) {
    let assets_file = assets_dir.join(filename);
    let config_file = config_dir.join(filename);

    if !assets_file.exists() && !config_file.exists() {
        let _ = fs::write(&assets_file, default_content);
    } else if config_file.exists() && !assets_file.exists() {
        // If config exists, sync a copy to assets folder
        let _ = fs::copy(&config_file, &assets_file);
    }
}

pub fn load_antigravity_accounts() -> Vec<AntigravityAccount> {
    let file = find_account_file("antigravity_accounts.json");
    if let Ok(content) = fs::read_to_string(&file)
        && let Ok(accounts) = serde_json::from_str::<Vec<AntigravityAccount>>(&content)
    {
        return accounts;
    }
    Vec::new()
}

pub fn load_codex_accounts() -> Vec<CodexAccount> {
    let file = find_account_file("codex_accounts.json");
    if let Ok(content) = fs::read_to_string(&file)
        && let Ok(accounts) = serde_json::from_str::<Vec<CodexAccount>>(&content)
    {
        return accounts;
    }
    Vec::new()
}

pub fn load_claude_accounts() -> Vec<ClaudeAccount> {
    let file = find_account_file("claude_accounts.json");
    if let Ok(content) = fs::read_to_string(&file)
        && let Ok(accounts) = serde_json::from_str::<Vec<ClaudeAccount>>(&content)
    {
        return accounts;
    }
    Vec::new()
}

pub fn load_custom_llm_accounts() -> Vec<CustomLlmAccount> {
    let file = find_account_file("api_llm_accounts.json");
    if let Ok(content) = fs::read_to_string(&file)
        && let Ok(accounts) = serde_json::from_str::<Vec<CustomLlmAccount>>(&content)
    {
        return accounts;
    }
    Vec::new()
}
