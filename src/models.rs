use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Todo,
    InProgress,
    Done,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub title: String,
    pub due_at: DateTime<FixedOffset>,
    pub duration_min: i64,
    pub priority: i64, // 1..=5
    pub status: TaskStatus,
    pub created_at: DateTime<FixedOffset>,
    pub tags: Option<Vec<String>>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaySettings {
    pub day_start: String, // "HH:MM"
    pub day_end: String,   // "HH:MM"
    pub focus_block_min: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Db {
    pub settings: DaySettings,
    pub tasks: Vec<Task>,
}
