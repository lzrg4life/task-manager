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

## Tickets
All endpoints require any signed-in user. Statuses: `archived, completed, in_review, active, up_next, backlog, removed`. Priorities: `normal, high, low`. Ticket key is `TM-<id>`.

| Endpoint | Description |
|---|---|
| GET `/api/tickets` | Board: array of `{status, tickets[]}` in order completed, in_review, active, up_next, backlog; tickets ordered by id; archived/removed excluded |
| GET `/api/tickets/archived?page=&per_page=` | Paginated `{items,page,per_page,total}` (default 25/page, max 100) |
| GET `/api/tickets/removed?page=&per_page=` | Same for removed tickets |
| POST `/api/tickets` `{title,details?,business_requirements?,technical_requirements?,implementation_notes?,status?,priority?,reviewer_id?,implementer_id?}` | Create (201); defaults backlog/normal; 400 on invalid title or inactive/unknown assignee |
| GET `/api/tickets/{id}` | Full ticket with creator, assignees and status/priority `history` |
| PATCH `/api/tickets/{id}` (any create field; `reviewer_id`/`implementer_id` accept `null` to clear) | Update; status/priority changes are recorded in history; 409 for any non-status change while archived |
| POST `/api/tickets/{id}/move` `{direction: "up"\|"down"}` | Move to adjacent board list; 400 at the ends or if not on the board |
| GET `/api/users/assignable` | Active users (`id,email,display_name`) for assignee pickers |
| POST `/api/tickets/{id}/links` `{url,label?}` | Link a webpage (201, returns the ticket). `url` must be http(s), max 2000 chars, no whitespace; `label` max 200; max 50 links per ticket. 409 if the URL is already linked or the ticket is archived |
| DELETE `/api/tickets/{id}/links/{link_id}` | Remove a link (returns the ticket); 404 if not on that ticket; 409 if archived |

Tickets include `links[]` (`id,url,label,added_by,created_at`). Link adds/removals are recorded in `history` with `field` = `link_added` (`new_value` = URL) or `link_removed` (`old_value` = URL, `new_value` null).

