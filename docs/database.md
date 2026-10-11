# Database

SQLite via sqlx; migrations in `migrations/` (embedded at compile time). Foreign keys on, WAL mode for file DBs.

- `users` (id, email unique nocase, password_hash [Argon2id PHC], display_name, is_active, is_admin, created_at, updated_at)
- `refresh_tokens` (id, user_id, token_hash [SHA-256 hex], family_id, expires_at, revoked_at, replaced_by, created_at) – times are unix seconds

Add changes as new numbered migration files; never edit an applied one.

Migration 0004 converts existing `admin` memberships to `is_admin = 1` and removes the obsolete role tables.

Migration 0010 renames the task-related tables and foreign-key columns introduced by migrations 0002–0009; existing records and relationships are preserved. The earlier migrations remain unchanged as historical upgrade steps.

## Tasks (migration 0011; originally created in 0002)
- `tasks` (id, title, expected_by_date, details, business_requirements, technical_requirements, implementation_notes, status, priority, created_by, reviewer_id, implementer_id, blocked_reason, created_at, updated_at) – `expected_by_date` is nullable and stores a calendar date as `YYYY-MM-DD`; status/priority constrained by CHECK; `blocked_reason` is NULL when not blocked and the reason text when blocked (added in 0008); user-friendly key `TM-<id>` is derived from id
- `task_history` (id, task_id, field `status|priority|link_added|link_removed|blocked|unblocked` [widened in 0003, 0008], old_value (NULL on creation/link add/block), new_value (NULL on link removal/unblock), changed_by, changed_at)

## Task links (migrations 0003, 0010)
- `task_links` (id, task_id, url, label, created_by, created_at; unique on task_id+url) – zero or more per task; removal deletes the row, and the add/remove is kept in `task_history`

## Task relations (migrations 0007, 0010)
- `task_relations` (task_id, related_task_id, created_by, created_at; composite primary key) – symmetric task relationships stored once with the lower task ID first; self-relations are prevented

## Task comments (migrations 0009, 0010)
- `task_comments` (id, task_id, user_id, body, created_at, updated_at, deleted_at) – comments belong to a task and an author; deleted comments are retained and hidden from API responses except for their deleted marker

## Tags (migrations 0005-0006, 0010)
- `tags` (id, name [unique, case-insensitive], description, archived) – created and archived by admins; archived tags remain associated but are excluded from new assignments
- `task_tags` (task_id, tag_id; composite primary key) – zero or more tag associations per task; both foreign keys cascade on deletion
