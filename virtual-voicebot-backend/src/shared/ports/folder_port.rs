use std::future::Future;
use std::pin::Pin;

use chrono::{DateTime, Utc};
use thiserror::Error;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct Folder {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub entity_type: String,
    pub name: String,
    pub description: Option<String>,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct UpsertFolder {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub entity_type: String,
    pub name: String,
    pub description: Option<String>,
    pub sort_order: i32,
}

#[derive(Debug, Error)]
pub enum FolderError {
    #[error("read failed: {0}")]
    ReadFailed(String),
    #[error("write failed: {0}")]
    WriteFailed(String),
}

pub type FolderFuture<T> = Pin<Box<dyn Future<Output = Result<T, FolderError>> + Send>>;

pub trait FolderPort: Send + Sync {
    fn list_by_entity_type(&self, entity_type: String) -> FolderFuture<Vec<Folder>>;
    fn upsert_folder(&self, folder: UpsertFolder) -> FolderFuture<()>;
}
