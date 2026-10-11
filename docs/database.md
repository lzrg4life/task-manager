# Database

SQLite via sqlx; migrations in `migrations/` (embedded at compile time). Foreign keys on, WAL mode for file DBs.

- `users` (id, email unique nocase, password_hash [Argon2id PHC], display_name, is_active, is_admin, created_at, updated_at)
- `refresh_tokens` (id, user_id, token_hash [SHA-256 hex], family_id, expires_at, revoked_at, replaced_by, created_at) – times are unix seconds

Add changes as new numbered migration files; never edit an applied one.

Migration 0004 converts existing `admin` memberships to `is_admin = 1` and removes the obsolete role tables.

## Tickets (migration 0002)
- `tickets` (id, title, details, business_requirements, technical_requirements, implementation_notes, status, priority, created_by, reviewer_id, implementer_id, created_at, updated_at) – status/priority constrained by CHECK; user-friendly key `TM-<id>` is derived from id
- `ticket_history` (id, ticket_id, field `status|priority|link_added|link_removed` [widened in 0003], old_value (NULL on creation/link add), new_value (NULL on link removal), changed_by, changed_at)

## Ticket links (migration 0003)
- `ticket_links` (id, ticket_id, url, label, created_by, created_at; unique on ticket_id+url) – zero or more per ticket; removal deletes the row, and the add/remove is kept in `ticket_history`

## Ticket relations (migration 0007)
- `ticket_relations` (ticket_id, related_ticket_id, created_by, created_at; composite primary key) – symmetric ticket relationships stored once with the lower ticket ID first; self-relations are prevented

## Tags (migrations 0005-0006)
- `tags` (id, name [unique, case-insensitive], description, archived) – created and archived by admins; archived tags remain associated but are excluded from new assignments
- `ticket_tags` (ticket_id, tag_id; composite primary key) – zero or more tag associations per ticket; both foreign keys cascade on deletion
