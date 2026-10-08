# Agent guide

Rust web API (axum 0.8, sqlx 0.8 + SQLite) serving a vanilla JS client from `static/`. Library crate `task_manager` (src/lib.rs) plus thin binary (src/main.rs, clap CLI: `serve` default, `init-db`).

## Commands (Windows/PowerShell)
- Build/test: `cargo test`; lint: `cargo clippy --all-targets -- -D warnings`; format: `cargo fmt`
- Run: set `JWT_SECRET` (>= 32 chars), `cargo run -- init-db --admin-email a@b.co` (password via prompt or `ADMIN_PASSWORD`), then `cargo run -- serve`
- All three of test, clippy, fmt must pass before committing.

## Module map
- `src/config.rs` env config · `src/db.rs` pool, migrations, `create_first_admin` · `src/error.rs` `AppError` -> JSON `{"error": ...}`
- `src/models.rs` `UserOut`, `Role`, fetch/list helpers, email/password validation
- `src/auth/` `password` (Argon2id, dummy-hash verify), `jwt` (HS256 claims), `refresh` (opaque rotating tokens, SHA-256 hashed in DB), `extractor` (`AuthUser`, `require_role`)
- `src/routes/` handlers; routes registered in `routes/mod.rs` under `/api`
- `static/` client (`js/api.js` handles tokens, auto-refresh, redirects to login)
- `tests/api.rs` integration tests driving the router with `tower::ServiceExt::oneshot`

## Conventions
- New endpoint: add handler in `src/routes/`, register in `routes/mod.rs`, take `AuthUser` and call `user.require_role("...")` for protected routes, add a test, update `docs/api.md`.
- New schema change: add `migrations/NNNN_name.sql` (never edit applied migrations), update `docs/database.md`.
- Return `AppResult<T>`; use `AppError` variants, never leak internals (Internal logs detail, returns generic message).
- Emails are case-insensitive (`COLLATE NOCASE`); trim input. Never store or log plaintext passwords/tokens.
- Client: render user data with `textContent`, never `innerHTML`.
- Users are deactivated, never deleted. The last active admin cannot be deactivated or lose the `admin` role.
- Access-token roles come from the JWT, so role changes apply after the next refresh (<= access TTL).
- Never commit `.env`, `data/`, or `*.db` files.
- Keep docs in sync with code changes.
