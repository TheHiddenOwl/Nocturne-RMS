use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};
use chrono::{DateTime, Utc};
use protocol::models::Message;
use tokio::sync::mpsc;

pub struct ClientSession {
    pub agent_id: String,
    pub last_seen: RwLock<DateTime<Utc>>,
    pub latency_ms: RwLock<u64>,
    pub tx: mpsc::UnboundedSender<Message>,
}

pub struct SessionManager {
    pub sessions: RwLock<HashMap<String, Arc<ClientSession>>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
        }
    }

    pub fn add_session(&self, agent_id: String, tx: mpsc::UnboundedSender<Message>) {
        let mut sessions = self.sessions.write().unwrap();
        sessions.insert(
            agent_id.clone(),
            Arc::new(ClientSession {
                agent_id,
                last_seen: RwLock::new(Utc::now()),
                latency_ms: RwLock::new(0),
                tx,
            }),
        );
    }

    pub fn remove_session(&self, agent_id: &str) {
        let mut sessions = self.sessions.write().unwrap();
        sessions.remove(agent_id);
    }

    pub fn update_activity(&self, agent_id: &str, latency_ms: Option<u64>) {
        let sessions = self.sessions.read().unwrap();
        if let Some(session) = sessions.get(agent_id) {
            *session.last_seen.write().unwrap() = Utc::now();
            if let Some(lat) = latency_ms {
                *session.latency_ms.write().unwrap() = lat;
            }
        }
    }
}
