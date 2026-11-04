use ragasukma_core::agent::events::get_session_sender;
use ragasukma_core::agent::models::AgentSessionEvent;
use ragasukma_core::agent::repository::{
    create_message, create_project, create_session, delete_project, delete_session, get_project,
    list_messages, list_projects, list_sessions, update_session_model,
};
use sqlx::SqlitePool;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

use super::types::{CoreBridgeCommand, CoreBridgeEvent, PersistedMessageDto, ProjectDto, SessionDto};

pub fn spawn_session_subscriber(
    sub_tx: mpsc::UnboundedSender<CoreBridgeEvent>,
    sub_sid: Arc<Mutex<String>>,
) {
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
}

fn detect_provider(model: &str) -> &'static str {
    let lower = model.to_lowercase();
    if lower.contains("claude") {
        "claude"
    } else if lower.contains("codex") || lower.contains("openai") || lower.contains("gpt") {
        "codex"
    } else {
        "antigravity"
    }
}

async fn record_turn_usage(
    pool: &SqlitePool,
    model: &str,
    input_text: &str,
    output_text: &str,
    duration_ms: u64,
    status: &str,
) {
    let input_tokens = (input_text.len() / 4).max(1);
    let output_tokens = output_text.len() / 4;
    let provider = detect_provider(model);

    if let Err(e) = ragasukma_core::llm::usage::record_usage_to_pool(
        pool,
        provider,
        model,
        input_tokens,
        0,
        output_tokens,
        duration_ms,
        status,
    )
    .await
    {
        tracing::warn!("Failed to record turn usage to pool: {e}");
    }
}

pub fn spawn_core_bridge_worker(
    mut cmd_rx: mpsc::UnboundedReceiver<CoreBridgeCommand>,
    worker_tx: mpsc::UnboundedSender<CoreBridgeEvent>,
    current_sid_clone: Arc<Mutex<String>>,
    mut worker_project_id: String,
    mut active_sid: String,
    db_pool: SqlitePool,
) {
    tokio::spawn(async move {
        let pool_clone = db_pool.clone();

        // Spawn subscriber loop
        spawn_session_subscriber(worker_tx.clone(), Arc::clone(&current_sid_clone));

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
                    if let Some((retry_prompt, model_override)) = last_turn_request.clone() {
                        let sid_exec = active_sid.clone();
                        let exec_tx = worker_tx.clone();
                        let last_prompt = retry_prompt;

                        let mut session_sub = ragasukma_core::agent::events::get_session_sender(&sid_exec).subscribe();
                        let stream_tx = exec_tx.clone();
                        let stream_forwarder = tokio::spawn(async move {
                            while let Ok(evt) = session_sub.recv().await {
                                match evt {
                                    ragasukma_core::agent::models::AgentSessionEvent::Delta { delta } => {
                                        let _ = stream_tx.send(CoreBridgeEvent::Delta(delta));
                                    }
                                    ragasukma_core::agent::models::AgentSessionEvent::ThoughtDelta { delta } => {
                                        let _ = stream_tx.send(CoreBridgeEvent::ThoughtDelta(delta));
                                    }
                                    ragasukma_core::agent::models::AgentSessionEvent::StatusChange { status } => {
                                        let _ = stream_tx.send(CoreBridgeEvent::StatusChange(status));
                                    }
                                    ragasukma_core::agent::models::AgentSessionEvent::StepInfo { iteration, action, details } => {
                                        let _ = stream_tx.send(CoreBridgeEvent::StepInfo { iteration, action, details });
                                    }
                                    _ => {}
                                }
                            }
                        });

                        let pool_exec = pool_clone.clone();
                        let handle = tokio::spawn(async move {
                            tracing::info!("Retrying turn for session: {sid_exec}");
                            let turn_start = std::time::Instant::now();
                            let res = ragasukma_core::agent::executor::execute_session_turn(
                                &sid_exec,
                                model_override.as_deref(),
                            )
                            .await;

                            stream_forwarder.abort();
                            let duration_ms = turn_start.elapsed().as_millis() as u64;
                            let chosen_model = model_override.as_deref().unwrap_or("antigravity");

                            match res {
                                Ok(reply) => {
                                    record_turn_usage(
                                        &pool_exec,
                                        chosen_model,
                                        &last_prompt,
                                        &reply,
                                        duration_ms,
                                        "success",
                                    )
                                    .await;
                                    let _ = exec_tx.send(CoreBridgeEvent::TurnCompleted(reply));
                                }
                                Err(e) => {
                                    tracing::error!("Retry turn error: {e:?}");
                                    record_turn_usage(
                                        &pool_exec,
                                        chosen_model,
                                        &last_prompt,
                                        "",
                                        duration_ms,
                                        "error",
                                    )
                                    .await;
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
                CoreBridgeCommand::CreateProject {
                    name,
                    description,
                    root_path,
                } => {
                    // Normalize & init git/gitignore for folders like ai-voice
                    if let Some(ref path_str) = root_path {
                        let parsed = ragasukma_core::agent::executor::helpers::parse_project_folders(Some(path_str));
                        for folder in &parsed {
                            let p = std::path::Path::new(folder);
                            if p.is_dir() {
                                let _ = ragasukma_core::agent::tools::ensure_git_and_gitignore(p);
                            }
                        }
                    }

                    match create_project(
                    &name,
                    description.as_deref().unwrap_or(""),
                    root_path.as_deref(),
                    None,
                )
                .await
                    {
                        Ok(new_proj) => {
                            let dto = ProjectDto {
                                id: new_proj.id.clone(),
                                name: new_proj.name.clone(),
                                description: Some(new_proj.description.clone()),
                                root_path: new_proj.root_path.clone(),
                            };
                            let _ =
                                worker_tx.send(CoreBridgeEvent::ProjectCreated { project: dto });

                            // Auto switch to newly created project
                            worker_project_id = new_proj.id.clone();
                            active_sid = String::new();
                            if let Ok(mut guard) = current_sid_clone.lock() {
                                *guard = active_sid.clone();
                            }
                            let _ = worker_tx.send(CoreBridgeEvent::ProjectSwitched {
                                project_id: worker_project_id.clone(),
                                project_name: new_proj.name.clone(),
                                root_path: new_proj.root_path.clone(),
                            });
                            let _ = worker_tx.send(CoreBridgeEvent::SessionsLoaded {
                                sessions: Vec::new(),
                                active_id: String::new(),
                            });
                        }
                        Err(e) => {
                            tracing::error!("Failed to create project: {e}");
                        }
                    }
                }
                CoreBridgeCommand::UpdateProjectFolders {
                    project_id,
                    folders,
                } => {
                    let mut valid_folders: Vec<String> = Vec::new();
                    for f in &folders {
                        let p = std::path::Path::new(f);
                        if p.is_dir() {
                            let _ = ragasukma_core::agent::tools::ensure_git_and_gitignore(p);
                            let canonical = p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
                            valid_folders.push(canonical.to_string_lossy().to_string());
                        }
                    }

                    let root_path_json = if valid_folders.is_empty() {
                        None
                    } else {
                        serde_json::to_string(&valid_folders).ok()
                    };
                    if let Ok(Some(proj)) = get_project(&project_id).await {
                        if let Err(e) = ragasukma_core::agent::repository::update_project(
                            &proj.id,
                            &proj.name,
                            &proj.description,
                            root_path_json.as_deref(),
                            proj.custom_instructions.as_deref(),
                        )
                        .await
                        {
                            tracing::error!("Failed to update project folders: {e}");
                        } else if let Ok(projects) = list_projects().await {
                            let active_name = proj.name;
                            let project_dtos: Vec<ProjectDto> = projects
                                .into_iter()
                                .map(|p| ProjectDto {
                                    id: p.id,
                                    name: p.name,
                                    description: Some(p.description),
                                    root_path: p.root_path,
                                })
                                .collect();
                            let _ = worker_tx.send(CoreBridgeEvent::ProjectsLoaded {
                                projects: project_dtos,
                                active_id: project_id.clone(),
                                active_name,
                            });
                        }
                    }
                }
                CoreBridgeCommand::SwitchProject { project_id } => {
                    worker_project_id = project_id.clone();
                    let _ = sqlx::query("UPDATE projects SET updated_at = unixepoch() WHERE id = ?1")
                        .bind(&worker_project_id)
                        .execute(&pool_clone)
                        .await;
                    let prj_opt = get_project(&worker_project_id).await.ok().flatten();
                    let project_name = prj_opt.as_ref().map(|p| p.name.clone()).unwrap_or_else(|| "Project".to_string());
                    let project_root_path = prj_opt.as_ref().and_then(|p| p.root_path.clone());
                    let mut sessions = list_sessions(&worker_project_id).await.unwrap_or_default();
                    sessions.sort_by_key(|a| std::cmp::Reverse(a.updated_at));
                    let active_session = sessions.first().cloned();
                    active_sid = active_session.as_ref().map(|s| s.id.clone()).unwrap_or_default();
                    if let Ok(mut guard) = current_sid_clone.lock() {
                        *guard = active_sid.clone();
                    }
                    let _ = worker_tx.send(CoreBridgeEvent::ProjectSwitched {
                        project_id: worker_project_id.clone(),
                        project_name,
                        root_path: project_root_path,
                    });
                    let sessions_summary = sessions.into_iter().map(SessionDto::from).collect();
                    let _ = worker_tx.send(CoreBridgeEvent::SessionsLoaded {
                        sessions: sessions_summary,
                        active_id: active_sid.clone(),
                    });
                    if !active_sid.is_empty()
                        && let Ok(msgs) = list_messages(&active_sid).await
                    {
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
                        let mut rem_projects = list_projects().await.unwrap_or_default();
                        rem_projects.sort_by_key(|a| std::cmp::Reverse(a.updated_at));
                        if let Some(first) = rem_projects.first() {
                            worker_project_id = first.id.clone();
                            let mut sessions = list_sessions(&worker_project_id).await.unwrap_or_default();
                            sessions.sort_by_key(|a| std::cmp::Reverse(a.updated_at));
                            let first_sess = sessions.first().cloned();
                            if let Some(fs) = first_sess {
                                active_sid = fs.id.clone();
                                if let Ok(mut guard) = current_sid_clone.lock() {
                                    *guard = active_sid.clone();
                                }
                                let _ = worker_tx.send(CoreBridgeEvent::ProjectSwitched {
                                    project_id: worker_project_id.clone(),
                                    project_name: first.name.clone(),
                                    root_path: first.root_path.clone(),
                                });
                                let _ = worker_tx.send(CoreBridgeEvent::SessionsLoaded {
                                    sessions: sessions.into_iter().map(SessionDto::from).collect(),
                                    active_id: active_sid.clone(),
                                });
                            }
                        }
                    }
                }
                CoreBridgeCommand::CreateSession { title, model } => {
                    if let Ok(new_sess) = create_session(
                        &worker_project_id,
                        &title,
                        model.as_deref(),
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
                    let _ = sqlx::query("UPDATE sessions SET updated_at = unixepoch() WHERE id = ?1")
                        .bind(&active_sid)
                        .execute(&pool_clone)
                        .await;
                    let _ = sqlx::query("UPDATE projects SET updated_at = unixepoch() WHERE id = ?1")
                        .bind(&worker_project_id)
                        .execute(&pool_clone)
                        .await;
                    if let Ok(msgs) = list_messages(&session_id).await {
                        let dtos = msgs.into_iter().map(PersistedMessageDto::from).collect();
                        let _ = worker_tx.send(CoreBridgeEvent::HistoryLoaded {
                            session_id,
                            messages: dtos,
                        });
                    }
                }
                CoreBridgeCommand::UpdateSessionModel { session_id, model } => {
                    let _ = update_session_model(&session_id, &model).await;
                    let _ = sqlx::query("UPDATE sessions SET updated_at = unixepoch() WHERE id = ?1")
                        .bind(&session_id)
                        .execute(&pool_clone)
                        .await;
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

                    // Realtime event streamer: subscribe to core agent session events
                    let mut session_sub = ragasukma_core::agent::events::get_session_sender(&sid_exec).subscribe();
                    let stream_tx = exec_tx.clone();
                    let stream_forwarder = tokio::spawn(async move {
                        while let Ok(evt) = session_sub.recv().await {
                            match evt {
                                ragasukma_core::agent::models::AgentSessionEvent::Delta { delta } => {
                                    let _ = stream_tx.send(CoreBridgeEvent::Delta(delta));
                                }
                                ragasukma_core::agent::models::AgentSessionEvent::ThoughtDelta { delta } => {
                                    let _ = stream_tx.send(CoreBridgeEvent::ThoughtDelta(delta));
                                }
                                ragasukma_core::agent::models::AgentSessionEvent::StatusChange { status } => {
                                    let _ = stream_tx.send(CoreBridgeEvent::StatusChange(status));
                                }
                                ragasukma_core::agent::models::AgentSessionEvent::StepInfo { iteration, action, details } => {
                                    let _ = stream_tx.send(CoreBridgeEvent::StepInfo { iteration, action, details });
                                }
                                _ => {}
                            }
                        }
                    });

                    let pool_exec = pool_clone.clone();
                    let prompt_for_usage = prompt.clone();
                    let handle = tokio::spawn(async move {
                        tracing::info!("Executing session turn for session: {sid_exec}");
                        let turn_start = std::time::Instant::now();
                        let res = ragasukma_core::agent::executor::execute_session_turn(
                            &sid_exec,
                            model_override.as_deref(),
                        )
                        .await;

                        stream_forwarder.abort();
                        let duration_ms = turn_start.elapsed().as_millis() as u64;
                        let chosen_model = model_override.as_deref().unwrap_or("antigravity");

                        match res {
                            Ok(reply) => {
                                record_turn_usage(
                                    &pool_exec,
                                    chosen_model,
                                    &prompt_for_usage,
                                    &reply,
                                    duration_ms,
                                    "success",
                                )
                                .await;
                                let _ = exec_tx.send(CoreBridgeEvent::TurnCompleted(reply));
                            }
                            Err(e) => {
                                tracing::error!("Execute session turn error: {e:?}");
                                record_turn_usage(
                                    &pool_exec,
                                    chosen_model,
                                    &prompt_for_usage,
                                    "",
                                    duration_ms,
                                    "error",
                                )
                                .await;
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
                                        let models = provider.get_dynamic_available_models().unwrap_or_default();
                                        quotas = crate::provider_view::codex::map_codex_quotas(res, models);
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
                                        let models = provider.get_dynamic_available_models().unwrap_or_default();
                                        quotas = crate::provider_view::claude::map_claude_quotas(res, models);
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
}