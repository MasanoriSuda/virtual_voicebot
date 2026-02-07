use std::future::Future;
use std::pin::Pin;

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use thiserror::Error;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct ScheduleTimeSlot {
    pub id: Uuid,
    pub schedule_id: Uuid,
    pub day_of_week: Option<i16>,
    pub start_time: NaiveTime,
    pub end_time: NaiveTime,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct Schedule {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub schedule_type: String,
    pub is_active: bool,
    pub folder_id: Option<Uuid>,
    pub date_range_start: Option<NaiveDate>,
    pub date_range_end: Option<NaiveDate>,
    pub action_type: String,
    pub action_target: Option<Uuid>,
    pub action_code: Option<String>,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub time_slots: Vec<ScheduleTimeSlot>,
}

#[derive(Clone, Debug)]
pub struct UpsertScheduleTimeSlot {
    pub id: Uuid,
    pub day_of_week: Option<i16>,
    pub start_time: NaiveTime,
    pub end_time: NaiveTime,
}

#[derive(Clone, Debug)]
pub struct UpsertSchedule {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub schedule_type: String,
    pub is_active: bool,
    pub folder_id: Option<Uuid>,
    pub date_range_start: Option<NaiveDate>,
    pub date_range_end: Option<NaiveDate>,
    pub action_type: String,
    pub action_target: Option<Uuid>,
    pub action_code: Option<String>,
    pub version: i32,
    pub time_slots: Vec<UpsertScheduleTimeSlot>,
}

#[derive(Debug, Error)]
pub enum ScheduleError {
    #[error("read failed: {0}")]
    ReadFailed(String),
    #[error("write failed: {0}")]
    WriteFailed(String),
}

pub type ScheduleFuture<T> = Pin<Box<dyn Future<Output = Result<T, ScheduleError>> + Send>>;

pub trait SchedulePort: Send + Sync {
    fn list_active(&self) -> ScheduleFuture<Vec<Schedule>>;
    fn upsert_schedule(&self, schedule: UpsertSchedule) -> ScheduleFuture<()>;
}
