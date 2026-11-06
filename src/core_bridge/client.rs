use anyhow::Result;
use ragasukma_core::agent::repository::{list_messages, list_projects, list_sessions};
use sqlx::SqlitePool;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

use super::types::{
    CoreBridgeCommand, CoreBridgeEvent, PersistedMessageDto, ProjectDto, SessionDto,
};
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
        let mut initial_projects = list_projects().await.unwrap_or_default();
        // Prioritize project with the most recent activity (latest updated_at)
        initial_projects.sort_by_key(|a| std::cmp::Reverse(a.updated_at));
        let project = initial_projects.first().cloned();
        let (project_id, project_name) = match &project {
            Some(p) => (p.id.clone(), p.name.clone()),
            None => (String::new(), "No Project".to_string()),
        };

        let mut existing_sessions = if !project_id.is_empty() {
            list_sessions(&project_id).await.unwrap_or_default()
        } else {
            Vec::new()
        };
        // Prioritize session with the most recent activity (latest updated_at)
        existing_sessions.sort_by_key(|a| std::cmp::Reverse(a.updated_at));
        let session = existing_sessions.first().cloned();
        let session_id = session.as_ref().map(|s| s.id.clone()).unwrap_or_default();

        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<CoreBridgeCommand>();

        let sid = session_id.clone();
        let pid = project_id.clone();

        let _ = event_tx.send(CoreBridgeEvent::Connected {
            project_id: pid.clone(),
            session_id: sid.clone(),
        });

        // Emit initial project list
        let project_dtos = initial_projects
            .into_iter()
            .map(|p| ProjectDto {
                id: p.id,
                name: p.name,
                description: Some(p.description),
                root_path: p.root_path,
            })
            .collect::<Vec<_>>();

        let _ = event_tx.send(CoreBridgeEvent::ProjectsLoaded {
            projects: project_dtos,
            active_id: pid.clone(),
            active_name: project_name,
        });

        // Emit initial session list and history
        let sessions_dto = existing_sessions
            .into_iter()
            .map(SessionDto::from)
            .collect::<Vec<_>>();

        let _ = event_tx.send(CoreBridgeEvent::SessionsLoaded {
            sessions: sessions_dto,
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
            sid.clone(),
            db_pool,
        );

        Ok(Self {
            cmd_tx,
            project_id: pid,
            session_id: sid,
        })
    }

    pub fn submit_prompt(&self, prompt: &str, model_override: Option<String>) -> Result<()> {
        self.cmd_tx
            .send(CoreBridgeCommand::ExecuteTurn {
                prompt: prompt.to_string(),
                model_override,
            })
            .map_err(|e| anyhow::anyhow!("Failed to send prompt to core bridge: {}", e))
    }

    pub fn switch_session(&self, session_id: &str) -> Result<()> {
        self.cmd_tx
            .send(CoreBridgeCommand::SwitchSession {
                session_id: session_id.to_string(),
            })
            .map_err(|e| anyhow::anyhow!("Failed to switch session: {}", e))
    }

    pub fn create_project(
        &self,
        name: &str,
        description: Option<String>,
        root_path: Option<String>,
    ) -> Result<()> {
        self.cmd_tx
            .send(CoreBridgeCommand::CreateProject {
                name: name.to_string(),
                description,
                root_path,
            })
            .map_err(|e| anyhow::anyhow!("Failed to create new project: {}", e))
    }

    pub fn update_project_folders(&self, project_id: &str, folders: Vec<String>) -> Result<()> {
        self.cmd_tx
            .send(CoreBridgeCommand::UpdateProjectFolders {
                project_id: project_id.to_string(),
                folders,
            })
            .map_err(|e| anyhow::anyhow!("Failed to update project folders: {}", e))
    }

    pub fn switch_project(&self, project_id: &str) -> Result<()> {
        self.cmd_tx
            .send(CoreBridgeCommand::SwitchProject {
                project_id: project_id.to_string(),
            })
            .map_err(|e| anyhow::anyhow!("Failed to switch project: {}", e))
    }

    pub fn delete_project(&self, project_id: &str) -> Result<()> {
        self.cmd_tx
            .send(CoreBridgeCommand::DeleteProject {
                project_id: project_id.to_string(),
            })
            .map_err(|e| anyhow::anyhow!("Failed to delete project: {}", e))
    }

    pub fn create_session(&self, title: &str, model: Option<String>) -> Result<()> {
        self.cmd_tx
            .send(CoreBridgeCommand::CreateSession {
                title: title.to_string(),
                model,
            })
            .map_err(|e| anyhow::anyhow!("Failed to create new session: {}", e))
    }

    pub fn update_session_model(&self, session_id: &str, model: &str) -> Result<()> {
        self.cmd_tx
            .send(CoreBridgeCommand::UpdateSessionModel {
                session_id: session_id.to_string(),
                model: model.to_string(),
            })
            .map_err(|e| anyhow::anyhow!("Failed to update session model: {}", e))
    }

    pub fn delete_session(&self, session_id: &str) -> Result<()> {
        self.cmd_tx
            .send(CoreBridgeCommand::DeleteSession {
                session_id: session_id.to_string(),
            })
            .map_err(|e| anyhow::anyhow!("Failed to delete session: {}", e))
    }

    pub fn refresh_quota(&self, provider: &str) -> Result<()> {
        self.cmd_tx
            .send(CoreBridgeCommand::RefreshQuota {
                provider: provider.to_string(),
            })
            .map_err(|e| anyhow::anyhow!("Failed to send refresh quota: {}", e))
    }
}
