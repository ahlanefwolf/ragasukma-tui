#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used)
)]

use std::time::Duration;

use color_eyre::Result;
use ragasukma_tui::{
    app::App,
    event::{AppEvent, EventHandler},
    terminal, ui,
};

#[tokio::main]
async fn main() -> Result<()> {
    // 0. Initialize file logger (keeps stdout/stderr clean for Ratatui alternate screen)
    let _log_guard = ragasukma_tui::logging::init_logging();

    // 1. Install terminal panic hook
    terminal::install_panic_hook();

    // 1.1 Ensure assets and configuration files exist (auto-create if missing)
    ragasukma_tui::assets::ensure_assets_exist();

    // 1.2 Background maintenance (N-1).
    //
    // This used to be dead code: `run_retention_loop` had no caller at all, so neither the
    // startup sweep that resets sessions stranded at `running` by a crashed process nor the
    // periodic retention pruning ever ran. `retention_task` performs one sweep immediately
    // and then repeats on the configured interval (a zero interval means "startup sweep
    // only"). It is spawned rather than awaited so the TUI starts regardless of DB latency,
    // and it runs before the bridge reads sessions so the first `SessionsLoaded` already
    // sees corrected rows.
    tokio::spawn(async {
        ragasukma_core::agent::maintenance::run_retention_loop().await;
    });

    // 2. Initialize terminal (raw mode, alternate screen)
    let mut term = terminal::init()?;

    // 3. Setup application state and async event listener
    let root_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let mut app = App::new(root_dir);
    let mut events = EventHandler::new(Duration::from_millis(250));

    // 4. Initialize Core Bridge and forward agent streaming events to TUI event loop
    let (bridge_evt_tx, mut bridge_evt_rx) = tokio::sync::mpsc::channel(1024);
    let app_tx = events.tx.clone();
    tokio::spawn(async move {
        while let Some(evt) = bridge_evt_rx.recv().await {
            let _ = app_tx.send(AppEvent::CoreEvent(evt));
        }
    });

    let pool = ragasukma_core::db::get_pool().await.clone();
    let bridge_res = ragasukma_tui::core_bridge::CoreBridge::initialize(pool, bridge_evt_tx).await;
    match bridge_res {
        Ok(bridge) => {
            tracing::info!("CoreBridge initialized successfully");
            app.bridge_cmd_tx = Some(bridge.cmd_tx);
        }
        Err(e) => {
            tracing::error!("Core initialization failed: {e:?}");
            app.error_modal = Some(format!("Core initialization error: {e}"));
        }
    }
    app.set_event_tx(events.tx.clone());

    // Spawn background MCP server loader
    let mcp_tx = events.tx.clone();
    tokio::spawn(async move {
        match ragasukma_core::repository::agent::projects_and_sessions::list_mcp_servers().await {
            Ok(servers) => {
                let _ = mcp_tx.send(AppEvent::McpServersLoaded(servers));
            }
            Err(e) => {
                tracing::warn!("Failed to load MCP servers in background: {e}");
            }
        }
    });

    // 5. Main application loop
    let run_res = run_app(&mut term, &mut app, &mut events).await;

    // 6. Graceful cleanup of all MCP server processes and reset DB PIDs
    ragasukma_core::agent::mcp::shutdown_all_mcp_servers().await;

    // 7. Restore terminal on exit
    terminal::restore()?;

    if let Err(ref err) = run_res {
        tracing::error!("Error during application execution: {err:?}");
        eprintln!("Error during application execution: {err:?}");
    }

    Ok(())
}

async fn run_app(
    term: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
    events: &mut EventHandler,
) -> Result<()> {
    let mut last_char_key_at: Option<std::time::Instant> = None;
    const PASTE_BURST_THRESHOLD: std::time::Duration = std::time::Duration::from_millis(25);

    while !app.should_quit {
        if app.needs_clear {
            term.clear()?;
            app.needs_clear = false;
        }

        term.draw(|f| ui::draw(f, app))?;

        let mut event = tokio::select! {
            res = events.next() => res?,
            _ = ragasukma_core::agent::mcp::wait_for_shutdown_signal() => {
                tracing::info!("Received shutdown signal (Ctrl+C / SIGTERM), shutting down cleanly");
                app.should_quit = true;
                break;
            }
        };
        loop {
            match event {
                AppEvent::Key(mut key) => {
                    let now = std::time::Instant::now();
                    let in_paste_burst = last_char_key_at
                        .map(|t| now.duration_since(t) < PASTE_BURST_THRESHOLD)
                        .unwrap_or(false);

                    if key.code == crossterm::event::KeyCode::Enter
                        && key.modifiers.is_empty()
                        && in_paste_burst
                    {
                        // Windows terminals (ConPTY/PowerShell) often simulate multi-line paste
                        // as rapid KeyEvent sequences instead of bracketed paste. Treat rapid
                        // Enter as Shift+Enter (newline) so it doesn't prematurely submit.
                        key.modifiers = crossterm::event::KeyModifiers::SHIFT;
                        last_char_key_at = Some(now);
                    } else if matches!(
                        key.code,
                        crossterm::event::KeyCode::Char(_) | crossterm::event::KeyCode::Tab
                    ) && !key
                        .modifiers
                        .intersects(crossterm::event::KeyModifiers::CONTROL | crossterm::event::KeyModifiers::ALT)
                    {
                        last_char_key_at = Some(now);
                    } else {
                        last_char_key_at = None;
                    }

                    tracing::debug!(
                        "key: code={:?} mods={:?} kind={:?} state={:?}",
                        key.code,
                        key.modifiers,
                        key.kind,
                        key.state
                    );
                    let is_paste_image = matches!(
                        key.code,
                        crossterm::event::KeyCode::Char('g') | crossterm::event::KeyCode::Char('G')
                            if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL)
                                && !key.modifiers.contains(crossterm::event::KeyModifiers::ALT)
                    );
                    if is_paste_image {
                        if !try_attach_clipboard_image(app) {
                            app.set_status("No image or image path in clipboard (Ctrl+G)".to_string());
                        }
                    } else if let Some(act) = app.handle_key_event(key) {
                        tracing::debug!("  -> action {:?}", act);
                        app.update(act);
                    }
                }
                AppEvent::Paste(text) => {
                    if (text.trim().is_empty() && try_attach_clipboard_image(app))
                        || try_attach_image_path(app, &text)
                    {
                        // Image attached from clipboard or pasted file path
                    } else if let Some(act) = app.handle_paste_event(text) {
                        app.update(act);
                    }
                }
                AppEvent::CoreEvent(evt) => {
                    app.handle_bridge_event(evt);
                }
                AppEvent::OAuthSuccess { provider, email } => {
                    tracing::info!("OAuth login success: provider='{provider}', email='{email}'");
                    term.clear()?;
                    if provider == "antigravity" {
                        app.antigravity =
                            ragasukma_tui::provider_view::antigravity::load_antigravity_tab();
                    } else if provider == "codex" {
                        app.codex = ragasukma_tui::provider_view::codex::load_codex_tab();
                    }
                    app.set_status(format!(
                        "OAuth Login Successful! Account {} ({}) connected.",
                        email, provider
                    ));
                }
                AppEvent::OAuthFailed { provider, error } => {
                    tracing::error!("OAuth login failed: provider='{provider}', error='{error}'");
                    term.clear()?;
                    app.error_modal = Some(format!("OAuth Login Failed [{}]: {}", provider, error));
                }
                AppEvent::McpServersLoaded(_) => {
                    let _ = app.agent.reload_mcp_servers();
                }
                AppEvent::Tick => {
                    app.on_tick();
                }
                AppEvent::Resize(_, _) => {
                    term.clear()?;
                }
                AppEvent::Mouse(_) => {}
                AppEvent::Error(err) => {
                    tracing::error!("Event error: {err}");
                }
            }

            if app.should_quit {
                break;
            }

            // Drain any pending events that are immediately ready in the queue before redrawing
            match events.rx.try_recv() {
                Ok(next_evt) => {
                    event = next_evt;
                }
                Err(_) => break,
            }
        }
    }
    Ok(())
}

fn decode_file_uri(raw: &str) -> Option<String> {
    let stripped = raw.strip_prefix("file://")?;
    let mut out = String::with_capacity(stripped.len());
    let bytes = stripped.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Ok(hex) = std::str::from_utf8(&bytes[i + 1..i + 3])
            && let Ok(val) = u8::from_str_radix(hex, 16)
        {
            out.push(val as char);
            i += 3;
            continue;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    Some(out)
}

fn try_attach_image_path(app: &mut App, raw: &str) -> bool {
    let first_line = raw.lines().next().unwrap_or(raw);
    let cleaned = first_line.trim().trim_matches(|c| c == '"' || c == '\'').trim();
    if cleaned.is_empty() {
        return false;
    }
    let decoded = decode_file_uri(cleaned);
    let path_str = decoded.as_deref().unwrap_or(cleaned);
    let path = std::path::Path::new(path_str);
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return false;
    };
    let mime_type = match ext.to_ascii_lowercase().as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        _ => return false,
    };
    if !path.is_file() {
        return false;
    }
    let Ok(bytes) = std::fs::read(path) else {
        return false;
    };
    use base64::Engine as _;
    let data_base64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("image")
        .to_string();
    app.attached_images.push(ragasukma_core::llm::types::ImageAttachment {
        mime_type: mime_type.to_string(),
        data_base64,
        name: Some(name.clone()),
    });
    let total = app.attached_images.len();
    if let Some(act) = app.handle_paste_event(format!("[image:{total}] ")) {
        app.update(act);
    }
    app.set_status(format!("Image attached: {name} ({total} total)"));
    tracing::info!("Attached image file '{cleaned}' ({} bytes)", bytes.len());
    true
}

fn attach_png_bytes(app: &mut App, png_bytes: &[u8]) -> bool {
    if png_bytes.is_empty() {
        return false;
    }
    use base64::Engine as _;
    let data_base64 = base64::engine::general_purpose::STANDARD.encode(png_bytes);
    let idx = app.attached_images.len() + 1;
    app.attached_images.push(ragasukma_core::llm::types::ImageAttachment {
        mime_type: "image/png".to_string(),
        data_base64,
        name: Some(format!("clipboard-{idx}.png")),
    });
    if let Some(act) = app.handle_paste_event(format!("[image:{idx}] ")) {
        app.update(act);
    }
    app.set_status(format!("Image attached ({idx} total)"));
    tracing::info!("Attached clipboard image #{idx} ({} bytes PNG)", png_bytes.len());
    true
}

#[cfg(target_os = "linux")]
fn try_linux_cli_clipboard_image(app: &mut App) -> bool {
    let is_wayland = std::env::var_os("WAYLAND_DISPLAY").is_some();

    if is_wayland {
        if let Ok(out) = std::process::Command::new("wl-paste")
            .args(["--no-newline", "--type", "image/png"])
            .output()
            && out.status.success()
            && attach_png_bytes(app, &out.stdout)
        {
            return true;
        }
        if let Ok(out) = std::process::Command::new("wl-paste")
            .args(["--no-newline", "--type", "text/uri-list"])
            .output()
            && out.status.success()
            && let Ok(text) = String::from_utf8(out.stdout)
            && try_attach_image_path(app, &text)
        {
            return true;
        }
    }

    if let Ok(out) = std::process::Command::new("xclip")
        .args(["-selection", "clipboard", "-t", "image/png", "-o"])
        .output()
        && out.status.success()
        && attach_png_bytes(app, &out.stdout)
    {
        return true;
    }

    if let Ok(out) = std::process::Command::new("xclip")
        .args(["-selection", "clipboard", "-t", "text/uri-list", "-o"])
        .output()
        && out.status.success()
        && let Ok(text) = String::from_utf8(out.stdout)
        && try_attach_image_path(app, &text)
    {
        return true;
    }

    false
}

fn try_attach_clipboard_image(app: &mut App) -> bool {
    match arboard::Clipboard::new() {
        Ok(mut clipboard) => {
            if let Ok(img_data) = clipboard.get_image() {
                if let Some(rgba) = image::RgbaImage::from_raw(
                    img_data.width as u32,
                    img_data.height as u32,
                    img_data.bytes.into_owned(),
                ) {
                    let mut png_bytes: Vec<u8> = Vec::new();
                    let mut cursor = std::io::Cursor::new(&mut png_bytes);
                    if image::DynamicImage::ImageRgba8(rgba)
                        .write_to(&mut cursor, image::ImageFormat::Png)
                        .is_ok()
                        && attach_png_bytes(app, &png_bytes)
                    {
                        return true;
                    }
                }
            } else if let Ok(text) = clipboard.get_text()
                && try_attach_image_path(app, &text)
            {
                return true;
            }
        }
        Err(err) => {
            tracing::debug!("arboard::Clipboard::new() failed: {err}");
        }
    }

    #[cfg(target_os = "linux")]
    {
        try_linux_cli_clipboard_image(app)
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}
