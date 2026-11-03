use anyhow::Result;
use ragasukma_core::agent::repository::{
    create_project, create_session, list_messages, list_projects, list_sessions,
};
use sqlx::SqlitePool;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

use super::types::{CoreBridgeCommand, CoreBridgeEvent, PersistedMessageDto, ProjectDto};
use super::worker::spawn_core_bridge_worker;

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

        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<CoreBridgeCommand>();

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
        let worker_project_id = pid.clone();
        let worker_tx = event_tx.clone();
        let current_sid_clone = Arc::clone(&current_sid);

        spawn_core_bridge_worker(
            cmd_rx,
            worker_tx,
            current_sid_clone,
            worker_project_id,
            sid,
            db_pool,
        );

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
