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
pub struct TicketSummary {
    id: i64,
    key: String,
    title: String,
    status: Status,
    priority: Priority,
    reviewer: Option<UserRef>,
    implementer: Option<UserRef>,
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
pub struct TicketOut {
    #[serde(flatten)]
    summary: TicketSummary,
    details: String,
    business_requirements: String,
    technical_requirements: String,
    implementation_notes: String,
    created_by: UserRef,
    created_at: String,
    updated_at: String,
    links: Vec<LinkOut>,
    history: Vec<HistoryEntry>,
}

#[derive(FromRow)]
struct TicketRow {
    id: i64,
    title: String,
    details: String,
    business_requirements: String,
    technical_requirements: String,
    implementation_notes: String,
    status: String,
    priority: String,
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

const SELECT_TICKET: &str = "SELECT t.id, t.title, t.details, t.business_requirements, \
    t.technical_requirements, t.implementation_notes, t.status, t.priority, t.created_at, \
    t.updated_at, t.created_by, cb.email AS cb_email, cb.display_name AS cb_name, \
    t.reviewer_id, rv.email AS rv_email, rv.display_name AS rv_name, \
    t.implementer_id, im.email AS im_email, im.display_name AS im_name \
    FROM tickets t JOIN users cb ON cb.id = t.created_by \
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

fn summary_of(r: &TicketRow) -> AppResult<TicketSummary> {
    Ok(TicketSummary {
        id: r.id,
        key: format!("TM-{}", r.id),
        title: r.title.clone(),
        status: Status::parse(&r.status)?,
        priority: Priority::parse(&r.priority)?,
        reviewer: optional_ref(r.reviewer_id, r.rv_email.clone(), r.rv_name.clone()),
        implementer: optional_ref(r.implementer_id, r.im_email.clone(), r.im_name.clone()),
    })
}

async fn fetch_row(pool: &SqlitePool, id: i64) -> AppResult<TicketRow> {
    sqlx::query_as::<_, TicketRow>(&format!("{SELECT_TICKET} WHERE t.id = ?"))
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)
}

async fn fetch_ticket(pool: &SqlitePool, id: i64) -> AppResult<TicketOut> {
    let r = fetch_row(pool, id).await?;
    let history = sqlx::query_as::<_, HistoryRow>(
        "SELECT h.field, h.old_value, h.new_value, h.changed_by, u.email AS u_email, \
         u.display_name AS u_name, h.changed_at FROM ticket_history h \
         JOIN users u ON u.id = h.changed_by WHERE h.ticket_id = ? ORDER BY h.id",
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
         u.display_name AS u_name, l.created_at FROM ticket_links l \
         JOIN users u ON u.id = l.created_by WHERE l.ticket_id = ? ORDER BY l.id",
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
    Ok(TicketOut {
        summary: summary_of(&r)?,
        details: r.details,
        business_requirements: r.business_requirements,
        technical_requirements: r.technical_requirements,
        implementation_notes: r.implementation_notes,
        created_by: user_ref(r.created_by, r.cb_email, r.cb_name),
        created_at: r.created_at,
        updated_at: r.updated_at,
        links,
        history,
    })
}

async fn list_by_status(pool: &SqlitePool, status: Status) -> AppResult<Vec<TicketSummary>> {
    sqlx::query_as::<_, TicketRow>(&format!("{SELECT_TICKET} WHERE t.status = ? ORDER BY t.id"))
        .bind(status.as_str())
        .fetch_all(pool)
        .await?
        .iter()
        .map(summary_of)
        .collect()
}

#[derive(Serialize)]
pub struct BoardList {
    status: Status,
    tickets: Vec<TicketSummary>,
}

pub async fn list(
    State(state): State<AppState>,
    _user: AuthUser,
) -> AppResult<Json<Vec<BoardList>>> {
    let mut out = Vec::with_capacity(BOARD.len());
    for status in BOARD {
        out.push(BoardList {
            status,
            tickets: list_by_status(&state.pool, status).await?,
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
}

#[derive(Serialize)]
pub struct Page {
    items: Vec<TicketSummary>,
    page: i64,
    per_page: i64,
    total: i64,
}

async fn paged(pool: &SqlitePool, status: Status, q: PageQuery) -> AppResult<Json<Page>> {
    let page = q.page.unwrap_or(1).max(1);
    let per_page = q.per_page.unwrap_or(25).clamp(1, 100);
    let search = q.search.as_deref().unwrap_or("").trim();
    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM tickets t WHERE t.status = ? \
         AND (? = '' OR instr(lower('TM-' || t.id), lower(?)) > 0 \
              OR instr(lower(t.title), lower(?)) > 0) \
         AND (? IS NULL OR t.reviewer_id = ? OR t.implementer_id = ?)",
    )
    .bind(status.as_str())
    .bind(search)
    .bind(search)
    .bind(search)
    .bind(q.assignee_id)
    .bind(q.assignee_id)
    .bind(q.assignee_id)
    .fetch_one(pool)
    .await?;
    let items = sqlx::query_as::<_, TicketRow>(&format!(
        "{SELECT_TICKET} WHERE t.status = ? \
         AND (? = '' OR instr(lower('TM-' || t.id), lower(?)) > 0 \
              OR instr(lower(t.title), lower(?)) > 0) \
         AND (? IS NULL OR t.reviewer_id = ? OR t.implementer_id = ?) \
         ORDER BY t.id LIMIT ? OFFSET ?"
    ))
    .bind(status.as_str())
    .bind(search)
    .bind(search)
    .bind(search)
    .bind(q.assignee_id)
    .bind(q.assignee_id)
    .bind(q.assignee_id)
    .bind(per_page)
    .bind((page - 1).saturating_mul(per_page))
    .fetch_all(pool)
    .await?
    .iter()
    .map(summary_of)
    .collect::<AppResult<Vec<_>>>()?;
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
) -> AppResult<Json<TicketOut>> {
    Ok(Json(fetch_ticket(&state.pool, id).await?))
}

fn double_option<'de, D, T>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(d).map(Some)
}

#[derive(Deserialize)]
pub struct NewTicket {
    title: String,
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
pub struct UpdateTicket {
    title: Option<String>,
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
    ticket_id: i64,
    field: &str,
    old: Option<&str>,
    new: impl Into<Option<&'a str>>,
    user_id: i64,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO ticket_history (ticket_id, field, old_value, new_value, changed_by) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(ticket_id)
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
    Json(req): Json<NewTicket>,
) -> AppResult<(StatusCode, Json<TicketOut>)> {
    let title = req.title.trim();
    validate_title(title)?;
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
        "INSERT INTO tickets (title, details, business_requirements, technical_requirements, \
         implementation_notes, status, priority, created_by, reviewer_id, implementer_id) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(title)
    .bind(&req.details)
    .bind(&req.business_requirements)
    .bind(&req.technical_requirements)
    .bind(&req.implementation_notes)
    .bind(status.as_str())
    .bind(priority.as_str())
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
        Json(fetch_ticket(&state.pool, id).await?),
    ))
}

pub async fn update(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateTicket>,
) -> AppResult<Json<TicketOut>> {
    let cur = fetch_row(&state.pool, id).await?;
    let cur_status = Status::parse(&cur.status)?;
    let cur_priority = Priority::parse(&cur.priority)?;

    let non_status_edit = req.title.is_some()
        || req.details.is_some()
        || req.business_requirements.is_some()
        || req.technical_requirements.is_some()
        || req.implementation_notes.is_some()
        || req.priority.is_some()
        || req.reviewer_id.is_some()
        || req.implementer_id.is_some();
    if cur_status == Status::Archived && non_status_edit {
        return Err(AppError::Conflict(
            "archived tickets are read-only; change the status to restore it first".into(),
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
        "UPDATE tickets SET title = ?, details = ?, business_requirements = ?, \
         technical_requirements = ?, implementation_notes = ?, status = ?, priority = ?, \
         reviewer_id = ?, implementer_id = ?, \
         updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now') WHERE id = ?",
    )
    .bind(&title)
    .bind(&details)
    .bind(&business)
    .bind(&technical)
    .bind(&notes)
    .bind(status.as_str())
    .bind(priority.as_str())
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
    Ok(Json(fetch_ticket(&state.pool, id).await?))
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

pub async fn move_ticket(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<MoveReq>,
) -> AppResult<Json<TicketOut>> {
    let cur = Status::parse(&fetch_row(&state.pool, id).await?.status)?;
    let idx = BOARD
        .iter()
        .position(|s| *s == cur)
        .ok_or_else(|| AppError::BadRequest("ticket is not on the board".into()))?;
    let target = match req.direction {
        Direction::Up => idx.checked_sub(1),
        Direction::Down => Some(idx + 1).filter(|i| *i < BOARD.len()),
    }
    .map(|i| BOARD[i])
    .ok_or_else(|| AppError::BadRequest("cannot move further in that direction".into()))?;

    let mut tx = state.pool.begin().await?;
    sqlx::query(
        "UPDATE tickets SET status = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now') \
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
    Ok(Json(fetch_ticket(&state.pool, id).await?))
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
            "archived tickets are read-only; change the status to restore it first".into(),
        ));
    }
    Ok(())
}

pub async fn add_link(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<NewLink>,
) -> AppResult<(StatusCode, Json<TicketOut>)> {
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
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM ticket_links WHERE ticket_id = ?")
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
    if count >= MAX_LINKS {
        return Err(AppError::BadRequest(format!(
            "a ticket can have at most {MAX_LINKS} links"
        )));
    }
    let inserted = sqlx::query(
        "INSERT INTO ticket_links (ticket_id, url, label, created_by) VALUES (?, ?, ?, ?) \
         ON CONFLICT(ticket_id, url) DO NOTHING",
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
            "this url is already linked to the ticket".into(),
        ));
    }
    record_history(&mut tx, id, "link_added", None, url, user.id).await?;
    touch(&mut tx, id).await?;
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(fetch_ticket(&state.pool, id).await?),
    ))
}

pub async fn remove_link(
    State(state): State<AppState>,
    user: AuthUser,
    Path((id, link_id)): Path<(i64, i64)>,
) -> AppResult<Json<TicketOut>> {
    ensure_editable(&state.pool, id).await?;
    let mut tx = state.pool.begin().await?;
    let url: String =
        sqlx::query_scalar("DELETE FROM ticket_links WHERE id = ? AND ticket_id = ? RETURNING url")
            .bind(link_id)
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(AppError::NotFound)?;
    record_history(&mut tx, id, "link_removed", Some(&url), None, user.id).await?;
    touch(&mut tx, id).await?;
    tx.commit().await?;
    Ok(Json(fetch_ticket(&state.pool, id).await?))
}

async fn touch(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, id: i64) -> AppResult<()> {
    sqlx::query(
        "UPDATE tickets SET updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now') WHERE id = ?",
    )
    .bind(id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}
