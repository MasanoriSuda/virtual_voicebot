use std::future::Future;
use std::pin::Pin;

use chrono::{DateTime, Utc};
use thiserror::Error;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct Announcement {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub announcement_type: String,
    pub is_active: bool,
    pub folder_id: Option<Uuid>,
    pub audio_file_url: Option<String>,
    pub tts_text: Option<String>,
    pub duration_sec: Option<i32>,
    pub language: String,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct UpsertAnnouncement {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub announcement_type: String,
    pub is_active: bool,
    pub folder_id: Option<Uuid>,
    pub audio_file_url: Option<String>,
    pub tts_text: Option<String>,
    pub duration_sec: Option<i32>,
    pub language: String,
    pub version: i32,
}

#[derive(Debug, Error)]
pub enum AnnouncementError {
    #[error("read failed: {0}")]
    ReadFailed(String),
    #[error("write failed: {0}")]
    WriteFailed(String),
}

pub type AnnouncementFuture<T> = Pin<Box<dyn Future<Output = Result<T, AnnouncementError>> + Send>>;

pub trait AnnouncementPort: Send + Sync {
    fn list_active(&self) -> AnnouncementFuture<Vec<Announcement>>;
    fn upsert_announcement(&self, announcement: UpsertAnnouncement) -> AnnouncementFuture<()>;
}
