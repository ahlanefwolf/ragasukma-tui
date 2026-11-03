use std::time::Duration;

use color_eyre::Result;
use ragasukma_tui::{
    app::App,
    event::{AppEvent, EventHandler},
    terminal, ui,
};

#[tokio::main]
async fn main() -> Result<()> {
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
            app.bridge_cmd_tx = Some(bridge.cmd_tx);
            app.status_message = Some("Connected to Ragasukma Core engine.".to_string());
        }
        Err(e) => {
            app.status_message = Some(format!("Core init warning: {}", e));
        }
    }

    // 5. Main application loop
    let run_res = run_app(&mut term, &mut app, &mut events).await;

    // 6. Restore terminal on exit
    terminal::restore()?;

    if let Err(err) = run_res {
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
        term.draw(|f| ui::draw(f, app))?;

        let event = events.next().await?;
        match event {
            AppEvent::Key(key) => {
                if let Some(act) = app.handle_key_event(key) {
                    app.update(act);
                }
            }
            AppEvent::CoreEvent(evt) => {
                app.handle_bridge_event(evt);
            }
            AppEvent::Tick => {
                app.on_tick();
            }
            AppEvent::Resize(_, _) => {}
            AppEvent::Mouse(_) => {}
            AppEvent::Error(err) => {
                eprintln!("Event error: {err}");
            }
        }
    }

    Ok(())
}
