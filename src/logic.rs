/*
Scheduling and scoring logic.
Module was independently written from HTTP / Axum for testing
*/


use chrono::{DateTime, Duration, FixedOffset, NaiveDate, TimeZone};
use crate::models::{Task, TaskStatus, DaySettings};


// Internal representation of single task after scoring
//     not exposed through API directly
#[derive(Debug, Clone)]
pub struct ScoredTask {
    pub task: Task,
    pub is_overdue: bool,   // determine whether the task is overdue
    pub urgency: i64,        // 0..5
    pub duration_score: i64, // 1..5
    pub total: i64,          // urgency + priority + duration_score
}

// Scheduled item placed on today's timeline
// Represents time block
#[derive(Debug, Clone)]
pub struct PlanItem {
    pub task_id: String,    // task id string
    pub title: String,      // task title
    pub start: DateTime<FixedOffset>,   // start time of task
    pub end: DateTime<FixedOffset>,     // end time of task
    pub score_breakdown: ScoreBreakdown,    // scoring info
    pub is_overdue: bool,   // whether the task is overdue
}

// Logic of how a task's score is calculated
#[derive(Debug, Clone)]
pub struct ScoreBreakdown {
    pub urgency: i64,
    pub priority: i64,
    pub duration_score: i64,
    pub total: i64,
}

// Task that cannot be scheduled today
#[derive(Debug, Clone)]
pub struct UnplannedItem {
    pub task_id: String,
    pub reason: String, // "insufficient_time" / "invalid_duration"
}

// Select tasks that are relevant for today's plan.
//
// Rules:
// - Task status must not be Done
// - Task must be either overdue OR due today
pub fn relevant_tasks(tasks: &[Task], date: NaiveDate, now: DateTime<FixedOffset>) -> Vec<Task> {
    tasks
        .iter()
        .filter(|t| t.status != TaskStatus::Done)
        .filter(|t| {
            let overdue = now > t.due_at;
            let due_today = t.due_at.date_naive() == date;
            overdue || due_today
        })
        .cloned()
        .collect()
}

// urgency (0..5):
// overdue -> 5
// 0-1 day:5, 1-2:4, 2-3:3, 3-4:2, 4-5:1, >=5:0
pub fn urgency_score(due_at: DateTime<FixedOffset>, now: DateTime<FixedOffset>) -> i64 {
    if now > due_at {
        return 5;
    }
    let diff = due_at - now;
    let secs = diff.num_seconds();
    if secs <= 0 {
        return 5;
    }
    let day = 24 * 60 * 60;
    if secs < 1 * day { 5 }
    else if secs < 2 * day { 4 }
    else if secs < 3 * day { 3 }
    else if secs < 4 * day { 2 }
    else if secs < 5 * day { 1 }
    else { 0 }
}

// Compute duration score based on estimated task length.
//
// Shorter tasks are prioritized:
//     0–60 min   -> 5
//     61–120 min -> 4
//     121–180 min-> 3
//     181–240 min-> 2
//     >240 min   -> 1
pub fn duration_score(duration_min: i64) -> i64 {
    let bucket = if duration_min <= 60 { 1 }
    else if duration_min <= 120 { 2 }
    else if duration_min <= 180 { 3 }
    else if duration_min <= 240 { 4 }
    else { 5 };

    // Invert bucket so that shorter durations get higher scores
    6 - bucket
}

// Score all tasks and sort them by priority.
//
// Sorting rules:
// 1) Higher total score first
// 2) If tied, alphabetical order by title
pub fn score_and_sort(tasks: Vec<Task>, now: DateTime<FixedOffset>) -> Vec<ScoredTask> {
    let mut scored: Vec<ScoredTask> = tasks
        .into_iter()
        .map(|t| {
            let is_overdue = now > t.due_at;
            let u = urgency_score(t.due_at, now);
            let d = duration_score(t.duration_min);
            let p = t.priority;
            let total = u + p + d;

            ScoredTask {
                task: t,
                is_overdue,
                urgency: u,
                duration_score: d,
                total,
            }
        })
        .collect();

    // sort: total desc, tie -> title alphabetical asc
    scored.sort_by(|a, b| {
        b.total
            .cmp(&a.total)
            .then_with(|| a.task.title.to_lowercase().cmp(&b.task.title.to_lowercase()))
    });

    scored
}


// Parse a "HH:MM" string into a DateTime on the given date.
fn parse_hhmm_to_today(
    date: NaiveDate,
    hhmm: &str,
    offset: FixedOffset,
) -> Option<DateTime<FixedOffset>> {
    let parts: Vec<&str> = hhmm.split(':').collect();
    if parts.len() != 2 {
        return None;
    }
    let h: u32 = parts[0].parse().ok()?;
    let m: u32 = parts[1].parse().ok()?;
    let naive = date.and_hms_opt(h, m, 0)?;
    offset.from_local_datetime(&naive).single()
}



/// Build today's schedule by placing tasks on a timeline.
///
/// Process:
/// - Start at max(now, day_start)
/// - Respect day_end and available minutes
/// - Place tasks sequentially in sorted order
/// - Tasks that do not fit are marked as unplanned
pub fn build_today_plan(
    scored_sorted: Vec<ScoredTask>,
    date: NaiveDate,
    now: DateTime<FixedOffset>,
    settings: &DaySettings,
    available_min: i64,
) -> (Vec<PlanItem>, Vec<UnplannedItem>) {
    let offset = *now.offset();

    let day_start_dt =
        parse_hhmm_to_today(date, &settings.day_start, offset).unwrap_or(now);
    let day_end_dt =
        parse_hhmm_to_today(date, &settings.day_end, offset).unwrap_or(now + Duration::hours(8));

    // start = max(now, day_start)
    let mut cursor = if now > day_start_dt { now } else { day_start_dt };

    let mut remaining = available_min;
    let mut plan: Vec<PlanItem> = Vec::new();
    let mut unplanned: Vec<UnplannedItem> = Vec::new();

    for st in scored_sorted {
        if remaining <= 0 {
            unplanned.push(UnplannedItem {
                task_id: st.task.id.to_string(),
                reason: "insufficient_time".to_string(),
            });
            continue;
        }

        let dur = st.task.duration_min;
        if dur <= 0 {
            unplanned.push(UnplannedItem {
                task_id: st.task.id.to_string(),
                reason: "invalid_duration".to_string(),
            });
            continue;
        }

        let end = cursor + Duration::minutes(dur);

        if end > day_end_dt || dur > remaining {
            unplanned.push(UnplannedItem {
                task_id: st.task.id.to_string(),
                reason: "insufficient_time".to_string(),
            });
            continue;
        }

        let breakdown = ScoreBreakdown {
            urgency: st.urgency,
            priority: st.task.priority,
            duration_score: st.duration_score,
            total: st.total,
        };

        plan.push(PlanItem {
            task_id: st.task.id.to_string(),
            title: st.task.title.clone(),
            start: cursor,
            end,
            score_breakdown: breakdown,
            is_overdue: st.is_overdue,
        });

        cursor = end;
        remaining -= dur;
    }

    (plan, unplanned)
}
