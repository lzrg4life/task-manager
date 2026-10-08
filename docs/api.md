# API reference

All bodies are JSON; errors are `{"error": "message"}`. Protected routes need `Authorization: Bearer <access_token>`.

| Method & path | Auth | Description |
|---|---|---|
| POST `/api/auth/login` `{email,password}` | none | Returns `{access_token, refresh_token, token_type, expires_in, user}`. 401 on any failure |
| POST `/api/auth/refresh` `{refresh_token}` | none | Rotates refresh token; same response shape as login |
| POST `/api/auth/logout` `{refresh_token}` | none | Revokes the token family; 204 |
| GET `/api/me` | any user | Current user |
| GET `/api/users` | admin | List users |
| POST `/api/users` `{email,password,display_name?,roles[]}` | admin | Create user (201); 409 duplicate email; roles must exist and be non-empty |
| GET `/api/users/{id}` | admin | One user |
| PATCH `/api/users/{id}` `{email?,display_name?,password?,roles?,is_active?}` | admin | Update; 409 if it would remove the last active admin |
| GET `/api/roles` | admin | List roles |
| POST `/api/roles` `{name,description?}` | admin | Create role (name `[a-z0-9_-]{1,32}`) |

Passwords: 10-128 characters. Unknown `/api/*` paths return JSON 404.
