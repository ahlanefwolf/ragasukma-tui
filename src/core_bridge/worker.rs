use ragasukma_core::agent::events::{get_session_sender, is_session_task_running};
use ragasukma_core::agent::models::AgentSessionEvent;
use ragasukma_core::agent::repository::{
    create_message, create_project, create_session, delete_project, delete_session, get_project,
    list_messages, list_projects, list_sessions, update_session_model,
};
use sqlx::SqlitePool;
use tokio::sync::{mpsc, watch};

use super::types::{
    CoreBridgeCommand, CoreBridgeEvent, PersistedMessageDto, ProjectDto, SessionDto,
};

/// A collection of detached `JoinHandle`s owned by a single worker.
///
/// Used for two purposes:
/// 1. **Lifecycle** - when the worker is `abort()`ed, the `Drop` here also
///    cancels all detached tasks (`RefreshQuota`, quota `spawn_blocking`,
///    subscriber). Without this, those tasks keep running after the bridge is
///    gone and try to send to a channel that is already closed.
/// 2. **Housekeeping** - `push()` discards already-finished handles first, so
///    the `Vec` does not grow without bound when the user triggers many
///    refreshes.
///
/// Housekeeping is deliberately attached to `push`, not delegated to the
/// caller: previously `reap()` had to be called manually from the main loop and
/// was eventually forgotten, so the `Vec` leaked slowly. Because `push` is the
/// only entry point for new elements, cleaning up there makes that kind of leak
/// impossible to happen again.
///
/// `abort()` only stops the task; it does not wait, so it is safe to call from
/// a synchronous `Drop` (there is no `.await` inside Drop).
#[derive(Default)]
pub struct TaskGroup {
    handles: Vec<tokio::task::JoinHandle<()>>,
}

impl TaskGroup {
    pub fn new() -> Self {
        Self::default()
    }

    /// Take ownership of a `JoinHandle`.
    ///
    /// Already-finished handles are discarded first, so the length of the `Vec`
    /// reflects the number of tasks that are truly still alive.
    pub fn push(&mut self, handle: tokio::task::JoinHandle<()>) {
        self.handles.retain(|h| !h.is_finished());
        self.handles.push(handle);
    }

    /// True if there are no tasks still alive.
    pub fn is_empty(&mut self) -> bool {
        self.handles.retain(|h| !h.is_finished());
        self.handles.is_empty()
    }
}

/// A collection of turn tasks per session (`ExecuteTurn` / `RetryTurn`).
///
/// Responsible for:
/// 1. Ensuring there is only 1 turn task per `session_id` (the old task is aborted automatically).
/// 2. `cancel(session_id)` cancels the specific task and removes it from the collection.
/// 3. `Drop` cancels all still-alive tasks when the worker finishes/is cancelled,
///    so no orphan turn keeps running in the background.
#[derive(Default)]
pub struct ActiveTurns {
    turns: std::collections::HashMap<String, tokio::task::JoinHandle<()>>,
}

impl ActiveTurns {
    pub fn new() -> Self {
        Self::default()
    }

    /// Clean up handles that have already finished running.
    pub fn retain_active(&mut self) {
        self.turns.retain(|_, handle| !handle.is_finished());
    }

    /// Cancel the turn task for a specific session if present.
    pub fn cancel(&mut self, session_id: &str) -> bool {
        if let Some(handle) = self.turns.remove(session_id) {
            handle.abort();
            true
        } else {
            false
        }
    }

    /// Register a new turn task for a session. If the session already has a task,
    /// the old task is aborted first.
    pub fn insert(&mut self, session_id: String, handle: tokio::task::JoinHandle<()>) {
        if let Some(old) = self.turns.insert(session_id, handle) {
            old.abort();
        }
    }

    /// Check whether the turn for this session is currently active.
    #[cfg(test)]
    pub fn is_active(&mut self, session_id: &str) -> bool {
        self.retain_active();
        self.turns.contains_key(session_id)
    }

    /// True if there is no active turn task.
    #[cfg(test)]
    pub fn is_empty(&mut self) -> bool {
        self.retain_active();
        self.turns.is_empty()
    }
}

impl Drop for ActiveTurns {
    fn drop(&mut self) {
        for (_, handle) in self.turns.drain() {
            handle.abort();
        }
    }
}

impl Drop for TaskGroup {
    fn drop(&mut self) {
        for handle in self.handles.drain(..) {
            handle.abort();
        }
    }
}

pub fn map_core_event_to_bridge(sid: &str, event: AgentSessionEvent) -> Option<CoreBridgeEvent> {
    match event {
        AgentSessionEvent::Delta { delta } => Some(CoreBridgeEvent::Delta {
            session_id: sid.to_string(),
            delta,
        }),
        AgentSessionEvent::ThoughtDelta { delta } => Some(CoreBridgeEvent::ThoughtDelta {
            session_id: sid.to_string(),
            thought: delta,
        }),
        AgentSessionEvent::StatusChange { status } => Some(CoreBridgeEvent::StatusChange {
            session_id: Some(sid.to_string()),
            status,
        }),
        AgentSessionEvent::StepInfo {
            iteration,
            action,
            details,
        } => Some(CoreBridgeEvent::StepInfo {
            session_id: sid.to_string(),
            iteration,
            action,
            details,
        }),
        AgentSessionEvent::Error { message } => Some(CoreBridgeEvent::Error(message)),
        AgentSessionEvent::Done { reply: _ } => {
            tracing::info!("Agent session turn stream finished for: {sid}");
            None
        }
        _ => None,
    }
}

/// Forward `AgentSessionEvent` dari broadcast core ke `CoreBridgeEvent`.
///
/// The active session is read from `watch::Receiver` (single source of truth, see
/// `spawn_core_bridge_worker`). There is no polling: session changes trigger
/// `watch_rx.changed()`, sehingga subscriber hanya bangun saat ada event
/// event or when the active session actually changes.
///
/// **Lifecycle:** mengembalikan `JoinHandle` yang disimpan di `TaskGroup` milik
/// `spawn_core_bridge_worker`, sehingga ikut di-`abort()` saat bridge di-drop.
/// Subscriber juga berhenti sendiri bila `sub_tx` atau `watch_tx` sudah drop.
///
/// `active_status` is the worker-side cache of the active session status (shared via
/// `Arc<RwLock<String>>`). Cache ini dipakai sebagai *resync*: core hanya
/// broadcasts `StatusChange` on transitions, so if the subscriber subscribes too late
/// or the `running` transition event is lost, the TUI would never know the
/// session is running. Every `MessageCreated`/`StepInfo` is evidence the session
/// masih aktif, jadi cache di-`running`-kan lagi dan perbedaannya di-emit.
pub fn spawn_session_subscriber(
    sub_tx: mpsc::UnboundedSender<CoreBridgeEvent>,
    mut watch_rx: watch::Receiver<String>,
    active_status: std::sync::Arc<std::sync::RwLock<String>>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut active_sid = watch_rx.borrow_and_update().clone();
        let mut broadcast_rx = get_session_sender(&active_sid).subscribe();

        loop {
            if sub_tx.is_closed() {
                break;
            }

            tokio::select! {
                changed = watch_rx.changed() => {
                    if changed.is_err() {
                        // Sender dropped: the worker has stopped, nothing can
                        // change the active session anymore.
                        break;
                    }
                    let cur = watch_rx.borrow_and_update().clone();
                    if cur != active_sid {
                        active_sid = cur;
                        broadcast_rx = get_session_sender(&active_sid).subscribe();
                        // New session: sync the actual status of whether the background task is running,
                        // and emit the status transition to the UI so it syncs immediately.
                        let initial_st = if !active_sid.is_empty() && is_session_task_running(&active_sid) {
                            "running".to_string()
                        } else {
                            "idle".to_string()
                        };
                        if let Ok(mut st) = active_status.write() {
                            *st = initial_st.clone();
                        }
                        let _ = sub_tx.send(CoreBridgeEvent::StatusChange {
                            session_id: Some(active_sid.clone()),
                            status: initial_st,
                        });
                    }
                }
                evt = broadcast_rx.recv() => {
                    match evt {
                        Ok(event) => {
                            // An explicit status transition from the core is the source
                            // of truth: update the cache before the event is forwarded,
                            // so `resync_status` does not overwrite it back.
                            if let AgentSessionEvent::StatusChange { status } = &event
                                && let Ok(mut st) = active_status.write()
                            {
                                st.clone_from(status);
                            } else if matches!(
                                event,
                                AgentSessionEvent::MessageCreated(_) | AgentSessionEvent::StepInfo { .. }
                            ) {
                                // Activity evidence without a visible `running` transition
                                // (lag / late subscribe) -> recover.
                                resync_status(&sub_tx, &active_sid, &active_status);
                            }
                            if let Some(bridge_evt) = map_core_event_to_bridge(&active_sid, event)
                                && sub_tx.send(bridge_evt).is_err()
                            {
                                break;
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(missed)) => {
                            tracing::warn!("Session broadcast subscriber lagged by {missed} events");
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                            // Broadcaster closed (e.g. session finished or cleaned up).
                            // Create a new receiver immediately without a sleep/polling timer.
                            broadcast_rx = get_session_sender(&active_sid).subscribe();
                        }
                    }
                }
            }
        }
    })
}

/// Resync the active session status to `running` if the cache is still `idle`.
///
/// The core only broadcasts `StatusChange` on transitions; if the transition event
/// `running` is lost (subscriber lag / late subscribe), the TUI would be stuck in
/// `idle` even though the turn is running. Activity evidence (`MessageCreated`,
/// `StepInfo`) dipakai untuk memulihkan state tersebut.
fn resync_status(
    sub_tx: &mpsc::UnboundedSender<CoreBridgeEvent>,
    sid: &str,
    active_status: &std::sync::RwLock<String>,
) {
    let Ok(mut st) = active_status.write() else {
        return;
    };
    if st.as_str() == "running" {
        return;
    }
    *st = "running".to_string();
    let _ = sub_tx.send(CoreBridgeEvent::StatusChange {
        session_id: Some(sid.to_string()),
        status: "running".to_string(),
    });
}

/// Explicitly set the active session status from the worker side (not from activity
/// evidence) and emit `StatusChange` only if the value changed.
///
/// This is the counterpart of [`resync_status`]: resync restores the status from the event stream,
/// while this function writes the truth known by the worker itself
/// (turn started -> `running`, turn cancelled -> `idle`). Both share the same cache
/// so transitions are not double-emitted.
fn set_active_status(
    sub_tx: &mpsc::UnboundedSender<CoreBridgeEvent>,
    sid: &str,
    active_status: &std::sync::RwLock<String>,
    status: &str,
) {
    let Ok(mut st) = active_status.write() else {
        return;
    };
    if st.as_str() == status {
        return;
    }
    *st = status.to_string();
    let _ = sub_tx.send(CoreBridgeEvent::StatusChange {
        session_id: Some(sid.to_string()),
        status: status.to_string(),
    });
}

async fn execute_turn_pipeline(
    session_id: String,
    model_override: Option<String>,
    worker_tx: mpsc::UnboundedSender<CoreBridgeEvent>,
    is_retry: bool,
) {
    if is_retry {
        tracing::info!("Retrying turn for session: {session_id}");
    } else {
        tracing::info!("Executing session turn for session: {session_id}");
    }

    let res = ragasukma_core::agent::executor::execute_session_turn(
        &session_id,
        model_override.as_deref(),
    )
    .await;

    match res {
        Ok(reply) => {
            let _ = worker_tx.send(CoreBridgeEvent::TurnCompleted {
                session_id,
                summary: reply,
            });
        }
        Err(e) => {
            if is_retry {
                tracing::error!("Retry turn error: {e:?}");
            } else {
                tracing::error!("Execute session turn error: {e:?}");
            }
            let _ = worker_tx.send(CoreBridgeEvent::TurnFailed {
                session_id,
                error: e.to_string(),
            });
        }
    }
}

/// Helper untuk memancarkan hasil pembaruan kuota pemakaian model (T8 / Fase 5.2).
fn emit_usage_quota_result(
    tx: &mpsc::UnboundedSender<CoreBridgeEvent>,
    provider: String,
    summary: ragasukma_core::llm::usage::LlmUsageSummary,
) {
    let _ = tx.send(CoreBridgeEvent::QuotaUpdated {
        provider,
        input_tokens: summary.total_input_tokens,
        output_tokens: summary.total_output_tokens,
        total_tokens: summary.total_tokens,
        requests: summary.total_requests,
    });
}

/// Helper untuk memancarkan hasil pembaruan kuota per akun provider (T8 / Fase 5.2).
fn emit_account_quota_result(
    tx: &mpsc::UnboundedSender<CoreBridgeEvent>,
    provider: String,
    account_id: String,
    result: Result<Vec<crate::provider_view::ModelQuotaItem>, String>,
) {
    match result {
        Ok(quotas) if !quotas.is_empty() => {
            tracing::info!(
                "Updated {} quotas for account: {} ({})",
                quotas.len(),
                account_id,
                provider
            );
            let _ = tx.send(CoreBridgeEvent::AccountQuotaUpdated {
                provider,
                account_id,
                quotas,
            });
        }
        Ok(_) => {}
        Err(err) => {
            tracing::warn!(
                "Failed to fetch quota for account {} ({}): {}",
                account_id,
                provider,
                err
            );
            let _ = tx.send(CoreBridgeEvent::StatusChange {
                session_id: None,
                status: format!("Failed to load quota for {}: {}", provider, err),
            });
        }
    }
}

pub fn spawn_core_bridge_worker(
    mut cmd_rx: mpsc::UnboundedReceiver<CoreBridgeCommand>,
    worker_tx: mpsc::UnboundedSender<CoreBridgeEvent>,
    watch_tx: watch::Sender<String>,
    mut worker_project_id: String,
    initial_sid: String,
    pool: SqlitePool,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut active_sid = initial_sid;
        // Active session status cache — shared between the worker (explicit status
        // writer) and the subscriber (resync based on activity evidence).
        let active_status = std::sync::Arc::new(std::sync::RwLock::new("idle".to_string()));

        // Spawn the subscriber loop (the active session is monitored via a watch channel).
        // The handle is stored in `tasks` so the subscriber also stops when the worker
        // is cancelled — without this it lives on as an orphan task.
        let mut tasks = TaskGroup::new();
        tasks.push(spawn_session_subscriber(
            worker_tx.clone(),
            watch_tx.subscribe(),
            active_status.clone(),
        ));

        let mut active_turns = ActiveTurns::new();
        let mut last_turn_request: Option<(String, Option<String>)> = None;

        while let Some(cmd) = cmd_rx.recv().await {
            // Clean up finished turn handles
            active_turns.retain_active();

            match cmd {
                CoreBridgeCommand::CancelTurn { session_id } => {
                    // Without an explicit session_id —> cancel the turn on the active session.
                    let sid = match session_id {
                        Some(sid) => sid,
                        None => {
                            set_active_status(&worker_tx, &active_sid, &active_status, "idle");
                            active_sid.clone()
                        }
                    };
                    active_turns.cancel(&sid);
                }
                CoreBridgeCommand::RetryTurn => {
                    if let Some((_retry_prompt, model_override)) = last_turn_request.clone() {
                        let sid_exec = active_sid.clone();
                        let exec_tx = worker_tx.clone();
                        let sid_key = sid_exec.clone();

                        let _ = worker_tx.send(CoreBridgeEvent::StatusChange {
                            session_id: Some(sid_exec.clone()),
                            status: "running".to_string(),
                        });

                        let handle = tokio::spawn(async move {
                            execute_turn_pipeline(
                                sid_exec,
                                model_override,
                                exec_tx,
                                true,
                            )
                            .await;
                        });
                        active_turns.insert(sid_key, handle);
                    } else {
                        let _ = worker_tx.send(CoreBridgeEvent::Error(
                            "Cannot retry: no previous prompt turn found".to_string(),
                        ));
                    }
                }
                CoreBridgeCommand::CreateProject {
                    name,
                    description,
                    root_path,
                } => {
                    // Normalize & init git/gitignore for folders like ai-voice
                    if let Some(ref path_str) = root_path {
                        let parsed =
                            ragasukma_core::agent::executor::helpers::parse_project_folders(Some(
                                path_str,
                            ));
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
                            let _ = watch_tx.send(active_sid.clone());
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
                    let _ =
                        sqlx::query("UPDATE projects SET updated_at = unixepoch() WHERE id = ?1")
                            .bind(&worker_project_id)
                            .execute(&pool)
                            .await;
                    let prj_opt = get_project(&worker_project_id).await.ok().flatten();
                    let project_name = prj_opt
                        .as_ref()
                        .map(|p| p.name.clone())
                        .unwrap_or_else(|| "Project".to_string());
                    let project_root_path = prj_opt.as_ref().and_then(|p| p.root_path.clone());
                    let mut sessions = list_sessions(&worker_project_id).await.unwrap_or_default();
                    sessions.sort_by_key(|a| std::cmp::Reverse(a.updated_at));
                    let active_session = sessions.first().cloned();
                    active_sid = active_session
                        .as_ref()
                        .map(|s| s.id.clone())
                        .unwrap_or_default();
                    let _ = watch_tx.send(active_sid.clone());
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
                            let mut sessions =
                                list_sessions(&worker_project_id).await.unwrap_or_default();
                            sessions.sort_by_key(|a| std::cmp::Reverse(a.updated_at));
                            let first_sess = sessions.first().cloned();
                            if let Some(fs) = first_sess {
                                active_sid = fs.id.clone();
                                let _ = watch_tx.send(active_sid.clone());
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
                    if let Ok(new_sess) =
                        create_session(&worker_project_id, &title, model.as_deref(), None, None)
                            .await
                    {
                        active_sid = new_sess.id.clone();
                        let _ = watch_tx.send(new_sess.id.clone());
                        let _ = worker_tx.send(CoreBridgeEvent::SessionCreated {
                            id: new_sess.id,
                            title: new_sess.title,
                        });
                    }
                }
                CoreBridgeCommand::SwitchSession { session_id } => {
                    active_sid = session_id.clone();
                    let _ = watch_tx.send(session_id.clone());
                    let _ =
                        sqlx::query("UPDATE sessions SET updated_at = unixepoch() WHERE id = ?1")
                            .bind(&active_sid)
                            .execute(&pool)
                            .await;
                    let _ =
                        sqlx::query("UPDATE projects SET updated_at = unixepoch() WHERE id = ?1")
                            .bind(&worker_project_id)
                            .execute(&pool)
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
                    let _ =
                        sqlx::query("UPDATE sessions SET updated_at = unixepoch() WHERE id = ?1")
                            .bind(&session_id)
                            .execute(&pool)
                            .await;
                }
                CoreBridgeCommand::DeleteSession { session_id } => {
                    let _ = delete_session(&session_id).await;
                    let _ = worker_tx.send(CoreBridgeEvent::SessionDeleted { id: session_id });
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
                    // The transition to `running` is announced by the worker, not the subscriber.
                    //
                    // The subscriber only resyncs the status when it sees activity evidence
                    // (`MessageCreated`/`StepInfo`), sedangkan `user` message ditulis
                    // directly to the DB (without broadcast) and `StepInfo("thinking")` only
                    // appears once the turn pipeline is ready. As a result the TUI briefly shows
                    // the old status during the first seconds of the turn, and `CancelTurn`
                    // looks like a no-op. Setting the cache + emitting here makes the status
                    // correct from the moment the command is received.
                    set_active_status(&worker_tx, &current_turn_sid, &active_status, "running");
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

                    let sid_key = sid_exec.clone();
                    let handle = tokio::spawn(async move {
                        execute_turn_pipeline(
                            sid_exec,
                            model_override,
                            exec_tx,
                            false,
                        )
                        .await;
                    });
                    active_turns.insert(sid_key, handle);
                }
                CoreBridgeCommand::RefreshQuota { provider } => {
                    let pool = pool.clone();
                    let quota_tx = worker_tx.clone();
                    let prov = provider.clone();
                    tasks.push(tokio::spawn(async move {
                        let summary = ragasukma_core::llm::usage::get_usage_summary_from_pool(
                            &pool,
                            Some(&prov),
                            None,
                        )
                        .await;

                        emit_usage_quota_result(&quota_tx, prov, summary);
                    }));
                }
                CoreBridgeCommand::RefreshAccountQuota {
                    provider,
                    account_id,
                } => {
                    let quota_tx = worker_tx.clone();
                    let prov = provider.clone();
                    let acc_id = account_id.clone();
                    tasks.push(tokio::task::spawn_blocking(move || {
                        let result = match prov.to_lowercase().as_str() {
                            p if p.contains("antigravity") => {
                                let mut provider = ragasukma_core::llm::providers::antigravity::AntigravityProvider::new();
                                provider
                                    .get_account_quota(&acc_id)
                                    .map(crate::provider_view::antigravity::map_antigravity_quotas)
                                    .map_err(|e| e.to_string())
                            }
                            p if p.contains("codex") => {
                                let mut provider =
                                    ragasukma_core::llm::providers::codex::CodexProvider::new();
                                provider
                                    .get_account_quota(&acc_id)
                                    .map(|res| {
                                        let models = provider
                                            .get_dynamic_available_models()
                                            .unwrap_or_default();
                                        crate::provider_view::codex::map_codex_quotas(res, models)
                                    })
                                    .map_err(|e| e.to_string())
                            }
                            p if p.contains("claude") => {
                                let mut provider =
                                    ragasukma_core::llm::providers::claude::ClaudeProvider::new();
                                provider
                                    .get_account_quota(&acc_id)
                                    .map(|res| {
                                        let models = provider
                                            .get_dynamic_available_models()
                                            .unwrap_or_default();
                                        crate::provider_view::claude::map_claude_quotas(res, models)
                                    })
                                    .map_err(|e| e.to_string())
                            }
                            _ => Ok(Vec::new()),
                        };

                        emit_account_quota_result(&quota_tx, prov, acc_id, result);
                    }));
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::{ActiveTurns, TaskGroup};
    use std::time::Duration;

    /// `push` must discard finished handles so the `Vec` does not
    /// grow unbounded when the user triggers refresh repeatedly.
    #[tokio::test]
    async fn push_discards_finished_handles() {
        let mut group = TaskGroup::new();

        for _ in 0..50 {
            group.push(tokio::spawn(async {}));
        }
        // Give tasks a chance to finish, then trigger housekeeping via `push`.
        tokio::time::sleep(Duration::from_millis(20)).await;
        group.push(tokio::spawn(async {}));
        tokio::time::sleep(Duration::from_millis(20)).await;

        assert!(
            group.is_empty(),
            "finished handles must be discarded by push(), not accumulate"
        );
    }

    /// Still-alive tasks must not be discarded by housekeeping.
    #[tokio::test]
    async fn push_retains_still_alive_tasks() {
        let (release, wait) = tokio::sync::oneshot::channel::<()>();
        let mut group = TaskGroup::new();
        group.push(tokio::spawn(async move {
            let _ = wait.await;
        }));
        group.push(tokio::spawn(async {}));
        tokio::time::sleep(Duration::from_millis(20)).await;

        // Only the first task (waiting on `release`) remains.
        assert!(
            !group.is_empty(),
            "unfinished tasks must not be discarded"
        );
        let _ = release.send(());
    }

    #[tokio::test]
    async fn switch_session_twice_in_a_row_no_stray_events() {
        use crate::core_bridge::types::CoreBridgeEvent;
        use crate::core_bridge::worker::spawn_session_subscriber;
        use ragasukma_core::agent::events::get_session_sender;
        use ragasukma_core::agent::models::AgentSessionEvent;
        use std::sync::{Arc, RwLock};
        use std::time::Duration;
        use tokio::sync::{mpsc, watch};
        use tokio::time::timeout;

        let sid1 = "test-session-1".to_string();
        let sid2 = "test-session-2".to_string();
        let sid3 = "test-session-3".to_string();

        let (watch_tx, watch_rx) = watch::channel(sid1.clone());
        let (sub_tx, mut sub_rx) = mpsc::unbounded_channel();
        let active_status = Arc::new(RwLock::new("idle".to_string()));

        let handle = spawn_session_subscriber(sub_tx, watch_rx, active_status);

        // Give the subscriber time to start and subscribe to sid1
        tokio::time::sleep(Duration::from_millis(20)).await;

        let sender1 = get_session_sender(&sid1);
        let sender2 = get_session_sender(&sid2);
        let sender3 = get_session_sender(&sid3);

        // 1. Emit on sid1, must be received
        let _ = sender1.send(AgentSessionEvent::Delta {
            delta: "hello-1".to_string(),
        });
        let mut got_hello1 = false;
        while let Ok(Some(evt)) = timeout(Duration::from_millis(100), sub_rx.recv()).await {
            if let CoreBridgeEvent::Delta { session_id, delta } = evt
                && session_id == sid1
                && delta == "hello-1"
            {
                got_hello1 = true;
                break;
            }
        }
        assert!(got_hello1, "must receive delta from sid1");

        // 2. Switch to sid2 (switch #1)
        watch_tx.send(sid2.clone()).unwrap();
        tokio::time::sleep(Duration::from_millis(20)).await;

        // Send an event on sid1 (old session) and sid2 (new session)
        let _ = sender1.send(AgentSessionEvent::Delta {
            delta: "stray-1".to_string(),
        });
        let _ = sender2.send(AgentSessionEvent::Delta {
            delta: "hello-2".to_string(),
        });

        let mut got_hello2 = false;
        let mut got_stray1 = false;
        while let Ok(Some(evt)) = timeout(Duration::from_millis(100), sub_rx.recv()).await {
            if let CoreBridgeEvent::Delta { session_id, delta } = evt {
                if delta == "stray-1" {
                    got_stray1 = true;
                }
                if session_id == sid2 && delta == "hello-2" {
                    got_hello2 = true;
                    break;
                }
            }
        }
        assert!(!got_stray1, "must not receive stray events from sid1");
        assert!(got_hello2, "must receive delta from sid2");

        // 3. Switch to sid3 (switch #2 in a row)
        watch_tx.send(sid3.clone()).unwrap();
        tokio::time::sleep(Duration::from_millis(20)).await;

        // Send an event on sid2 (old session) and sid3 (new session)
        let _ = sender2.send(AgentSessionEvent::Delta {
            delta: "stray-2".to_string(),
        });
        let _ = sender3.send(AgentSessionEvent::Delta {
            delta: "hello-3".to_string(),
        });

        let mut got_hello3 = false;
        let mut got_stray2 = false;
        while let Ok(Some(evt)) = timeout(Duration::from_millis(100), sub_rx.recv()).await {
            if let CoreBridgeEvent::Delta { session_id, delta } = evt {
                if delta == "stray-2" {
                    got_stray2 = true;
                }
                if session_id == sid3 && delta == "hello-3" {
                    got_hello3 = true;
                    break;
                }
            }
        }
        assert!(!got_stray2, "must not receive stray events from sid2");
        assert!(got_hello3, "must receive delta from sid3");

        handle.abort();
    }

    #[tokio::test]
    async fn drop_task_group_and_active_turns_cancel_all_tasks() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        use std::time::Duration;

        let flag_tg = Arc::new(AtomicBool::new(false));
        let flag_at = Arc::new(AtomicBool::new(false));

        {
            let mut tg = TaskGroup::new();
            let mut at = ActiveTurns::new();

            let f1 = flag_tg.clone();
            let h1 = tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(10)).await;
                f1.store(true, Ordering::SeqCst);
            });
            tg.push(h1);

            let f2 = flag_at.clone();
            let h2 = tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(10)).await;
                f2.store(true, Ordering::SeqCst);
            });
            at.insert("sess-drop-test".to_string(), h2);

            // At this point both tasks are running.
            // On leaving the block, tg and at are dropped.
        }

        // Give a short delay for the abort signal to be processed
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Neither flag must ever be set to true because the tasks are aborted on drop
        assert!(
            !flag_tg.load(Ordering::SeqCst),
            "TaskGroup must cancel tasks when dropped"
        );
        assert!(
            !flag_at.load(Ordering::SeqCst),
            "ActiveTurns must cancel turns when dropped"
        );
    }
}