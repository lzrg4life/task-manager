# Database

SQLite via sqlx; migrations in `migrations/` (embedded at compile time). Foreign keys on, WAL mode for file DBs.

- `users` (id, email unique nocase, password_hash [Argon2id PHC], display_name, is_active, created_at, updated_at)
- `roles` (id, name unique nocase, description) – `admin` seeded by migration 0001
- `user_roles` (user_id, role_id) – many-to-many
- `refresh_tokens` (id, user_id, token_hash [SHA-256 hex], family_id, expires_at, revoked_at, replaced_by, created_at) – times are unix seconds

Add changes as new numbered migration files; never edit an applied one.

## Tickets (migration 0002)
- `tickets` (id, title, details, business_requirements, technical_requirements, implementation_notes, status, priority, created_by, reviewer_id, implementer_id, created_at, updated_at) – status/priority constrained by CHECK; user-friendly key `TM-<id>` is derived from id
- `ticket_history` (id, ticket_id, field `status|priority`, old_value (NULL on creation), new_value, changed_by, changed_at)
