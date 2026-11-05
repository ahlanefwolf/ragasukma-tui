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
            app.status_message = Some("Connected to Ragasukma Core engine.".to_string());
        }
        Err(e) => {
            tracing::error!("Core initialization failed: {e:?}");
            app.status_message = Some(format!("Core init warning: {}", e));
        }
    }
    app.set_event_tx(events.tx.clone());

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

        let mut event = events.next().await?;
        loop {
            match event {
                AppEvent::Key(key) => {
                    if let Some(act) = app.handle_key_event(key) {
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
                    app.set_status(format!("OAuth Login Failed [{}]: {}", provider, error));
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