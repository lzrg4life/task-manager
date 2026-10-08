# Task Manager

Rust (axum + SQLite) web API that also serves a static HTML/CSS/JS client. Accounts are admin-managed (no self sign-up), users have one or more roles, and auth uses short-lived JWTs plus rotating refresh tokens.

## Quick start
```powershell
copy .env.example .env            # then set JWT_SECRET (>= 32 chars)
cargo run -- init-db --admin-email you@example.com   # prompts for password; creates DB + first admin
cargo run -- serve                # or just `cargo run`; http://127.0.0.1:3000/login.html
```
On Windows, `run-dev.bat [admin-email]` does all of the above (creates `.env`, initializes the DB on first run, starts the server). `clean-dev.bat [/y] [/all]` undoes it (removes `.env` and `data\`; `/all` also runs `cargo clean`).

`init-db` reads the password from a prompt, or from `ADMIN_PASSWORD` when no terminal is available. It refuses to run if users already exist.

## Commands
- `cargo test` – integration tests (in-memory SQLite)
- `cargo clippy --all-targets -- -D warnings` and `cargo fmt`

## Configuration (env vars or `.env`)
`DATABASE_URL`, `JWT_SECRET` (required for serve), `BIND_ADDR`, `ACCESS_TOKEN_TTL_SECS` (900), `REFRESH_TOKEN_TTL_SECS` (30 days), `STATIC_DIR` (static).

## Docs
[AGENTS.md](AGENTS.md) (start here for AI agents) · [docs/architecture.md](docs/architecture.md) · [docs/api.md](docs/api.md) · [docs/database.md](docs/database.md) · [docs/decisions.md](docs/decisions.md)
