// --------------------------------------------------
// Handles API endpoints related to generating a daily plan.
// This file connects HTTP requests (/api/plan/today)
// to the core scheduling logic implemented in logic.rs.
// --------------------------------------------------

use axum::{
    extract::Query,         // parse query parameter
    http::StatusCode,       // return HTTP status codes
    response::IntoResponse, // allow returning different responses
    Json,                   // JSON response wrapper
};
use chrono::{DateTime, FixedOffset, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::logic; // scheduling logic
use crate::models::{Db, DaySettings};
use crate::store; // JSON database load/save utilities


// Query parameters for /plan/today
#[derive(Debug, Deserialize)]
pub struct PlanQuery {
    pub date: String,         // Target date in "YYYY-MM-DD" format
    pub available_min: i64,   // Total minutes user can work today
}


// Full response returned to frontend
#[derive(Debug, Serialize)]
pub struct PlanResponse {
    pub date: String,                       // requested date
    pub now: String,                        // server time(now)
    pub available_min: i64, 
    pub settings: DaySettings,              // day start/end setting
    pub plan: Vec<PlanItemResponse>,        // scheduled task
    pub unplanned: Vec<UnplannedResponse>,  // tasks that do not fit
}

// A single scheduled task in the final plan
#[derive(Debug, Serialize)]
pub struct PlanItemResponse {
    pub task_id: String,
    pub title: String,
    pub start: String,  // start time
    pub end: String,    // end time
    pub score_breakdown: ScoreBreakdownResponse,
    pub is_overdue: bool,
}

// Score breakdown used for ranking tasks
#[derive(Debug, Serialize)]
pub struct ScoreBreakdownResponse {
    pub urgency: i64,
    pub priority: i64,
    pub duration_score: i64,
    pub total: i64,
}

// Tasks that could not be scheduled
#[derive(Debug, Serialize)]
pub struct UnplannedResponse {
    pub task_id: String,
    pub reason: String,
}

// --------------------------------------------------
// Helper: returns "current time" with a fixed offset.
// For now, CST (-06:00) is hardcoded for simplicity.
// --------------------------------------------------
fn now_fixed_offset() -> DateTime<FixedOffset> {
    let local = chrono::Local::now();
    let offset_seconds = local.offset().local_minus_utc();
    let fixed = FixedOffset::east_opt(offset_seconds).unwrap();
    local.with_timezone(&fixed)
}


// --------------------------------------------------
// GET /api/plan/today
//
// High-level flow:
// 1. Parse and validate query parameters
// 2. Load DB from JSON
// 3. Filter tasks relevant to the given date
// 4. Score and sort tasks by urgency/priority/duration
// 5. Build today's plan within available time
// 6. Return structured JSON for frontend rendering
// --------------------------------------------------
pub async fn get_today_plan(Query(q): Query<PlanQuery>) -> impl IntoResponse {
    // Parse date string into NaiveDate
    let date = match NaiveDate::parse_from_str(&q.date, "%Y-%m-%d") {
        Ok(d) => d,
        Err(_) => return (StatusCode::BAD_REQUEST, "invalid date").into_response(),
    };

    let now = now_fixed_offset();

    // Load database from data/db.json
    let db: Db = match store::load_db() {
        Ok(db) => db,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "failed to load db").into_response(),
    };

    // Step 1: extract tasks relevant to this date
    let relevant = logic::relevant_tasks(&db.tasks, date, now);

    // Step 2: score tasks and sort by total score (descending)
    let scored_sorted = logic::score_and_sort(relevant, now);

    // Step 3: build today's schedule within available minutes
    let (plan, unplanned) =
        logic::build_today_plan(scored_sorted, date, now, &db.settings, q.available_min);

    // Convert internal structs into API response format
    let plan_resp: Vec<PlanItemResponse> = plan
        .into_iter()
        .map(|p| PlanItemResponse {
            task_id: p.task_id,
            title: p.title,
            start: p.start.to_rfc3339(),
            end: p.end.to_rfc3339(),
            score_breakdown: ScoreBreakdownResponse {
                urgency: p.score_breakdown.urgency,
                priority: p.score_breakdown.priority,
                duration_score: p.score_breakdown.duration_score,
                total: p.score_breakdown.total,
            },
            is_overdue: p.is_overdue,
        })
        .collect();

    let unplanned_resp: Vec<UnplannedResponse> = unplanned
        .into_iter()
        .map(|u| UnplannedResponse {
            task_id: u.task_id,
            reason: u.reason,
        })
        .collect();

    Json(PlanResponse {
        date: q.date,
        now: now.to_rfc3339(),
        available_min: q.available_min,
        settings: db.settings,
        plan: plan_resp,
        unplanned: unplanned_resp,
    })
    .into_response()
}
