/// Launch browser for OAuth authorization URL cleanly across all platforms
pub fn open_browser_oauth(url: &str) {
    tracing::info!("Opening OAuth authorization URL in browser: {url}");
    if let Err(e) = open::that_detached(url) {
        tracing::error!("Failed to launch browser for OAuth: {e}");
    }
}
