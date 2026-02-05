use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::sync::Mutex;

/// ストリーム状態を保持するマネージャ（Call-ID 等をキーに Seq/Ts/SSRC を管理）
#[derive(Clone, Default)]
pub struct StreamManager {
    inner: Arc<Mutex<HashMap<String, StreamEntry>>>,
}

#[derive(Debug, Clone)]
pub struct StreamEntry {
    pub dst: SocketAddr,
    pub pt: u8,
    pub ssrc: u32,
    pub seq: u16,
    pub ts: u32,
    pub packet_count: u32,
    pub octet_count: u32,
    pub last_rtp_ts: u32,
}

impl StreamManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn upsert(
        &self,
        key: String,
        dst: SocketAddr,
        pt: u8,
        ssrc: u32,
        seq: u16,
        ts: u32,
    ) {
        let mut map = self.inner.lock().await;
        if let Some(entry) = map.get_mut(&key) {
            entry.dst = dst;
            entry.pt = pt;
            entry.ssrc = ssrc;
            entry.seq = seq;
            entry.ts = ts;
            entry.last_rtp_ts = ts;
            return;
        }
        map.insert(
            key,
            StreamEntry {
                dst,
                pt,
                ssrc,
                seq,
                ts,
                packet_count: 0,
                octet_count: 0,
                last_rtp_ts: ts,
            },
        );
    }

    pub async fn remove(&self, key: &str) {
        let mut map = self.inner.lock().await;
        map.remove(key);
    }

    pub async fn with_mut<F, R>(&self, key: &str, f: F) -> Option<R>
    where
        F: FnOnce(&mut StreamEntry) -> R,
    {
        let mut map = self.inner.lock().await;
        map.get_mut(key).map(f)
    }

    pub async fn is_empty(&self) -> bool {
        self.inner.lock().await.is_empty()
    }

    pub async fn list(&self) -> Vec<(String, StreamEntry)> {
        let map = self.inner.lock().await;
        map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }
}
