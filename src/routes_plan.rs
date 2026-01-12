use axum::{
    extract::Query,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, FixedOffset, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::logic;
use crate::models::{Db, DaySettings};
use crate::store;

#[derive(Debug, Deserialize)]
pub struct PlanQuery {
    pub date: String,         // "YYYY-MM-DD"
    pub available_min: i64,
}

#[derive(Debug, Serialize)]
pub struct PlanResponse {
    pub date: String,
    pub now: String,
    pub available_min: i64,
    pub settings: DaySettings,
    pub plan: Vec<PlanItemResponse>,
    pub unplanned: Vec<UnplannedResponse>,
}

#[derive(Debug, Serialize)]
pub struct PlanItemResponse {
    pub task_id: String,
    pub title: String,
    pub start: String,
    pub end: String,
    pub score_breakdown: ScoreBreakdownResponse,
    pub is_overdue: bool,
}

#[derive(Debug, Serialize)]
pub struct ScoreBreakdownResponse {
    pub urgency: i64,
    pub priority: i64,
    pub duration_score: i64,
    pub total: i64,
}

#[derive(Debug, Serialize)]
pub struct UnplannedResponse {
    pub task_id: String,
    pub reason: String,
}

// Local -> FixedOffset (현재 시스템 오프셋 사용)
fn now_fixed_offset() -> DateTime<FixedOffset> {
    let local = chrono::Local::now();
    let offset_seconds = local.offset().local_minus_utc();
    let fixed = FixedOffset::east_opt(offset_seconds).unwrap();
    local.with_timezone(&fixed)
}

pub async fn get_today_plan(Query(q): Query<PlanQuery>) -> impl IntoResponse {
    let date = match NaiveDate::parse_from_str(&q.date, "%Y-%m-%d") {
        Ok(d) => d,
        Err(_) => return (StatusCode::BAD_REQUEST, "invalid date").into_response(),
    };

    let now = now_fixed_offset();

    let db: Db = match store::load_db() {
        Ok(db) => db,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "failed to load db").into_response(),
    };

    let relevant = logic::relevant_tasks(&db.tasks, date, now);
    let scored_sorted = logic::score_and_sort(relevant, now);
    let (plan, unplanned) =
        logic::build_today_plan(scored_sorted, date, now, &db.settings, q.available_min);

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
