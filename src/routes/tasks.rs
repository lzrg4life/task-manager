use std::collections::HashMap;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::{Deserialize, Deserializer, Serialize};
use sqlx::{FromRow, SqlitePool};

use crate::{
    AppState,
    auth::AuthUser,
    error::{AppError, AppResult},
    models::TagOut,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Archived,
    Completed,
    InReview,
    Active,
    UpNext,
    Backlog,
    Removed,
}

/// Lists shown on the main page, top-down. "Up" moves toward the start.
const BOARD: [Status; 5] = [
    Status::Completed,
    Status::InReview,
    Status::Active,
    Status::UpNext,
    Status::Backlog,
];

impl Status {
    fn as_str(self) -> &'static str {
        match self {
            Status::Archived => "archived",
            Status::Completed => "completed",
            Status::InReview => "in_review",
            Status::Active => "active",
            Status::UpNext => "up_next",
            Status::Backlog => "backlog",
            Status::Removed => "removed",
        }
    }

    fn parse(s: &str) -> AppResult<Self> {
        Ok(match s {
            "archived" => Status::Archived,
            "completed" => Status::Completed,
            "in_review" => Status::InReview,
            "active" => Status::Active,
            "up_next" => Status::UpNext,
            "backlog" => Status::Backlog,
            "removed" => Status::Removed,
            _ => return Err(AppError::Internal(format!("bad status in db: {s}"))),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Normal,
    High,
    Low,
}

impl Priority {
    fn as_str(self) -> &'static str {
        match self {
            Priority::Normal => "normal",
            Priority::High => "high",
            Priority::Low => "low",
        }
    }

    fn parse(s: &str) -> AppResult<Self> {
        Ok(match s {
            "normal" => Priority::Normal,
            "high" => Priority::High,
            "low" => Priority::Low,
            _ => return Err(AppError::Internal(format!("bad priority in db: {s}"))),
        })
    }
}

#[derive(Serialize, Clone)]
pub struct UserRef {
    id: i64,
    name: String,
    email: String,
}

#[derive(Serialize)]
pub struct TaskSummary {
    id: i64,
    key: String,
    title: String,
    expected_by_date: Option<String>,
    status: Status,
    priority: Priority,
    blocked: bool,
    blocked_reason: Option<String>,
    reviewer: Option<UserRef>,
    implementer: Option<UserRef>,
    tags: Vec<TagOut>,
}

#[derive(Serialize)]
pub struct HistoryEntry {
    field: String,
    old_value: Option<String>,
    new_value: Option<String>,
    changed_by: UserRef,
    changed_at: String,
}

#[derive(Serialize)]
pub struct LinkOut {
    id: i64,
    url: String,
    label: String,
    added_by: UserRef,
    created_at: String,
}

#[derive(Serialize)]
pub struct TaskOut {
    #[serde(flatten)]
    summary: TaskSummary,
    details: String,
    business_requirements: String,
    technical_requirements: String,
    implementation_notes: String,
    created_by: UserRef,
    created_at: String,
    updated_at: String,
    links: Vec<LinkOut>,
    related_tasks: Vec<TaskSummary>,
    history: Vec<HistoryEntry>,
}

#[derive(FromRow)]
struct TaskRow {
    id: i64,
    title: String,
    details: String,
    business_requirements: String,
    technical_requirements: String,
    implementation_notes: String,
    status: String,
    priority: String,
    expected_by_date: Option<String>,
    blocked_reason: Option<String>,
    created_at: String,
    updated_at: String,
    created_by: i64,
    cb_email: String,
    cb_name: String,
    reviewer_id: Option<i64>,
    rv_email: Option<String>,
    rv_name: Option<String>,
    implementer_id: Option<i64>,
    im_email: Option<String>,
    im_name: Option<String>,
}

#[derive(FromRow)]
struct HistoryRow {
    field: String,
    old_value: Option<String>,
    new_value: Option<String>,
    changed_by: i64,
    u_email: String,
    u_name: String,
    changed_at: String,
}

#[derive(FromRow)]
struct LinkRow {
    id: i64,
    url: String,
    label: String,
    created_by: i64,
    u_email: String,
    u_name: String,
    created_at: String,
}

#[derive(FromRow)]
struct TaskTagRow {
    task_id: i64,
    id: i64,
    name: String,
    description: String,
    archived: bool,
}

const SELECT_TASK: &str = "SELECT t.id, t.title, t.details, t.business_requirements, \
    t.technical_requirements, t.implementation_notes, t.status, t.priority, t.expected_by_date, \
    t.blocked_reason, t.created_at, \
    t.updated_at, t.created_by, cb.email AS cb_email, cb.display_name AS cb_name, \
    t.reviewer_id, rv.email AS rv_email, rv.display_name AS rv_name, \
    t.implementer_id, im.email AS im_email, im.display_name AS im_name \
    FROM tasks t JOIN users cb ON cb.id = t.created_by \
    LEFT JOIN users rv ON rv.id = t.reviewer_id \
    LEFT JOIN users im ON im.id = t.implementer_id";

fn user_ref(id: i64, email: String, name: String) -> UserRef {
    let name = if name.trim().is_empty() {
        email.clone()
    } else {
        name
    };
    UserRef { id, name, email }
}

fn optional_ref(id: Option<i64>, email: Option<String>, name: Option<String>) -> Option<UserRef> {
    match (id, email, name) {
        (Some(id), Some(email), Some(name)) => Some(user_ref(id, email, name)),
        _ => None,
    }
}

fn summary_of(r: &TaskRow, tags: Vec<TagOut>) -> AppResult<TaskSummary> {
    Ok(TaskSummary {
        id: r.id,
        key: format!("TM-{}", r.id),
        title: r.title.clone(),
        expected_by_date: r.expected_by_date.clone(),
        status: Status::parse(&r.status)?,
        priority: Priority::parse(&r.priority)?,
        blocked: r.blocked_reason.is_some(),
        blocked_reason: r.blocked_reason.clone(),
        reviewer: optional_ref(r.reviewer_id, r.rv_email.clone(), r.rv_name.clone()),
        implementer: optional_ref(r.implementer_id, r.im_email.clone(), r.im_name.clone()),
        tags,
    })
}

async fn load_task_tags(
    pool: &SqlitePool,
    task_ids: &[i64],
) -> AppResult<HashMap<i64, Vec<TagOut>>> {
    let mut tags_by_task = HashMap::new();
    if task_ids.is_empty() {
        return Ok(tags_by_task);
    }
    let placeholders = vec!["?"; task_ids.len()].join(", ");
    let sql = format!(
        "SELECT tt.task_id, t.id, t.name, t.description, t.archived FROM task_tags tt \
         JOIN tags t ON t.id = tt.tag_id WHERE tt.task_id IN ({placeholders}) \
         ORDER BY t.name COLLATE NOCASE, t.id"
    );
    let mut query = sqlx::query_as::<_, TaskTagRow>(&sql);
    for id in task_ids {
        query = query.bind(id);
    }
    for row in query.fetch_all(pool).await? {
        tags_by_task
            .entry(row.task_id)
            .or_insert_with(Vec::new)
            .push(TagOut {
                id: row.id,
                name: row.name,
                description: row.description,
                archived: row.archived,
            });
    }
    Ok(tags_by_task)
}

async fn summaries_of(pool: &SqlitePool, rows: Vec<TaskRow>) -> AppResult<Vec<TaskSummary>> {
    let ids = rows.iter().map(|row| row.id).collect::<Vec<_>>();
    let mut tags_by_task = load_task_tags(pool, &ids).await?;
    rows.iter()
        .map(|row| summary_of(row, tags_by_task.remove(&row.id).unwrap_or_default()))
        .collect()
}

async fn fetch_row(pool: &SqlitePool, id: i64) -> AppResult<TaskRow> {
    sqlx::query_as::<_, TaskRow>(&format!("{SELECT_TASK} WHERE t.id = ?"))
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)
}

async fn fetch_task(pool: &SqlitePool, id: i64) -> AppResult<TaskOut> {
    let r = fetch_row(pool, id).await?;
    let tags = load_task_tags(pool, &[id])
        .await?
        .remove(&id)
        .unwrap_or_default();
    let history = sqlx::query_as::<_, HistoryRow>(
        "SELECT h.field, h.old_value, h.new_value, h.changed_by, u.email AS u_email, \
         u.display_name AS u_name, h.changed_at FROM task_history h \
         JOIN users u ON u.id = h.changed_by WHERE h.task_id = ? ORDER BY h.id",
    )
    .bind(id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|h| HistoryEntry {
        field: h.field,
        old_value: h.old_value,
        new_value: h.new_value,
        changed_by: user_ref(h.changed_by, h.u_email, h.u_name),
        changed_at: h.changed_at,
    })
    .collect();
    let links = sqlx::query_as::<_, LinkRow>(
        "SELECT l.id, l.url, l.label, l.created_by, u.email AS u_email, \
         u.display_name AS u_name, l.created_at FROM task_links l \
         JOIN users u ON u.id = l.created_by WHERE l.task_id = ? ORDER BY l.id",
    )
    .bind(id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|l| LinkOut {
        id: l.id,
        url: l.url,
        label: l.label,
        added_by: user_ref(l.created_by, l.u_email, l.u_name),
        created_at: l.created_at,
    })
    .collect();
    let related_rows = sqlx::query_as::<_, TaskRow>(&format!(
        "{SELECT_TASK} WHERE t.id IN ( \
            SELECT CASE WHEN task_id = ? THEN related_task_id ELSE task_id END \
            FROM task_relations WHERE task_id = ? OR related_task_id = ? \
         ) ORDER BY t.id"
    ))
    .bind(id)
    .bind(id)
    .bind(id)
    .fetch_all(pool)
    .await?;
    let related_tasks = summaries_of(pool, related_rows).await?;
    Ok(TaskOut {
        summary: summary_of(&r, tags)?,
        details: r.details,
        business_requirements: r.business_requirements,
        technical_requirements: r.technical_requirements,
        implementation_notes: r.implementation_notes,
        created_by: user_ref(r.created_by, r.cb_email, r.cb_name),
        created_at: r.created_at,
        updated_at: r.updated_at,
        links,
        related_tasks,
        history,
    })
}

async fn list_by_status(pool: &SqlitePool, status: Status) -> AppResult<Vec<TaskSummary>> {
    let rows =
        sqlx::query_as::<_, TaskRow>(&format!("{SELECT_TASK} WHERE t.status = ? ORDER BY t.id"))
            .bind(status.as_str())
            .fetch_all(pool)
            .await?;
    summaries_of(pool, rows).await
}

#[derive(Serialize)]
pub struct BoardList {
    status: Status,
    tasks: Vec<TaskSummary>,
}

pub async fn list(
    State(state): State<AppState>,
    _user: AuthUser,
) -> AppResult<Json<Vec<BoardList>>> {
    let mut out = Vec::with_capacity(BOARD.len());
    for status in BOARD {
        out.push(BoardList {
            status,
            tasks: list_by_status(&state.pool, status).await?,
        });
    }
    Ok(Json(out))
}

#[derive(Deserialize)]
pub struct PageQuery {
    page: Option<i64>,
    per_page: Option<i64>,
    search: Option<String>,
    assignee_id: Option<i64>,
    tag_ids: Option<String>,
}

#[derive(Serialize)]
pub struct Page {
    items: Vec<TaskSummary>,
    page: i64,
    per_page: i64,
    total: i64,
}

async fn paged(pool: &SqlitePool, status: Status, q: PageQuery) -> AppResult<Json<Page>> {
    let page = q.page.unwrap_or(1).max(1);
    let per_page = q.per_page.unwrap_or(25).clamp(1, 100);
    let search = q.search.as_deref().unwrap_or("").trim();
    let raw_tag_ids = q.tag_ids.as_deref().unwrap_or("");
    let mut tag_ids = if raw_tag_ids.is_empty() {
        Vec::new()
    } else {
        raw_tag_ids
            .split(',')
            .map(|id| {
                id.parse::<i64>()
                    .map_err(|_| AppError::BadRequest("tag_ids must be comma-separated IDs".into()))
            })
            .collect::<AppResult<Vec<_>>>()?
    };
    if tag_ids.iter().any(|id| *id < 1) {
        return Err(AppError::BadRequest(
            "tag_ids must contain positive IDs".into(),
        ));
    }
    tag_ids.sort_unstable();
    tag_ids.dedup();
    if tag_ids.len() > 50 {
        return Err(AppError::BadRequest(
            "at most 50 tags can be selected".into(),
        ));
    }
    let tag_filter = " AND EXISTS (SELECT 1 FROM task_tags tt \
        WHERE tt.task_id = t.id AND tt.tag_id = ?)"
        .repeat(tag_ids.len());
    let count_sql = format!(
        "SELECT COUNT(*) FROM tasks t WHERE t.status = ? \
         AND (? = '' OR instr(lower('TM-' || t.id), lower(?)) > 0 \
              OR instr(lower(t.title), lower(?)) > 0) \
         AND (? IS NULL OR t.reviewer_id = ? OR t.implementer_id = ?){tag_filter}"
    );
    let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql)
        .bind(status.as_str())
        .bind(search)
        .bind(search)
        .bind(search)
        .bind(q.assignee_id)
        .bind(q.assignee_id)
        .bind(q.assignee_id);
    for id in &tag_ids {
        count_query = count_query.bind(*id);
    }
    let total = count_query.fetch_one(pool).await?;
    let page_sql = format!(
        "{SELECT_TASK} WHERE t.status = ? \
         AND (? = '' OR instr(lower('TM-' || t.id), lower(?)) > 0 \
              OR instr(lower(t.title), lower(?)) > 0) \
         AND (? IS NULL OR t.reviewer_id = ? OR t.implementer_id = ?) \
         {tag_filter} ORDER BY t.id LIMIT ? OFFSET ?"
    );
    let mut items_query = sqlx::query_as::<_, TaskRow>(&page_sql)
        .bind(status.as_str())
        .bind(search)
        .bind(search)
        .bind(search)
        .bind(q.assignee_id)
        .bind(q.assignee_id)
        .bind(q.assignee_id);
    for id in &tag_ids {
        items_query = items_query.bind(*id);
    }
    let rows = items_query
        .bind(per_page)
        .bind((page - 1).saturating_mul(per_page))
        .fetch_all(pool)
        .await?;
    let items = summaries_of(pool, rows).await?;
    Ok(Json(Page {
        items,
        page,
        per_page,
        total,
    }))
}

pub async fn list_archived(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(q): Query<PageQuery>,
) -> AppResult<Json<Page>> {
    paged(&state.pool, Status::Archived, q).await
}

pub async fn list_removed(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(q): Query<PageQuery>,
) -> AppResult<Json<Page>> {
    paged(&state.pool, Status::Removed, q).await
}

pub async fn get_one(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<i64>,
) -> AppResult<Json<TaskOut>> {
    Ok(Json(fetch_task(&state.pool, id).await?))
}

#[derive(Deserialize)]
pub struct TaskSearchQuery {
    q: String,
}

pub async fn search(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(q): Query<TaskSearchQuery>,
) -> AppResult<Json<Vec<TaskSummary>>> {
    let q = q.q.trim();
    if q.is_empty() || q.chars().count() > 200 {
        return Err(AppError::BadRequest("q must be 1-200 characters".into()));
    }
    let rows = sqlx::query_as::<_, TaskRow>(&format!(
        "{SELECT_TASK} WHERE t.status != 'archived' \
         AND (instr(lower('TM-' || t.id), lower(?)) > 0 \
              OR instr(lower(t.title), lower(?)) > 0) \
         ORDER BY t.id DESC LIMIT 20"
    ))
    .bind(q)
    .bind(q)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(summaries_of(&state.pool, rows).await?))
}

fn double_option<'de, D, T>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(d).map(Some)
}

#[derive(Deserialize)]
pub struct NewTask {
    title: String,
    expected_by_date: Option<String>,
    #[serde(default)]
    details: String,
    #[serde(default)]
    business_requirements: String,
    #[serde(default)]
    technical_requirements: String,
    #[serde(default)]
    implementation_notes: String,
    status: Option<Status>,
    priority: Option<Priority>,
    reviewer_id: Option<i64>,
    implementer_id: Option<i64>,
}

#[derive(Deserialize)]
pub struct UpdateTask {
    title: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    expected_by_date: Option<Option<String>>,
    details: Option<String>,
    business_requirements: Option<String>,
    technical_requirements: Option<String>,
    implementation_notes: Option<String>,
    status: Option<Status>,
    priority: Option<Priority>,
    #[serde(default, deserialize_with = "double_option")]
    reviewer_id: Option<Option<i64>>,
    #[serde(default, deserialize_with = "double_option")]
    implementer_id: Option<Option<i64>>,
}

const MAX_TEXT: usize = 20_000;

fn validate_title(t: &str) -> AppResult<()> {
    if t.is_empty() || t.chars().count() > 200 {
        return Err(AppError::BadRequest(
            "title must be 1-200 characters".into(),
        ));
    }
    Ok(())
}

fn validate_expected_by_date(date: &str) -> AppResult<()> {
    let invalid =
        || AppError::BadRequest("expected_by_date must be a valid YYYY-MM-DD date".into());
    let bytes = date.as_bytes();
    if bytes.len() != 10
        || !bytes.iter().all(|b| b.is_ascii_digit() || *b == b'-')
        || bytes[4] != b'-'
        || bytes[7] != b'-'
    {
        return Err(invalid());
    }
    let year = date[..4].parse::<u32>().map_err(|_| invalid())?;
    let month = date[5..7].parse::<u32>().map_err(|_| invalid())?;
    let day = date[8..10].parse::<u32>().map_err(|_| invalid())?;
    if !(1..=9999).contains(&year) || !(1..=12).contains(&month) {
        return Err(invalid());
    }
    let leap_year = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days_in_month = match month {
        2 if leap_year => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    if !(1..=days_in_month).contains(&day) {
        return Err(invalid());
    }
    Ok(())
}

fn validate_text(name: &str, t: &str) -> AppResult<()> {
    if t.chars().count() > MAX_TEXT {
        return Err(AppError::BadRequest(format!(
            "{name} must be at most {MAX_TEXT} characters"
        )));
    }
    Ok(())
}

async fn check_assignee(pool: &SqlitePool, id: Option<i64>) -> AppResult<()> {
    if let Some(id) = id {
        let active: Option<bool> = sqlx::query_scalar("SELECT is_active FROM users WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?;
        if active != Some(true) {
            return Err(AppError::BadRequest(
                "assignee must be an active user".into(),
            ));
        }
    }
    Ok(())
}

async fn record_history<'a>(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    task_id: i64,
    field: &str,
    old: Option<&str>,
    new: impl Into<Option<&'a str>>,
    user_id: i64,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO task_history (task_id, field, old_value, new_value, changed_by) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(task_id)
    .bind(field)
    .bind(old)
    .bind(new.into())
    .bind(user_id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn create(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<NewTask>,
) -> AppResult<(StatusCode, Json<TaskOut>)> {
    let title = req.title.trim();
    validate_title(title)?;
    if let Some(date) = &req.expected_by_date {
        validate_expected_by_date(date)?;
    }
    for (n, v) in [
        ("details", &req.details),
        ("business_requirements", &req.business_requirements),
        ("technical_requirements", &req.technical_requirements),
        ("implementation_notes", &req.implementation_notes),
    ] {
        validate_text(n, v)?;
    }
    check_assignee(&state.pool, req.reviewer_id).await?;
    check_assignee(&state.pool, req.implementer_id).await?;
    let status = req.status.unwrap_or(Status::Backlog);
    let priority = req.priority.unwrap_or(Priority::Normal);

    let mut tx = state.pool.begin().await?;
    let id = sqlx::query(
        "INSERT INTO tasks (title, details, business_requirements, technical_requirements, \
         implementation_notes, status, priority, expected_by_date, created_by, reviewer_id, \
         implementer_id) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(title)
    .bind(&req.details)
    .bind(&req.business_requirements)
    .bind(&req.technical_requirements)
    .bind(&req.implementation_notes)
    .bind(status.as_str())
    .bind(priority.as_str())
    .bind(&req.expected_by_date)
    .bind(user.id)
    .bind(req.reviewer_id)
    .bind(req.implementer_id)
    .execute(&mut *tx)
    .await?
    .last_insert_rowid();
    record_history(&mut tx, id, "status", None, status.as_str(), user.id).await?;
    record_history(&mut tx, id, "priority", None, priority.as_str(), user.id).await?;
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(fetch_task(&state.pool, id).await?),
    ))
}

pub async fn update(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateTask>,
) -> AppResult<Json<TaskOut>> {
    let cur = fetch_row(&state.pool, id).await?;
    let cur_status = Status::parse(&cur.status)?;
    let cur_priority = Priority::parse(&cur.priority)?;

    let non_status_edit = req.title.is_some()
        || req.details.is_some()
        || req.business_requirements.is_some()
        || req.technical_requirements.is_some()
        || req.implementation_notes.is_some()
        || req.expected_by_date.is_some()
        || req.priority.is_some()
        || req.reviewer_id.is_some()
        || req.implementer_id.is_some();
    if cur_status == Status::Archived && non_status_edit {
        return Err(AppError::Conflict(
            "archived tasks are read-only; change the status to restore it first".into(),
        ));
    }

    let title = req
        .title
        .as_deref()
        .map(str::trim)
        .unwrap_or(&cur.title)
        .to_string();
    validate_title(&title)?;
    let pick = |new: &Option<String>, old: &str| new.clone().unwrap_or_else(|| old.to_string());
    let details = pick(&req.details, &cur.details);
    let business = pick(&req.business_requirements, &cur.business_requirements);
    let technical = pick(&req.technical_requirements, &cur.technical_requirements);
    let notes = pick(&req.implementation_notes, &cur.implementation_notes);
    let expected_by_date = req
        .expected_by_date
        .clone()
        .unwrap_or_else(|| cur.expected_by_date.clone());
    if let Some(date) = &expected_by_date {
        validate_expected_by_date(date)?;
    }
    for (n, v) in [
        ("details", &details),
        ("business_requirements", &business),
        ("technical_requirements", &technical),
        ("implementation_notes", &notes),
    ] {
        validate_text(n, v)?;
    }

    let status = req.status.unwrap_or(cur_status);
    let priority = req.priority.unwrap_or(cur_priority);
    let reviewer = req.reviewer_id.unwrap_or(cur.reviewer_id);
    let implementer = req.implementer_id.unwrap_or(cur.implementer_id);
    if reviewer != cur.reviewer_id {
        check_assignee(&state.pool, reviewer).await?;
    }
    if implementer != cur.implementer_id {
        check_assignee(&state.pool, implementer).await?;
    }

    let mut tx = state.pool.begin().await?;
    sqlx::query(
        "UPDATE tasks SET title = ?, details = ?, business_requirements = ?, \
         technical_requirements = ?, implementation_notes = ?, status = ?, priority = ?, \
         expected_by_date = ?, reviewer_id = ?, implementer_id = ?, \
         updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now') WHERE id = ?",
    )
    .bind(&title)
    .bind(&details)
    .bind(&business)
    .bind(&technical)
    .bind(&notes)
    .bind(status.as_str())
    .bind(priority.as_str())
    .bind(&expected_by_date)
    .bind(reviewer)
    .bind(implementer)
    .bind(id)
    .execute(&mut *tx)
    .await?;
    if status != cur_status {
        record_history(
            &mut tx,
            id,
            "status",
            Some(cur_status.as_str()),
            status.as_str(),
            user.id,
        )
        .await?;
    }
    if priority != cur_priority {
        record_history(
            &mut tx,
            id,
            "priority",
            Some(cur_priority.as_str()),
            priority.as_str(),
            user.id,
        )
        .await?;
    }
    tx.commit().await?;
    Ok(Json(fetch_task(&state.pool, id).await?))
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Up,
    Down,
}

#[derive(Deserialize)]
pub struct MoveReq {
    direction: Direction,
}

pub async fn move_task(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<MoveReq>,
) -> AppResult<Json<TaskOut>> {
    let cur = Status::parse(&fetch_row(&state.pool, id).await?.status)?;
    let idx = BOARD
        .iter()
        .position(|s| *s == cur)
        .ok_or_else(|| AppError::BadRequest("task is not on the board".into()))?;
    let target = match req.direction {
        Direction::Up => idx.checked_sub(1),
        Direction::Down => Some(idx + 1).filter(|i| *i < BOARD.len()),
    }
    .map(|i| BOARD[i])
    .ok_or_else(|| AppError::BadRequest("cannot move further in that direction".into()))?;

    let mut tx = state.pool.begin().await?;
    sqlx::query(
        "UPDATE tasks SET status = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now') \
         WHERE id = ?",
    )
    .bind(target.as_str())
    .bind(id)
    .execute(&mut *tx)
    .await?;
    record_history(
        &mut tx,
        id,
        "status",
        Some(cur.as_str()),
        target.as_str(),
        user.id,
    )
    .await?;
    tx.commit().await?;
    Ok(Json(fetch_task(&state.pool, id).await?))
}

#[derive(Serialize, FromRow)]
pub struct Assignable {
    id: i64,
    email: String,
    display_name: String,
}

pub async fn assignable_users(
    State(state): State<AppState>,
    _user: AuthUser,
) -> AppResult<Json<Vec<Assignable>>> {
    Ok(Json(
        sqlx::query_as::<_, Assignable>(
            "SELECT id, email, display_name FROM users WHERE is_active = 1 ORDER BY email",
        )
        .fetch_all(&state.pool)
        .await?,
    ))
}

const MAX_URL: usize = 2000;
const MAX_LABEL: usize = 200;
const MAX_LINKS: i64 = 50;

fn validate_url(url: &str) -> AppResult<()> {
    let bad = || AppError::BadRequest("url must be a valid http or https address".into());
    if url.chars().count() > MAX_URL {
        return Err(AppError::BadRequest(format!(
            "url must be at most {MAX_URL} characters"
        )));
    }
    if url.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(bad());
    }
    let lower = url.to_ascii_lowercase();
    let rest = lower
        .strip_prefix("https://")
        .or_else(|| lower.strip_prefix("http://"))
        .ok_or_else(bad)?;
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host = authority.rsplit('@').next().unwrap_or("");
    if host.is_empty() || host.starts_with(':') {
        return Err(bad());
    }
    Ok(())
}

#[derive(Deserialize)]
pub struct NewLink {
    url: String,
    #[serde(default)]
    label: String,
}

async fn ensure_editable(pool: &SqlitePool, id: i64) -> AppResult<()> {
    if Status::parse(&fetch_row(pool, id).await?.status)? == Status::Archived {
        return Err(AppError::Conflict(
            "archived tasks are read-only; change the status to restore it first".into(),
        ));
    }
    Ok(())
}

const MAX_BLOCK_REASON: usize = 500;

#[derive(Deserialize)]
pub struct BlockReq {
    reason: String,
}

pub async fn block(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<BlockReq>,
) -> AppResult<Json<TaskOut>> {
    let reason = req.reason.trim();
    if reason.is_empty() || reason.chars().count() > MAX_BLOCK_REASON {
        return Err(AppError::BadRequest(format!(
            "reason must be 1-{MAX_BLOCK_REASON} characters"
        )));
    }
    ensure_editable(&state.pool, id).await?;
    let mut tx = state.pool.begin().await?;
    let changed = sqlx::query(
        "UPDATE tasks SET blocked_reason = ?, \
         updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now') \
         WHERE id = ? AND blocked_reason IS NULL",
    )
    .bind(reason)
    .bind(id)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if changed == 0 {
        return Err(AppError::Conflict("task is already blocked".into()));
    }
    record_history(&mut tx, id, "blocked", None, reason, user.id).await?;
    tx.commit().await?;
    Ok(Json(fetch_task(&state.pool, id).await?))
}

pub async fn unblock(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<i64>,
) -> AppResult<Json<TaskOut>> {
    ensure_editable(&state.pool, id).await?;
    let mut tx = state.pool.begin().await?;
    let reason: Option<String> =
        sqlx::query_scalar("SELECT blocked_reason FROM tasks WHERE id = ?")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(AppError::NotFound)?;
    let reason = reason.ok_or_else(|| AppError::Conflict("task is not blocked".into()))?;
    sqlx::query(
        "UPDATE tasks SET blocked_reason = NULL, \
         updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now') WHERE id = ?",
    )
    .bind(id)
    .execute(&mut *tx)
    .await?;
    record_history(&mut tx, id, "unblocked", Some(&reason), None, user.id).await?;
    tx.commit().await?;
    Ok(Json(fetch_task(&state.pool, id).await?))
}

pub async fn add_link(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<NewLink>,
) -> AppResult<(StatusCode, Json<TaskOut>)> {
    let url = req.url.trim();
    let label = req.label.trim();
    validate_url(url)?;
    if label.chars().count() > MAX_LABEL {
        return Err(AppError::BadRequest(format!(
            "label must be at most {MAX_LABEL} characters"
        )));
    }
    ensure_editable(&state.pool, id).await?;

    let mut tx = state.pool.begin().await?;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM task_links WHERE task_id = ?")
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
    if count >= MAX_LINKS {
        return Err(AppError::BadRequest(format!(
            "a task can have at most {MAX_LINKS} links"
        )));
    }
    let inserted = sqlx::query(
        "INSERT INTO task_links (task_id, url, label, created_by) VALUES (?, ?, ?, ?) \
         ON CONFLICT(task_id, url) DO NOTHING",
    )
    .bind(id)
    .bind(url)
    .bind(label)
    .bind(user.id)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if inserted == 0 {
        return Err(AppError::Conflict(
            "this url is already linked to the task".into(),
        ));
    }
    record_history(&mut tx, id, "link_added", None, url, user.id).await?;
    touch(&mut tx, id).await?;
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(fetch_task(&state.pool, id).await?),
    ))
}

pub async fn remove_link(
    State(state): State<AppState>,
    user: AuthUser,
    Path((id, link_id)): Path<(i64, i64)>,
) -> AppResult<Json<TaskOut>> {
    ensure_editable(&state.pool, id).await?;
    let mut tx = state.pool.begin().await?;
    let url: String =
        sqlx::query_scalar("DELETE FROM task_links WHERE id = ? AND task_id = ? RETURNING url")
            .bind(link_id)
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(AppError::NotFound)?;
    record_history(&mut tx, id, "link_removed", Some(&url), None, user.id).await?;
    touch(&mut tx, id).await?;
    tx.commit().await?;
    Ok(Json(fetch_task(&state.pool, id).await?))
}

#[derive(Deserialize)]
pub struct AddRelatedTask {
    related_task_id: i64,
}

async fn ensure_relation_editable(pool: &SqlitePool, id: i64, related_id: i64) -> AppResult<()> {
    if id == related_id {
        return Err(AppError::BadRequest(
            "a task cannot be related to itself".into(),
        ));
    }
    ensure_editable(pool, id).await?;
    ensure_editable(pool, related_id).await?;
    Ok(())
}

fn relation_ids(id: i64, related_id: i64) -> (i64, i64) {
    if id < related_id {
        (id, related_id)
    } else {
        (related_id, id)
    }
}

pub async fn add_related_task(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<AddRelatedTask>,
) -> AppResult<(StatusCode, Json<TaskOut>)> {
    if req.related_task_id < 1 {
        return Err(AppError::BadRequest(
            "related_task_id must be a positive task ID".into(),
        ));
    }
    ensure_relation_editable(&state.pool, id, req.related_task_id).await?;
    let (task_id, related_task_id) = relation_ids(id, req.related_task_id);
    let mut tx = state.pool.begin().await?;
    let inserted = sqlx::query(
        "INSERT INTO task_relations (task_id, related_task_id, created_by) \
         VALUES (?, ?, ?) ON CONFLICT(task_id, related_task_id) DO NOTHING",
    )
    .bind(task_id)
    .bind(related_task_id)
    .bind(user.id)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if inserted == 0 {
        return Err(AppError::Conflict("these tasks are already related".into()));
    }
    touch(&mut tx, task_id).await?;
    touch(&mut tx, related_task_id).await?;
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(fetch_task(&state.pool, id).await?),
    ))
}

pub async fn remove_related_task(
    State(state): State<AppState>,
    _user: AuthUser,
    Path((id, related_id)): Path<(i64, i64)>,
) -> AppResult<Json<TaskOut>> {
    ensure_relation_editable(&state.pool, id, related_id).await?;
    let (task_id, related_task_id) = relation_ids(id, related_id);
    let mut tx = state.pool.begin().await?;
    let removed =
        sqlx::query("DELETE FROM task_relations WHERE task_id = ? AND related_task_id = ?")
            .bind(task_id)
            .bind(related_task_id)
            .execute(&mut *tx)
            .await?
            .rows_affected();
    if removed == 0 {
        return Err(AppError::NotFound);
    }
    touch(&mut tx, task_id).await?;
    touch(&mut tx, related_task_id).await?;
    tx.commit().await?;
    Ok(Json(fetch_task(&state.pool, id).await?))
}

#[derive(Deserialize)]
pub struct AddTag {
    tag_id: i64,
}

pub async fn add_tag(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<AddTag>,
) -> AppResult<Json<TaskOut>> {
    fetch_row(&state.pool, id).await?;
    let tag_archived: Option<bool> = sqlx::query_scalar("SELECT archived FROM tags WHERE id = ?")
        .bind(req.tag_id)
        .fetch_optional(&state.pool)
        .await?;
    match tag_archived {
        None => return Err(AppError::NotFound),
        Some(true) => {
            return Err(AppError::Conflict(
                "archived tags cannot be assigned to tasks".into(),
            ));
        }
        Some(false) => {}
    }
    let mut tx = state.pool.begin().await?;
    let inserted = sqlx::query(
        "INSERT INTO task_tags (task_id, tag_id) VALUES (?, ?) \
         ON CONFLICT(task_id, tag_id) DO NOTHING",
    )
    .bind(id)
    .bind(req.tag_id)
    .execute(&mut *tx)
    .await?;
    if inserted.rows_affected() > 0 {
        touch(&mut tx, id).await?;
    }
    tx.commit().await?;
    Ok(Json(fetch_task(&state.pool, id).await?))
}

pub async fn remove_tag(
    State(state): State<AppState>,
    _user: AuthUser,
    Path((id, tag_id)): Path<(i64, i64)>,
) -> AppResult<Json<TaskOut>> {
    fetch_row(&state.pool, id).await?;
    let mut tx = state.pool.begin().await?;
    let removed = sqlx::query("DELETE FROM task_tags WHERE task_id = ? AND tag_id = ?")
        .bind(id)
        .bind(tag_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    if removed == 0 {
        return Err(AppError::NotFound);
    }
    touch(&mut tx, id).await?;
    tx.commit().await?;
    Ok(Json(fetch_task(&state.pool, id).await?))
}

async fn touch(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, id: i64) -> AppResult<()> {
    sqlx::query("UPDATE tasks SET updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now') WHERE id = ?")
        .bind(id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}
