// --------------------------------------------------
// Handles API endpoints related to task CRUD operations
// and global settings management.
//
// Responsibilities:
// - Create / read / update / delete tasks
// - Toggle task status (Todo <-> Done)
// - Get / update day settings
// -------------------------------------------------

use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, FixedOffset, NaiveDate};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::{Db, DaySettings, Task, TaskStatus};
use crate::store;

fn now_fixed_offset() -> DateTime<FixedOffset> {
    let local = chrono::Local::now();
    let offset_seconds = local.offset().local_minus_utc();
    let fixed = FixedOffset::east_opt(offset_seconds).unwrap();
    local.with_timezone(&fixed)
}

#[derive(Debug, Deserialize)]
pub struct TasksQuery {
    pub date: String, // "YYYY-MM-DD"
}

#[derive(Debug, Serialize)]
pub struct TasksResponse {
    pub date: String,
    pub now: String,
    pub tasks: Vec<Task>,
}

// -----------------------------
// GET /api/tasks
// Returns all tasks stored in db.json
// -----------------------------
pub async fn get_tasks(Query(q): Query<TasksQuery>) -> impl IntoResponse {
    let date = match NaiveDate::parse_from_str(&q.date, "%Y-%m-%d") {
        Ok(d) => d,
        Err(_) => return (StatusCode::BAD_REQUEST, "invalid date").into_response(),
    };
    let now = now_fixed_offset();

    let db: Db = match store::load_db() {
        Ok(db) => db,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "failed to load db").into_response(),
    };

    let tasks: Vec<Task> = db
        .tasks
        .into_iter()
        .filter(|t| t.status != TaskStatus::Done)
        .filter(|t| {
            let overdue = now > t.due_at;
            let due_today = t.due_at.date_naive() == date;
            overdue || due_today
        })
        .collect();

    Json(TasksResponse {
        date: q.date,
        now: now.to_rfc3339(),
        tasks,
    })
    .into_response()
}

#[derive(Debug, Deserialize)]
pub struct CreateTaskInput {
    pub title: String,
    pub due_at: String, // RFC3339
    pub duration_min: i64,
    pub priority: i64, // 1..=5
    pub tags: Option<Vec<String>>,
    pub notes: Option<String>,
}

// -----------------------------
// POST /api/tasks
// Creates a new task and saves it to db.json
// -----------------------------
pub async fn create_task(Json(input): Json<CreateTaskInput>) -> impl IntoResponse {
    if input.title.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, "title required").into_response();
    }
    if !(1..=5).contains(&input.priority) {
        return (StatusCode::BAD_REQUEST, "priority must be 1..=5").into_response();
    }

    let due_at = match DateTime::parse_from_rfc3339(&input.due_at) {
        Ok(dt) => dt,
        Err(_) => return (StatusCode::BAD_REQUEST, "invalid due_at").into_response(),
    };

    let now = now_fixed_offset();

    let mut db: Db = match store::load_db() {
        Ok(db) => db,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "failed to load db").into_response(),
    };

    let task = Task {
        id: Uuid::new_v4(),
        title: input.title,
        due_at,
        duration_min: input.duration_min,
        priority: input.priority,
        status: TaskStatus::Todo,
        created_at: now,
        tags: input.tags,
        notes: input.notes,
    };

    db.tasks.push(task.clone());

    if store::save_db(&db).is_err() {
        return (StatusCode::INTERNAL_SERVER_ERROR, "failed to save db").into_response();
    }

    Json(task).into_response()
}

#[derive(Debug, Deserialize)]
pub struct UpdateTaskInput {
    pub title: String,
    pub due_at: String, // RFC3339
    pub duration_min: i64,
    pub priority: i64,
    pub status: TaskStatus,
    pub tags: Option<Vec<String>>,
    pub notes: Option<String>,
}

// -----------------------------
// PUT /api/tasks/:id
// Updates an existing task by ID
// ----------------------------
pub async fn update_task(
    Path(id): Path<String>,
    Json(input): Json<UpdateTaskInput>,
) -> impl IntoResponse {
    let id = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, "invalid id").into_response(),
    };

    if input.title.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, "title required").into_response();
    }
    if !(1..=5).contains(&input.priority) {
        return (StatusCode::BAD_REQUEST, "priority must be 1..=5").into_response();
    }

    let due_at = match DateTime::parse_from_rfc3339(&input.due_at) {
        Ok(dt) => dt,
        Err(_) => return (StatusCode::BAD_REQUEST, "invalid due_at").into_response(),
    };

    let mut db: Db = match store::load_db() {
        Ok(db) => db,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "failed to load db").into_response(),
    };

    let Some(t) = db.tasks.iter_mut().find(|t| t.id == id) else {
        return (StatusCode::NOT_FOUND, "task not found").into_response();
    };

    t.title = input.title;
    t.due_at = due_at;
    t.duration_min = input.duration_min;
    t.priority = input.priority;
    t.status = input.status.clone();
    t.tags = input.tags;
    t.notes = input.notes;

    let updated = t.clone();

    if store::save_db(&db).is_err() {
        return (StatusCode::INTERNAL_SERVER_ERROR, "failed to save db").into_response();
    }

    Json(updated).into_response()
}

// -----------------------------
// DELETE /api/tasks/:id
// Removes a task permanently
// -----------------------------
pub async fn delete_task(Path(id): Path<String>) -> impl IntoResponse {
    let id = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, "invalid id").into_response(),
    };

    let mut db: Db = match store::load_db() {
        Ok(db) => db,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "failed to load db").into_response(),
    };

    let before = db.tasks.len();
    db.tasks.retain(|t| t.id != id);

    if db.tasks.len() == before {
        return (StatusCode::NOT_FOUND, "task not found").into_response();
    }

    if store::save_db(&db).is_err() {
        return (StatusCode::INTERNAL_SERVER_ERROR, "failed to save db").into_response();
    }

    Json(serde_json::json!({ "ok": true })).into_response()
}

// -----------------------------
// POST /api/tasks/:id/toggle
// Toggles task status between Todo and Done
// -----------------------------
pub async fn toggle_task(Path(id): Path<String>) -> impl IntoResponse {
    let id = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, "invalid id").into_response(),
    };

    let mut db: Db = match store::load_db() {
        Ok(db) => db,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "failed to load db").into_response(),
    };

    let Some(t) = db.tasks.iter_mut().find(|t| t.id == id) else {
        return (StatusCode::NOT_FOUND, "task not found").into_response();
    };

    t.status = match t.status {
        TaskStatus::Todo => TaskStatus::InProgress,
        TaskStatus::InProgress => TaskStatus::Done,
        TaskStatus::Done => TaskStatus::Todo,
    };

    let updated = t.clone();

    if store::save_db(&db).is_err() {
        return (StatusCode::INTERNAL_SERVER_ERROR, "failed to save db").into_response();
    }

    Json(updated).into_response()
}

// -----------------------------
// GET /api/settings
// Returns day-level settings (start/end/focus block)
// -----------------------------
pub async fn get_settings() -> impl IntoResponse {
    let db: Db = match store::load_db() {
        Ok(db) => db,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "failed to load db").into_response(),
    };
    Json(db.settings).into_response()
}

// -----------------------------
// PUT /api/settings
// Updates day-level settings
// -----------------------------
pub async fn put_settings(Json(s): Json<DaySettings>) -> impl IntoResponse {
    let mut db: Db = match store::load_db() {
        Ok(db) => db,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "failed to load db").into_response(),
    };

    db.settings = s;

    if store::save_db(&db).is_err() {
        return (StatusCode::INTERNAL_SERVER_ERROR, "failed to save db").into_response();
    }

    Json(db.settings).into_response()
}
