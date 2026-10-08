# Architecture

- Single binary. `serve` builds an axum `Router`: `/api/*` JSON handlers, everything else falls back to `ServeDir(static/)`.
- State (`AppState`): `SqlitePool` + `Arc<Config>`.
- Migrations are embedded (`sqlx::migrate!`) and run on `init-db` and on `serve` start.

## Auth flow
1. `POST /api/auth/login` verifies the Argon2id hash and returns a JWT access token (default 15 min) and an opaque refresh token (default 30 days).
2. Client sends `Authorization: Bearer <jwt>`; `AuthUser` extractor validates signature/expiry and exposes id, email, roles.
3. On expiry, the client calls `POST /api/auth/refresh`; the refresh token is rotated (old one revoked, new one in the same family). Presenting an already-used token revokes the whole family.
4. Deactivating a user or resetting their password revokes all of their refresh tokens.

## Client token storage
Access token in `sessionStorage`, refresh token in `localStorage` (readable by JS, so XSS is the main risk; client avoids `innerHTML`). Multiple tabs refreshing simultaneously can trip reuse detection and force a re-login; a known tradeoff.
