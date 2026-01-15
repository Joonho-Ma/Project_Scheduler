use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Represents the current state of a task.
//
// This enum is serialized as snake_case strings in JSON:
// - "todo"
// - "in_progress"
// - "done"
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Todo,
    InProgress,
    Done,
}

// Core task entity stored in db.json.
//
// This struct represents a single unit of work
// and is used across the entire application
// (storage, logic, and API responses).
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


// User-configurable settings that affect daily scheduling.
//
// These settings are shared by all tasks
// and determine how the daily plan is constructed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaySettings {
    pub day_start: String, // start of the day with format "HH:MM"
    pub day_end: String,   // end of the day with format "HH:MM"
    pub focus_block_min: i64,   // preferred focus block length in minutes
}

// Top-level structure representing the entire database.
//
// This is what gets serialized/deserialized
// from `data/db.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Db {
    pub settings: DaySettings,
    pub tasks: Vec<Task>,
}
