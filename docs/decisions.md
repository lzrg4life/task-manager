# Decisions

- axum + tokio: mainstream, tower ecosystem.
- sqlx (runtime queries, no compile-time DB needed) with embedded migrations.
- Argon2id for passwords; per-hash random salt.
- Vanilla HTML/CSS/JS, no build step, to keep the client simple.
- JWT HS256 access tokens (short TTL) + opaque rotating refresh tokens stored hashed; reuse detection revokes the family.
- Free-form roles table; `admin` is the only built-in role. Permissions per role are a future extension.
- Users are deactivated, not deleted. First admin is created only via `init-db`.
