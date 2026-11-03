use anyhow::Result;
use ragasukma_core::agent::events::get_session_sender;
use ragasukma_core::agent::models::{AgentSessionEvent, Message};
use ragasukma_core::agent::repository::{
    create_message, create_project, create_session, delete_project, delete_session, get_project,
    list_messages, list_projects, list_sessions,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectDto {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
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
    ProjectsLoaded {
        projects: Vec<ProjectDto>,
        active_id: String,
        active_name: String,
    },
    ProjectCreated {
        project: ProjectDto,
    },
    ProjectSwitched {
        project_id: String,
        project_name: String,
    },
    ProjectDeleted {
        id: String,
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
    TurnCancelled {
        session_id: String,
    },
    TurnFailed {
        session_id: String,
        error: String,
    },
    QuotaUpdated {
        provider: String,
        input_tokens: usize,
        output_tokens: usize,
        total_tokens: usize,
        requests: usize,
    },
    AccountQuotaUpdated {
        provider: String,
        account_id: String,
        quotas: Vec<crate::provider_view::ModelQuotaItem>,
    },
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreBridgeCommand {
    ExecuteTurn {
        prompt: String,
        model_override: Option<String>,
    },
    RefreshQuota {
        provider: String,
    },
    RefreshAccountQuota {
        provider: String,
        account_id: String,
    },
    CreateProject {
        name: String,
        description: Option<String>,
    },
    SwitchProject {
        project_id: String,
    },
    DeleteProject {
        project_id: String,
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
    CancelTurn,
    RetryTurn,
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
        let initial_projects = list_projects().await.unwrap_or_default();
        let project = if let Some(first) = initial_projects.first() {
            first.clone()
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

        // Emit initial project list
        let all_projects = list_projects().await.unwrap_or_default();
        let project_dtos = all_projects
            .into_iter()
            .map(|p| ProjectDto {
                id: p.id,
                name: p.name,
                description: Some(p.description),
            })
            .collect::<Vec<_>>();

        let _ = event_tx.send(CoreBridgeEvent::ProjectsLoaded {
            projects: project_dtos,
            active_id: pid.clone(),
            active_name: project.name.clone(),
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
        let mut worker_project_id = pid.clone();
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
                                        tracing::info!("Turn completed successfully");
                                        let _ = sub_tx.send(CoreBridgeEvent::TurnCompleted(reply));
                                    }
                                    AgentSessionEvent::Error { message } => {
                                        tracing::error!("Agent session error: {message}");
                                        let _ = sub_tx.send(CoreBridgeEvent::Error(message));
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            });

            let mut current_turn_handle: Option<tokio::task::JoinHandle<()>> = None;
            let mut last_turn_request: Option<(String, Option<String>)> = None;

            while let Some(cmd) = cmd_rx.recv().await {
                match cmd {
                    CoreBridgeCommand::CancelTurn => {
                        if let Some(handle) = current_turn_handle.take() {
                            handle.abort();
                            let _ = worker_tx.send(CoreBridgeEvent::TurnCancelled {
                                session_id: active_sid.clone(),
                            });
                        }
                    }
                    CoreBridgeCommand::RetryTurn => {
                        if let Some((_prompt, model_override)) = last_turn_request.clone() {
                            let sid_exec = active_sid.clone();
                            let exec_tx = worker_tx.clone();
                            let handle = tokio::spawn(async move {
                                tracing::info!("Retrying turn for session: {sid_exec}");
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
                                        tracing::error!("Retry turn error: {e:?}");
                                        let _ = exec_tx.send(CoreBridgeEvent::TurnFailed {
                                            session_id: sid_exec,
                                            error: e.to_string(),
                                        });
                                    }
                                }
                            });
                            current_turn_handle = Some(handle);
                        }
                    }
                    CoreBridgeCommand::CreateProject { name, description } => {
                        match create_project(&name, description.as_deref().unwrap_or(""), None, None).await {
                            Ok(new_proj) => {
                                let dto = ProjectDto {
                                    id: new_proj.id.clone(),
                                    name: new_proj.name.clone(),
                                    description: Some(new_proj.description.clone()),
                                };
                                let _ = worker_tx.send(CoreBridgeEvent::ProjectCreated { project: dto });
                                
                                // Auto switch to newly created project
                                worker_project_id = new_proj.id.clone();
                                let new_sess = match create_session(
                                    &worker_project_id,
                                    "Default Session",
                                    None,
                                    None,
                                    None,
                                ).await {
                                    Ok(s) => s,
                                    Err(_) => continue,
                                };
                                active_sid = new_sess.id.clone();
                                if let Ok(mut guard) = current_sid_clone.lock() {
                                    *guard = active_sid.clone();
                                }
                                let _ = worker_tx.send(CoreBridgeEvent::ProjectSwitched {
                                    project_id: worker_project_id.clone(),
                                    project_name: new_proj.name.clone(),
                                });
                                let _ = worker_tx.send(CoreBridgeEvent::SessionsLoaded {
                                    sessions: vec![(new_sess.id.clone(), new_sess.title)],
                                    active_id: new_sess.id,
                                });
                            }
                            Err(e) => {
                                tracing::error!("Failed to create project: {e}");
                            }
                        }
                    }
                    CoreBridgeCommand::SwitchProject { project_id } => {
                        worker_project_id = project_id.clone();
                        let project_name = get_project(&project_id)
                            .await
                            .ok()
                            .flatten()
                            .map(|p| p.name)
                            .unwrap_or_else(|| "Workspace".to_string());
                        
                        let sessions = list_sessions(&worker_project_id).await.unwrap_or_default();
                        let active_session = if let Some(first) = sessions.first() {
                            first.clone()
                        } else {
                            match create_session(
                                &worker_project_id,
                                "Default Session",
                                None,
                                None,
                                None,
                            ).await {
                                Ok(s) => s,
                                Err(_) => continue,
                            }
                        };
                        active_sid = active_session.id.clone();
                        if let Ok(mut guard) = current_sid_clone.lock() {
                            *guard = active_sid.clone();
                        }
                        let _ = worker_tx.send(CoreBridgeEvent::ProjectSwitched {
                            project_id: worker_project_id.clone(),
                            project_name,
                        });
                        let sessions_summary = if sessions.is_empty() {
                            vec![(active_session.id.clone(), active_session.title)]
                        } else {
                            sessions.into_iter().map(|s| (s.id, s.title)).collect()
                        };
                        let _ = worker_tx.send(CoreBridgeEvent::SessionsLoaded {
                            sessions: sessions_summary,
                            active_id: active_sid.clone(),
                        });
                        if let Ok(msgs) = list_messages(&active_sid).await {
                            let dtos = msgs.into_iter().map(PersistedMessageDto::from).collect();
                            let _ = worker_tx.send(CoreBridgeEvent::HistoryLoaded {
                                session_id: active_sid.clone(),
                                messages: dtos,
                            });
                        }
                    }
                    CoreBridgeCommand::DeleteProject { project_id } => {
                        let _ = delete_project(&project_id).await;
                        let _ = worker_tx.send(CoreBridgeEvent::ProjectDeleted {
                            id: project_id.clone(),
                        });
                        if worker_project_id == project_id {
                            // If deleted current project, switch to first available project
                            let rem_projects = list_projects().await.unwrap_or_default();
                            if let Some(first) = rem_projects.first() {
                                worker_project_id = first.id.clone();
                                let sessions = list_sessions(&worker_project_id).await.unwrap_or_default();
                                let first_sess = sessions.first().cloned();
                                if let Some(fs) = first_sess {
                                    active_sid = fs.id.clone();
                                    if let Ok(mut guard) = current_sid_clone.lock() {
                                        *guard = active_sid.clone();
                                    }
                                    let _ = worker_tx.send(CoreBridgeEvent::ProjectSwitched {
                                        project_id: worker_project_id.clone(),
                                        project_name: first.name.clone(),
                                    });
                                    let _ = worker_tx.send(CoreBridgeEvent::SessionsLoaded {
                                        sessions: sessions.into_iter().map(|s| (s.id, s.title)).collect(),
                                        active_id: active_sid.clone(),
                                    });
                                }
                            }
                        }
                    }
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

                        last_turn_request = Some((prompt.clone(), model_override.clone()));
                        let handle = tokio::spawn(async move {
                            tracing::info!("Executing session turn for session: {sid_exec}");
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
                                    tracing::error!("Execute session turn error: {e:?}");
                                    let _ = exec_tx.send(CoreBridgeEvent::TurnFailed {
                                        session_id: sid_exec,
                                        error: e.to_string(),
                                    });
                                }
                            }
                        });
                        current_turn_handle = Some(handle);
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
                    CoreBridgeCommand::RefreshAccountQuota { provider, account_id } => {
                        let quota_tx = worker_tx.clone();
                        let prov = provider.clone();
                        let acc_id = account_id.clone();
                        tokio::task::spawn_blocking(move || {
                            let mut quotas = Vec::new();
                            let mut err_msg = None;
                            match prov.to_lowercase().as_str() {
                                p if p.contains("antigravity") => {
                                    let mut provider = ragasukma_core::llm::providers::antigravity::AntigravityProvider::new();
                                    match provider.get_account_quota(&acc_id) {
                                        Ok(res) => {
                                            quotas = crate::provider_view::antigravity::map_antigravity_quotas(res);
                                        }
                                        Err(e) => {
                                            err_msg = Some(e.to_string());
                                        }
                                    }
                                }
                                p if p.contains("codex") => {
                                    let mut provider = ragasukma_core::llm::providers::codex::CodexProvider::new();
                                    match provider.get_account_quota(&acc_id) {
                                        Ok(res) => {
                                            quotas = crate::provider_view::codex::map_codex_quotas(res);
                                        }
                                        Err(e) => {
                                            err_msg = Some(e.to_string());
                                        }
                                    }
                                }
                                p if p.contains("claude") => {
                                    let mut provider = ragasukma_core::llm::providers::claude::ClaudeProvider::new();
                                    match provider.get_account_quota(&acc_id) {
                                        Ok(res) => {
                                            quotas = crate::provider_view::claude::map_claude_quotas(res);
                                        }
                                        Err(e) => {
                                            err_msg = Some(e.to_string());
                                        }
                                    }
                                }
                                _ => {}
                            }

                            if !quotas.is_empty() {
                                tracing::info!("Updated {} quotas for account: {} ({})", quotas.len(), acc_id, prov);
                                let _ = quota_tx.send(CoreBridgeEvent::AccountQuotaUpdated {
                                    provider: prov,
                                    account_id: acc_id,
                                    quotas,
                                });
                            } else if let Some(err) = err_msg {
                                tracing::warn!("Failed to fetch quota for account {} ({}): {}", acc_id, prov, err);
                                let _ = quota_tx.send(CoreBridgeEvent::StatusChange(format!(
                                    "Gagal memuat kuota {}: {}",
                                    prov, err
                                )));
                            }
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

    pub fn create_project(&self, name: &str, description: Option<String>) -> Result<()> {
        self.cmd_tx
            .send(CoreBridgeCommand::CreateProject {
                name: name.to_string(),
                description,
            })
            .map_err(|e| anyhow::anyhow!("Gagal membuat project baru: {}", e))
    }

    pub fn switch_project(&self, project_id: &str) -> Result<()> {
        self.cmd_tx
            .send(CoreBridgeCommand::SwitchProject {
                project_id: project_id.to_string(),
            })
            .map_err(|e| anyhow::anyhow!("Gagal switch project: {}", e))
    }

    pub fn delete_project(&self, project_id: &str) -> Result<()> {
        self.cmd_tx
            .send(CoreBridgeCommand::DeleteProject {
                project_id: project_id.to_string(),
            })
            .map_err(|e| anyhow::anyhow!("Gagal menghapus project: {}", e))
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