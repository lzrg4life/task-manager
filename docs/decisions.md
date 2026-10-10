# Decisions

- axum + tokio: mainstream, tower ecosystem.
- sqlx (runtime queries, no compile-time DB needed) with embedded migrations.
- Argon2id for passwords; per-hash random salt.
- Vanilla HTML/CSS/JS, no build step, to keep the client simple.
- JWT HS256 access tokens (short TTL) + opaque rotating refresh tokens stored hashed; reuse detection revokes the family.
- Authorization uses a single `is_admin` boolean on each user; non-admin users have no additional role concept.
- Users are deactivated, not deleted. First admin is created only via `init-db`.
