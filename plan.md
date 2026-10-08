# Task Manager – Rust Web API + Static Client (Foundation Plan)

(On approval, first copy this file to `c:\Users\wds85\repos\task_manager\plan.md`.)

## Goal
A Rust (axum) web API that also serves a static HTML/CSS/vanilla-JS client. SQLite persistence (sqlx). Multi-user, admin-managed accounts (no self sign-up), roles per user, short-lived JWT bearer auth plus rotating refresh tokens. Foundation for a larger project.

## Decisions
- Framework: axum + tokio, tower-http (`ServeDir`, trace, compression)
- DB: SQLite via sqlx (runtime queries, `sqlx::migrate!` migrations in `migrations/`)
- Password hashing: Argon2id (`argon2` crate, PHC string stored)
- Client: plain HTML/CSS/JS in `static/`, no build step
- Tokens: JWT access (HS256, ~15 min) via `Authorization: Bearer`; opaque random refresh token (~30 days), stored as SHA-256 hash in DB, rotated on every use, revocable; reuse of a rotated token revokes its family
- Roles: free-form `roles` table; `admin` seeded; user<->role many-to-many; user needs >= 1 role
- Users: admins create/edit/deactivate (no hard delete); deactivated users cannot log in or refresh
- First admin: `init-db` subcommand; `--admin-email`, password via interactive prompt, fallback env `ADMIN_PASSWORD`

## Project layout
```
task_manager/
  Cargo.toml  README.md  AGENTS.md  .env.example  .gitignore (target/, *.db*, .env)
  docs/{architecture,api,database,decisions}.md
  migrations/0001_init.sql
  src/
    main.rs        # clap: `serve` (default) | `init-db`
    config.rs      # DATABASE_URL, JWT_SECRET, BIND_ADDR, TTLs
    db.rs  error.rs  models.rs
    auth/{mod,password,jwt,refresh,extractor}.rs
    routes/{mod,auth,users,roles}.rs
  static/
    index.html  login.html  users.html
    css/app.css
    js/{api.js,login.js,users.js}   # api.js: fetch wrapper, token storage, one auto-refresh on 401
  tests/            # integration tests, in-memory SQLite
```

## Schema (0001_init.sql)
- `users`(id, email UNIQUE COLLATE NOCASE, password_hash, display_name, is_active, created_at, updated_at)
- `roles`(id, name UNIQUE, description); seed `admin`
- `user_roles`(user_id, role_id, composite PK, FKs)
- `refresh_tokens`(id, user_id, token_hash UNIQUE, family_id, expires_at, revoked_at, replaced_by, created_at)
- `PRAGMA foreign_keys=ON`, WAL mode

## API
- `POST /api/auth/login` -> access_token, refresh_token, expires_in, user
- `POST /api/auth/refresh` -> new access + rotated refresh
- `POST /api/auth/logout` -> revoke refresh token
- `GET /api/me` (authenticated)
- `GET/POST /api/users`, `GET/PATCH /api/users/{id}` (admin; PATCH covers email, name, roles, is_active, password reset)
- `GET/POST /api/roles` (admin)
- Static at `/`; unknown `/api/*` -> JSON 404
- Login uses generic errors and dummy-hash verify for unknown users (timing); cannot deactivate or strip admin from the last active admin

## Client
- `login.html`: form; access token in memory, refresh token in localStorage (XSS tradeoff documented); admins go to `users.html`
- `users.html`: user table, add form, edit dialog (name, email, roles, active, reset password); redirects if not authenticated/admin
- Render with `textContent` only

## Todos
1. git-init: `git init -b main`, `.gitignore`, initial commit (with Co-authored-by trailer)
2. scaffold: `cargo init`, deps (axum, tokio, tower-http, sqlx, serde, jsonwebtoken, argon2, rand, sha2, clap, rpassword, thiserror, tracing, time, uuid; dev: tower, http-body-util), config, `.env.example`
3. schema: migration + db module
4. init-db: CLI subcommand, runs migrations, creates admin; refuses if users already exist; validates email/password
5. auth-core: hashing, JWT, refresh rotate/revoke, `AuthUser` extractor, `require_role`
6. auth-routes: login/refresh/logout/me
7. user-routes: users + roles with admin guard and last-admin safeguard
8. static-serving: `ServeDir` fallback, router, graceful shutdown
9. client: pages, css, js
10. tests: login ok/fail, bad/expired JWT, refresh rotation + reuse detection, admin-only, deactivated user, last-admin guard, init-db re-run; `cargo fmt`, `clippy -D warnings`, `cargo test`
11. docs: README, AGENTS.md (commands, module map, conventions, how to add migration/route/role check, never commit `.env`/db, keep docs in sync), docs/*.md
12. verify: run init-db, start server, curl flows, load pages in browser, final commit

Dependency order: git-init -> scaffold -> schema -> (init-db, auth-core) -> auth-routes -> user-routes -> static-serving -> client -> tests -> docs -> verify.

## Notes
- `JWT_SECRET` required (>= 32 bytes), startup fails if missing; never log secrets/tokens
- Run: `cargo run -- init-db --admin-email you@example.com`, then `cargo run -- serve`
- Future: per-feature permissions, rate limiting, HTTPS via reverse proxy
