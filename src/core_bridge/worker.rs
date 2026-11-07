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

/// Kumpulan `JoinHandle` tak-terkait yang dimiliki satu worker.
///
/// Dipakai untuk dua hal:
/// 1. **Lifecycle** — saat worker di-`abort()`, `Drop` di sini ikut
///    membatalkan semua task lepas (`RefreshQuota`, `spawn_blocking` kuota,
///    subscriber). Tanpa ini task tersebut hidup terus setelah bridge hilang
///    dan mencoba mengirim ke channel yang sudah tertutup.
/// 2. **Housekeeping** — `push()` membuang handle yang sudah selesai lebih
///    dulu, supaya `Vec` tidak tumbuh tanpa batas bila user memicu banyak
///    refresh.
///
/// Housekeeping sengaja ditempel di `push`, bukan diserahkan ke pemanggil:
/// sebelumnya `reap()` harus dipanggil manual dari loop utama dan ternyata
/// terlupakan, sehingga `Vec` bocor pelan-pelan. Karena `push` adalah
/// satu-satunya pintu masuk elemen baru, membersihkan di situ membuat
/// kebocoran jenis itu mustahil terulang.
///
/// `abort()` hanya menghentikan task; ia tidak menunggu, jadi aman dipanggil
/// dari `Drop` yang sinkron (tidak ada `.await` di dalam Drop).
#[derive(Default)]
pub struct TaskGroup {
    handles: Vec<tokio::task::JoinHandle<()>>,
}

impl TaskGroup {
    pub fn new() -> Self {
        Self::default()
    }

    /// Ambil alih kepemilikan sebuah `JoinHandle`.
    ///
    /// Handle yang sudah selesai dibuang lebih dulu, jadi panjang `Vec`
    /// mencerminkan jumlah task yang benar-benar masih hidup.
    pub fn push(&mut self, handle: tokio::task::JoinHandle<()>) {
        self.handles.retain(|h| !h.is_finished());
        self.handles.push(handle);
    }

    /// Benar bila tidak ada task yang masih hidup.
    pub fn is_empty(&mut self) -> bool {
        self.handles.retain(|h| !h.is_finished());
        self.handles.is_empty()
    }
}

/// Kumpulan task turn per sesi (`ExecuteTurn` / `RetryTurn`).
///
/// Bertanggung jawab atas:
/// 1. Memastikan hanya ada 1 task turn per `session_id` (task lama otomatis di-abort).
/// 2. `cancel(session_id)` membatalkan task spesifik dan menghapusnya dari koleksi.
/// 3. `Drop` membatalkan semua task yang masih hidup saat worker selesai/dibatalkan,
///    sehingga tidak ada turn yatim yang berjalan di background.
#[derive(Default)]
pub struct ActiveTurns {
    turns: std::collections::HashMap<String, tokio::task::JoinHandle<()>>,
}

impl ActiveTurns {
    pub fn new() -> Self {
        Self::default()
    }

    /// Bersihkan handle yang sudah selesai berjalan.
    pub fn retain_active(&mut self) {
        self.turns.retain(|_, handle| !handle.is_finished());
    }

    /// Batalkan task turn pada sesi tertentu jika ada.
    pub fn cancel(&mut self, session_id: &str) -> bool {
        if let Some(handle) = self.turns.remove(session_id) {
            handle.abort();
            true
        } else {
            false
        }
    }

    /// Daftarkan task turn baru untuk sesi. Jika sesi sebelumnya masih punya task,
    /// task lama langsung di-abort terlebih dahulu.
    pub fn insert(&mut self, session_id: String, handle: tokio::task::JoinHandle<()>) {
        if let Some(old) = self.turns.insert(session_id, handle) {
            old.abort();
        }
    }

    /// Cek apakah turn untuk sesi ini sedang aktif berjalan.
    #[cfg(test)]
    pub fn is_active(&mut self, session_id: &str) -> bool {
        self.retain_active();
        self.turns.contains_key(session_id)
    }

    /// Benar bila tidak ada task turn yang aktif.
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
/// Sesi aktif dibaca dari `watch::Receiver` (satu sumber kebenaran, lihat
/// `spawn_core_bridge_worker`). Tidak ada polling: perubahan sesi memicu
/// `watch_rx.changed()`, sehingga subscriber hanya bangun saat ada event
/// stream atau saat sesi aktif benar-benar berubah.
///
/// **Lifecycle:** mengembalikan `JoinHandle` yang disimpan di `TaskGroup` milik
/// `spawn_core_bridge_worker`, sehingga ikut di-`abort()` saat bridge di-drop.
/// Subscriber juga berhenti sendiri bila `sub_tx` atau `watch_tx` sudah drop.
///
/// `active_status` adalah cache status sesi aktif di sisi worker (di-*share* lewat
/// `Arc<RwLock<String>>`). Cache ini dipakai sebagai *resync*: core hanya
/// mem-broadcast `StatusChange` saat transisi, jadi kalau subscriber terlambat
/// berlangganan atau event transisi `running` hilang, TUI tidak akan pernah tahu
/// sesi sedang berjalan. Setiap `MessageCreated`/`StepInfo` adalah bukti sesi
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
                        // Sender di-drop: worker sudah berhenti, tidak ada lagi
                        // yang bisa mengubah sesi aktif.
                        break;
                    }
                    let cur = watch_rx.borrow_and_update().clone();
                    if cur != active_sid {
                        active_sid = cur;
                        broadcast_rx = get_session_sender(&active_sid).subscribe();
                        // Sesi baru: sinkronkan status aktual apakah background task sedang jalan,
                        // dan emit transisi status ke UI agar sinkron seketika.
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
                            // Transisi status eksplisit dari core adalah sumber
                            // kebenaran: perbarui cache sebelum event dilewatkan,
                            // supaya `resync_status` tidak menimpanya balik.
                            if let AgentSessionEvent::StatusChange { status } = &event
                                && let Ok(mut st) = active_status.write()
                            {
                                st.clone_from(status);
                            } else if matches!(
                                event,
                                AgentSessionEvent::MessageCreated(_) | AgentSessionEvent::StepInfo { .. }
                            ) {
                                // Bukti aktivitas tanpa transisi `running` yang
                                // terlihat (lag / terlambat subscribe) → pulihkan.
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
                            // Broadcaster ditutup (mis. sesi selesai atau dibersihkan).
                            // Buat receiver baru langsung tanpa sleep/polling timer.
                            broadcast_rx = get_session_sender(&active_sid).subscribe();
                        }
                    }
                }
            }
        }
    })
}

/// Resync status sesi aktif ke `running` bila cache masih `idle`.
///
/// Core hanya mem-broadcast `StatusChange` saat transisi; kalau event transisi
/// `running` hilang (subscriber lag / terlambat subscribe), TUI akan stuck di
/// `idle` walau turn sedang berjalan. Bukti aktivitas (`MessageCreated`,
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

/// Menyetel status sesi aktif secara eksplisit dari sisi worker (bukan dari bukti
/// aktivitas) dan meng-emit `StatusChange` hanya bila nilainya berubah.
///
/// Ini pasangan dari [`resync_status`]: resync memulihkan status dari event stream,
/// sedangkan fungsi ini menulis kebenaran yang diketahui worker sendiri
/// (turn dimulai → `running`, turn dibatalkan → `idle`). Keduanya berbagi cache yang
/// sama supaya transisi tidak dobel-emit.
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

async fn execute_turn_pipeline(
    session_id: String,
    model_override: Option<String>,
    prompt_for_usage: String,
    pool: SqlitePool,
    worker_tx: mpsc::UnboundedSender<CoreBridgeEvent>,
    is_retry: bool,
) {
    if is_retry {
        tracing::info!("Retrying turn for session: {session_id}");
    } else {
        tracing::info!("Executing session turn for session: {session_id}");
    }

    let turn_start = std::time::Instant::now();
    let res = ragasukma_core::agent::executor::execute_session_turn(
        &session_id,
        model_override.as_deref(),
    )
    .await;

    let duration_ms = turn_start.elapsed().as_millis() as u64;
    let chosen_model = model_override.as_deref().unwrap_or("antigravity");

    match res {
        Ok(reply) => {
            record_turn_usage(
                &pool,
                chosen_model,
                &prompt_for_usage,
                &reply,
                duration_ms,
                "success",
            )
            .await;
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
            record_turn_usage(
                &pool,
                chosen_model,
                &prompt_for_usage,
                "",
                duration_ms,
                "error",
            )
            .await;
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
        // Cache status sesi aktif — di-*share* antara worker (penulis status
        // eksplisit) dan subscriber (resync berbasis bukti aktivitas).
        let active_status = std::sync::Arc::new(std::sync::RwLock::new("idle".to_string()));

        // Spawn subscriber loop (sesi aktif dipantau lewat watch channel).
        // Handle disimpan di `tasks` agar subscriber ikut berhenti saat worker
        // dibatalkan — tanpa ini ia hidup sebagai task yatim.
        let mut tasks = TaskGroup::new();
        tasks.push(spawn_session_subscriber(
            worker_tx.clone(),
            watch_tx.subscribe(),
            active_status.clone(),
        ));

        let mut active_turns = ActiveTurns::new();
        let mut last_turn_request: Option<(String, Option<String>)> = None;

        while let Some(cmd) = cmd_rx.recv().await {
            // Bersihkan handle turn yang sudah selesai
            active_turns.retain_active();

            match cmd {
                CoreBridgeCommand::CancelTurn { session_id } => {
                    // Tanpa session_id eksplisit —> batalkan turn pada sesi aktif.
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
                    if let Some((retry_prompt, model_override)) = last_turn_request.clone() {
                        let sid_exec = active_sid.clone();
                        let exec_tx = worker_tx.clone();
                        let pool = pool.clone();
                        let sid_key = sid_exec.clone();

                        let handle = tokio::spawn(async move {
                            execute_turn_pipeline(
                                sid_exec,
                                model_override,
                                retry_prompt,
                                pool,
                                exec_tx,
                                true,
                            )
                            .await;
                        });
                        active_turns.insert(sid_key, handle);
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
                    // Transisi ke `running` diumumkan worker, bukan subscriber.
                    //
                    // Subscriber hanya me-resync status saat melihat bukti aktivitas
                    // (`MessageCreated`/`StepInfo`), sedangkan `user` message ditulis
                    // langsung ke DB (tanpa broadcast) dan `StepInfo("thinking")` baru
                    // muncul setelah pipa turn siap. Akibatnya TUI sempat menampilkan
                    // status lama selama detik-detik pertama turn, dan `CancelTurn`
                    // terlihat no-op. Menyetel cache + emit di sini membuat status
                    // benar sejak perintah diterima.
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

                    let pool = pool.clone();
                    let prompt_for_usage = prompt.clone();
                    let sid_key = sid_exec.clone();
                    let handle = tokio::spawn(async move {
                        execute_turn_pipeline(
                            sid_exec,
                            model_override,
                            prompt_for_usage,
                            pool,
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

    /// `push` harus membuang handle yang sudah selesai, supaya `Vec` tidak
    /// tumbuh tanpa batas saat user memicu refresh berkali-kali.
    #[tokio::test]
    async fn push_membuang_handle_yang_sudah_selesai() {
        let mut group = TaskGroup::new();

        for _ in 0..50 {
            group.push(tokio::spawn(async {}));
        }
        // Beri kesempatan task selesai, lalu picu housekeeping via `push`.
        tokio::time::sleep(Duration::from_millis(20)).await;
        group.push(tokio::spawn(async {}));
        tokio::time::sleep(Duration::from_millis(20)).await;

        assert!(
            group.is_empty(),
            "handle selesai harus dibuang oleh push(), bukan menumpuk"
        );
    }

    /// Task yang masih hidup tidak boleh dibuang oleh housekeeping.
    #[tokio::test]
    async fn push_mempertahankan_task_yang_masih_hidup() {
        let (release, wait) = tokio::sync::oneshot::channel::<()>();
        let mut group = TaskGroup::new();
        group.push(tokio::spawn(async move {
            let _ = wait.await;
        }));
        group.push(tokio::spawn(async {}));
        tokio::time::sleep(Duration::from_millis(20)).await;

        // Hanya task pertama (menunggu `release`) yang tersisa.
        assert!(
            !group.is_empty(),
            "task yang belum selesai tidak boleh ikut dibuang"
        );
        let _ = release.send(());
    }

    #[tokio::test]
    async fn switch_session_2x_berturut_turut_tidak_ada_event_nyasar() {
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

        // Beri waktu subscriber start dan subscribe ke sid1
        tokio::time::sleep(Duration::from_millis(20)).await;

        let sender1 = get_session_sender(&sid1);
        let sender2 = get_session_sender(&sid2);
        let sender3 = get_session_sender(&sid3);

        // 1. Emit di sid1, harus diterima
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
        assert!(got_hello1, "harus menerima delta dari sid1");

        // 2. Switch ke sid2 (Pindah ke-1)
        watch_tx.send(sid2.clone()).unwrap();
        tokio::time::sleep(Duration::from_millis(20)).await;

        // Kirim event di sid1 (sesi lama) dan sid2 (sesi baru)
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
        assert!(!got_stray1, "tidak boleh menerima event nyasar dari sid1");
        assert!(got_hello2, "harus menerima delta dari sid2");

        // 3. Switch ke sid3 (Pindah ke-2 berturut-turut)
        watch_tx.send(sid3.clone()).unwrap();
        tokio::time::sleep(Duration::from_millis(20)).await;

        // Kirim event di sid2 (sesi lama) dan sid3 (sesi baru)
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
        assert!(!got_stray2, "tidak boleh menerima event nyasar dari sid2");
        assert!(got_hello3, "harus menerima delta dari sid3");

        handle.abort();
    }

    #[tokio::test]
    async fn drop_task_group_dan_active_turns_membatalkan_seluruh_task() {
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

            // Pada titik ini kedua task sedang berjalan.
            // Saat keluar blok, tg dan at di-drop.
        }

        // Beri sedikit jeda agar sinyal abort diproses
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Kedua flag tidak boleh pernah diset true karena task di-abort saat drop
        assert!(
            !flag_tg.load(Ordering::SeqCst),
            "TaskGroup harus membatalkan task saat di-drop"
        );
        assert!(
            !flag_at.load(Ordering::SeqCst),
            "ActiveTurns harus membatalkan turn saat di-drop"
        );
    }
}