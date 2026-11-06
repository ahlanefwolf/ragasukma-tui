use color_eyre::Result;
use crossterm::event::{Event as CrosstermEvent, EventStream, KeyEvent, KeyEventKind, MouseEvent};
use futures_util::StreamExt;
use ragasukma_core::agent::models::McpServer;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::core_bridge::CoreBridgeEvent;

#[derive(Clone, Debug)]
pub enum AppEvent {
    Key(KeyEvent),
    Paste(String),
    Mouse(MouseEvent),
    Resize(u16, u16),
    Tick,
    CoreEvent(CoreBridgeEvent),
    OAuthSuccess { provider: String, email: String },
    OAuthFailed { provider: String, error: String },
    McpServersLoaded(Vec<McpServer>),
    Error(String),
}

pub struct EventHandler {
    pub tx: mpsc::UnboundedSender<AppEvent>,
    pub rx: mpsc::UnboundedReceiver<AppEvent>,
    _task: JoinHandle<()>,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let event_tx = tx.clone();

        let task = tokio::spawn(async move {
            let mut reader = EventStream::new();
            let mut tick_interval = tokio::time::interval(tick_rate);

            loop {
                let tick_delay = tick_interval.tick();
                let crossterm_event = reader.next();

                tokio::select! {
                    _ = tick_delay => {
                        if event_tx.send(AppEvent::Tick).is_err() {
                            break;
                        }
                    }
                    maybe_event = crossterm_event => {
                        match maybe_event {
                            Some(Ok(event)) => {
                                let app_event = match event {
                                    CrosstermEvent::Key(key) => {
                                        if key.kind == KeyEventKind::Release {
                                            continue;
                                        }
                                        AppEvent::Key(key)
                                    }
                                    CrosstermEvent::Paste(text) => AppEvent::Paste(text),
                                    CrosstermEvent::Mouse(mouse) => AppEvent::Mouse(mouse),
                                    CrosstermEvent::Resize(w, h) => AppEvent::Resize(w, h),
                                    _ => continue,
                                };

                                if event_tx.send(app_event).is_err() {
                                    break;
                                }
                            }
                            Some(Err(err)) => {
                                let _ = event_tx.send(AppEvent::Error(err.to_string()));
                            }
                            None => break,
                        }
                    }
                }
            }
        });

        Self {
            tx,
            rx,
            _task: task,
        }
    }

    pub async fn next(&mut self) -> Result<AppEvent> {
        self.rx
            .recv()
            .await
            .ok_or_else(|| color_eyre::eyre::eyre!("Event channel closed"))
    }
}

impl Drop for EventHandler {
    fn drop(&mut self) {
        self._task.abort();
    }
}
