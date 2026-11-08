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
    let (bridge_evt_tx, mut bridge_evt_rx) = tokio::sync::mpsc::unbounded_channel();
    let app_tx = events.tx.clone();
    tokio::spawn(async move {
        while let Some(evt) = bridge_evt_rx.recv().await {
            let _ = app_tx.send(AppEvent::CoreEvent(evt));
        }
    });

    let pool = ragasukma_core::llm::usage::get_pool().await.clone();
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
        match ragasukma_core::agent::repository::projects_and_sessions::list_mcp_servers().await {
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

    // 6. Restore terminal on exit
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
    while !app.should_quit {
        if app.needs_clear {
            term.clear()?;
            app.needs_clear = false;
        }

        term.draw(|f| ui::draw(f, app))?;

        let mut event = tokio::select! {
            res = events.next() => res?,
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("Received Ctrl+C interrupt signal, shutting down cleanly");
                app.should_quit = true;
                break;
            }
        };
        loop {
            match event {
                AppEvent::Key(key) => {
                    tracing::debug!(
                        "key: code={:?} mods={:?} kind={:?} state={:?}",
                        key.code,
                        key.modifiers,
                        key.kind,
                        key.state
                    );
                    if let Some(act) = app.handle_key_event(key) {
                        tracing::debug!("  -> action {:?}", act);
                        app.update(act);
                    }
                }
                AppEvent::Paste(text) => {
                    if let Some(act) = app.handle_paste_event(text) {
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
                AppEvent::McpServersLoaded(servers) => {
                    app.mcp_server.set_servers(servers);
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
