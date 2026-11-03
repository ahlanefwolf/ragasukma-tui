use anyhow::Result;
use ragasukma_core::agent::events::get_session_sender;
use ragasukma_core::agent::models::{AgentSessionEvent, Message};
use ragasukma_core::agent::repository::{
    create_message, create_project, create_session, delete_session, list_messages, list_projects,
    list_sessions,
};
use sqlx::SqlitePool;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedMessageDto {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: Option<String>,
    pub thought: Option<String>,
    pub tool_calls: Option<String>,
    pub created_at: i64,
}

impl From<Message> for PersistedMessageDto {
    fn from(m: Message) -> Self {
        Self {
            id: m.id,
            session_id: m.session_id,
            role: m.role,
            content: m.content,
            thought: m.thought,
            tool_calls: m.tool_calls,
            created_at: m.created_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreBridgeEvent {
    Connected {
        project_id: String,
        session_id: String,
    },
    SessionsLoaded {
        sessions: Vec<(String, String)>,
        active_id: String,
    },
    SessionCreated {
        id: String,
        title: String,
    },
    SessionDeleted {
        id: String,
    },
    HistoryLoaded {
        session_id: String,
        messages: Vec<PersistedMessageDto>,
    },
    StatusChange(String),
    Delta(String),
    ThoughtDelta(String),
    StepInfo {
        iteration: usize,
        action: String,
        details: Option<String>,
    },
    ToolCall {
        name: String,
        args: String,
    },
    TurnCompleted(String),
    QuotaUpdated {
        provider: String,
        input_tokens: usize,
        output_tokens: usize,
        total_tokens: usize,
        requests: usize,
    },
    Error(String),
}

#[derive(Debug, Clone)]
pub enum CoreBridgeCommand {
    ExecuteTurn {
        prompt: String,
        model_override: Option<String>,
    },
    RefreshQuota {
        provider: String,
    },
    CreateSession {
        title: String,
    },
    SwitchSession {
        session_id: String,
    },
    DeleteSession {
        session_id: String,
    },
    LoadHistory {
        session_id: String,
    },
}

pub struct CoreBridge {
    pub cmd_tx: mpsc::UnboundedSender<CoreBridgeCommand>,
    pub project_id: String,
    pub session_id: String,
}

impl CoreBridge {
    pub async fn initialize(
        db_pool: SqlitePool,
        event_tx: mpsc::UnboundedSender<CoreBridgeEvent>,
    ) -> Result<Self> {
        let projects = list_projects().await.unwrap_or_default();
        let project = if let Some(first) = projects.into_iter().next() {
            first
        } else {
            create_project("Default Workspace", "TUI Interactive Workspace", None, None).await?
        };

        let existing_sessions = list_sessions(&project.id).await.unwrap_or_default();
        let session = if let Some(first) = existing_sessions.first() {
            first.clone()
        } else {
            create_session(
                &project.id,
                "TUI Interactive Session",
                None,
                None,
                None,
            )
            .await?
        };

        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<CoreBridgeCommand>();

        let sid = session.id.clone();
        let pid = project.id.clone();

        let _ = event_tx.send(CoreBridgeEvent::Connected {
            project_id: pid.clone(),
            session_id: sid.clone(),
        });

        // Emit initial session list and history
        let all_sessions = list_sessions(&pid).await.unwrap_or_default();
        let sessions_summary = all_sessions
            .into_iter()
            .map(|s| (s.id, s.title))
            .collect::<Vec<_>>();

        let _ = event_tx.send(CoreBridgeEvent::SessionsLoaded {
            sessions: sessions_summary,
            active_id: sid.clone(),
        });

        if let Ok(msgs) = list_messages(&sid).await {
            let dtos = msgs.into_iter().map(PersistedMessageDto::from).collect();
            let _ = event_tx.send(CoreBridgeEvent::HistoryLoaded {
                session_id: sid.clone(),
                messages: dtos,
            });
        }

        let current_sid = Arc::new(Mutex::new(sid.clone()));
        let worker_project_id = pid.clone();
        let worker_tx = event_tx.clone();
        let current_sid_clone = Arc::clone(&current_sid);

        tokio::spawn(async move {
            let pool_clone = db_pool.clone();
            let mut active_sid = sid.clone();

            // Spawn subscriber loop
            let sub_tx = worker_tx.clone();
            let sub_sid = Arc::clone(&current_sid_clone);
            tokio::spawn(async move {
                loop {
                    let cur = {
                        let guard = sub_sid.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                        guard.clone()
                    };
                    let mut broadcast_rx = get_session_sender(&cur).subscribe();
                    tokio::select! {
                        evt = broadcast_rx.recv() => {
                            if let Ok(event) = evt {
                                match event {
                                    AgentSessionEvent::Delta { delta } => {
                                        let _ = sub_tx.send(CoreBridgeEvent::Delta(delta));
                                    }
                                    AgentSessionEvent::ThoughtDelta { delta } => {
                                        let _ = sub_tx.send(CoreBridgeEvent::ThoughtDelta(delta));
                                    }
                                    AgentSessionEvent::StatusChange { status } => {
                                        let _ = sub_tx.send(CoreBridgeEvent::StatusChange(status));
                                    }
                                    AgentSessionEvent::StepInfo {
                                        iteration,
                                        action,
                                        details,
                                    } => {
                                        let _ = sub_tx.send(CoreBridgeEvent::StepInfo {
                                            iteration,
                                            action,
                                            details,
                                        });
                                    }
                                    AgentSessionEvent::Done { reply } => {
                                        let _ = sub_tx.send(CoreBridgeEvent::TurnCompleted(reply));
                                    }
                                    AgentSessionEvent::Error { message } => {
                                        let _ = sub_tx.send(CoreBridgeEvent::Error(message));
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            });

            while let Some(cmd) = cmd_rx.recv().await {
                match cmd {
                    CoreBridgeCommand::CreateSession { title } => {
                        if let Ok(new_sess) = create_session(
                            &worker_project_id,
                            &title,
                            None,
                            None,
                            None,
                        )
                        .await
                        {
                            active_sid = new_sess.id.clone();
                            if let Ok(mut guard) = current_sid_clone.lock() {
                                *guard = new_sess.id.clone();
                            }
                            let _ = worker_tx.send(CoreBridgeEvent::SessionCreated {
                                id: new_sess.id,
                                title: new_sess.title,
                            });
                        }
                    }
                    CoreBridgeCommand::SwitchSession { session_id } => {
                        active_sid = session_id.clone();
                        if let Ok(mut guard) = current_sid_clone.lock() {
                            *guard = session_id.clone();
                        }
                        if let Ok(msgs) = list_messages(&session_id).await {
                            let dtos = msgs.into_iter().map(PersistedMessageDto::from).collect();
                            let _ = worker_tx.send(CoreBridgeEvent::HistoryLoaded {
                                session_id,
                                messages: dtos,
                            });
                        }
                    }
                    CoreBridgeCommand::DeleteSession { session_id } => {
                        let _ = delete_session(&session_id).await;
                        let _ = worker_tx.send(CoreBridgeEvent::SessionDeleted {
                            id: session_id,
                        });
                    }
                    CoreBridgeCommand::LoadHistory { session_id } => {
                        if let Ok(msgs) = list_messages(&session_id).await {
                            let dtos = msgs.into_iter().map(PersistedMessageDto::from).collect();
                            let _ = worker_tx.send(CoreBridgeEvent::HistoryLoaded {
                                session_id,
                                messages: dtos,
                            });
                        }
                    }
                    CoreBridgeCommand::ExecuteTurn {
                        prompt,
                        model_override,
                    } => {
                        let current_turn_sid = active_sid.clone();
                        let _ = create_message(
                            &current_turn_sid,
                            "user",
                            Some(&prompt),
                            None,
                            None,
                            None,
                            "completed",
                            None,
                            None,
                        )
                        .await;

                        let sid_exec = current_turn_sid;
                        let exec_tx = worker_tx.clone();

                        tokio::spawn(async move {
                            match ragasukma_core::agent::executor::execute_session_turn(
                                &sid_exec,
                                model_override.as_deref(),
                            )
                            .await
                            {
                                Ok(reply) => {
                                    let _ = exec_tx.send(CoreBridgeEvent::TurnCompleted(reply));
                                }
                                Err(e) => {
                                    let _ = exec_tx.send(CoreBridgeEvent::Error(e.to_string()));
                                }
                            }
                        });
                    }
                    CoreBridgeCommand::RefreshQuota { provider } => {
                        let pool_ref = pool_clone.clone();
                        let quota_tx = worker_tx.clone();
                        let prov = provider.clone();
                        tokio::spawn(async move {
                            let summary = ragasukma_core::llm::usage::get_usage_summary_from_pool(
                                &pool_ref,
                                Some(&prov),
                                None,
                            )
                            .await;

                            let _ = quota_tx.send(CoreBridgeEvent::QuotaUpdated {
                                provider: prov,
                                input_tokens: summary.total_input_tokens,
                                output_tokens: summary.total_output_tokens,
                                total_tokens: summary.total_tokens,
                                requests: summary.total_requests,
                            });
                        });
                    }
                }
            }
        });

        Ok(Self {
            cmd_tx,
            project_id: pid,
            session_id: session.id,
        })
    }

    pub fn submit_prompt(&self, prompt: &str, model_override: Option<String>) -> Result<()> {
        self.cmd_tx
            .send(CoreBridgeCommand::ExecuteTurn {
                prompt: prompt.to_string(),
                model_override,
            })
            .map_err(|e| anyhow::anyhow!("Gagal mengirim prompt ke core bridge: {}", e))
    }

    pub fn switch_session(&self, session_id: &str) -> Result<()> {
        self.cmd_tx
            .send(CoreBridgeCommand::SwitchSession {
                session_id: session_id.to_string(),
            })
            .map_err(|e| anyhow::anyhow!("Gagal switch session: {}", e))
    }

    pub fn create_session(&self, title: &str) -> Result<()> {
        self.cmd_tx
            .send(CoreBridgeCommand::CreateSession {
                title: title.to_string(),
            })
            .map_err(|e| anyhow::anyhow!("Gagal membuat session baru: {}", e))
    }

    pub fn delete_session(&self, session_id: &str) -> Result<()> {
        self.cmd_tx
            .send(CoreBridgeCommand::DeleteSession {
                session_id: session_id.to_string(),
            })
            .map_err(|e| anyhow::anyhow!("Gagal menghapus session: {}", e))
    }

    pub fn refresh_quota(&self, provider: &str) -> Result<()> {
        self.cmd_tx
            .send(CoreBridgeCommand::RefreshQuota {
                provider: provider.to_string(),
            })
            .map_err(|e| anyhow::anyhow!("Gagal mengirim refresh quota: {}", e))
    }
}