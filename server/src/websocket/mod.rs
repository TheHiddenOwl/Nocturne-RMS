use std::{
    collections::HashMap,
    sync::{Arc, RwLock, PoisonError},
};
use chrono::{DateTime, Utc};
use protocol::models::{Message, CommandRequest, CommandResponse};
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SessionError {
    #[error("Session not found: {0}")]
    NotFound(String),
    #[error("Lock poisoned")]
    Poisoned,
    #[error("Failed to send message to agent")]
    SendError,
    #[error("Command timed out")]
    Timeout,
}

impl<T> From<PoisonError<T>> for SessionError {
    fn from(_: PoisonError<T>) -> Self {
        Self::Poisoned
    }
}

pub struct ClientSession {
    pub agent_id: String,
    pub last_seen: RwLock<DateTime<Utc>>,
    pub latency_ms: RwLock<u64>,
    pub tx: mpsc::UnboundedSender<Message>,
}

pub struct SessionManager {
    pub sessions: RwLock<HashMap<String, Arc<ClientSession>>>,
    pub pending_commands: RwLock<HashMap<Uuid, oneshot::Sender<CommandResponse>>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            pending_commands: RwLock::new(HashMap::new()),
        }
    }

    pub fn add_session(&self, agent_id: String, tx: mpsc::UnboundedSender<Message>) -> Result<(), SessionError> {
        let mut sessions = self.sessions.write()?;
        // Evict existing session with the same ID if it exists
        if sessions.contains_key(&agent_id) {
            log::warn!("Agent {} reconnected, evicting old session", agent_id);
            sessions.remove(&agent_id);
        }
        sessions.insert(
            agent_id.clone(),
            Arc::new(ClientSession {
                agent_id,
                last_seen: RwLock::new(Utc::now()),
                latency_ms: RwLock::new(0),
                tx,
            }),
        );
        Ok(())
    }

    pub fn remove_session(&self, agent_id: &str) -> Result<(), SessionError> {
        let mut sessions = self.sessions.write()?;
        sessions.remove(agent_id);
        Ok(())
    }

    pub fn update_activity(&self, agent_id: &str, latency_ms: Option<u64>) -> Result<(), SessionError> {
        let sessions = self.sessions.read()?;
        if let Some(session) = sessions.get(agent_id) {
            *session.last_seen.write()? = Utc::now();
            if let Some(lat) = latency_ms {
                *session.latency_ms.write()? = lat;
            }
            Ok(())
        } else {
            Err(SessionError::NotFound(agent_id.to_string()))
        }
    }

    pub async fn send_command(
        &self,
        agent_id: &str,
        req: CommandRequest,
    ) -> Result<CommandResponse, SessionError> {
        let (tx_resp, rx_resp) = oneshot::channel();
        let request_id = req.request_id;

        // Register pending command
        {
            let mut pending = self.pending_commands.write()?;
            pending.insert(request_id, tx_resp);
        }

        // Send command to agent
        let res = {
            let sessions = self.sessions.read()?;
            if let Some(session) = sessions.get(agent_id) {
                session.tx.send(Message::Command(req)).map_err(|_| SessionError::SendError)
            } else {
                Err(SessionError::NotFound(agent_id.to_string()))
            }
        };

        if let Err(e) = res {
            let mut pending = self.pending_commands.write()?;
            pending.remove(&request_id);
            return Err(e);
        }

        // Wait for response
        match tokio::time::timeout(std::time::Duration::from_secs(30), rx_resp).await {
            Ok(Ok(resp)) => Ok(resp),
            Ok(Err(_)) => {
                // Sender dropped, probably session closed
                Err(SessionError::SendError)
            }
            Err(_) => {
                // Timeout
                let mut pending = self.pending_commands.write()?;
                pending.remove(&request_id);
                Err(SessionError::Timeout)
            }
        }
    }

    pub fn handle_response(&self, resp: CommandResponse) -> Result<(), SessionError> {
        let mut pending = self.pending_commands.write()?;
        if let Some(tx) = pending.remove(&resp.request_id) {
            let _ = tx.send(resp);
        }
        Ok(())
    }
}
