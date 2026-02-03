use std::sync::Arc;

use serde_json::Value;

use crate::ports::ingest::IngestPort;

pub struct IngestManager {
    ingest_url: Option<String>,
    ingest_sent: bool,
    ingest_port: Arc<dyn IngestPort>,
}

impl IngestManager {
    pub fn new(ingest_url: Option<String>, ingest_port: Arc<dyn IngestPort>) -> Self {
        Self {
            ingest_url,
            ingest_sent: false,
            ingest_port,
        }
    }

    pub fn should_post(&self) -> bool {
        self.ingest_url.is_some() && !self.ingest_sent
    }

    pub async fn post_once(&mut self, payload: Value) {
        if self.ingest_sent {
            return;
        }
        let Some(url) = self.ingest_url.clone() else {
            return;
        };
        self.ingest_sent = true;
        if let Err(e) = self.ingest_port.post(url, payload).await {
            log::warn!("[ingest] failed to post: {:?}", e);
        }
    }
}
