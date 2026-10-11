# API reference

All bodies are JSON; errors are `{"error": "message"}`. Protected routes need `Authorization: Bearer <access_token>`.

| Method & path | Auth | Description |
|---|---|---|
| POST `/api/auth/login` `{email,password}` | none | Returns `{access_token, refresh_token, token_type, expires_in, user}`. 401 on any failure |
| POST `/api/auth/refresh` `{refresh_token}` | none | Rotates refresh token; same response shape as login |
| POST `/api/auth/logout` `{refresh_token}` | none | Revokes the token family; 204 |
| GET `/api/me` | any user | Current user |
| GET `/api/users` | admin | List users |
| POST `/api/users` `{email,password,display_name?,is_admin?}` | admin | Create user (201); 409 duplicate email |
| GET `/api/users/{id}` | admin | One user |
| PATCH `/api/users/{id}` `{email?,display_name?,password?,is_admin?,is_active?}` | admin | Update; 409 if it would remove the last active admin |
| GET `/api/tags` | any user | List active tags (`id,name,description,archived`) |
| GET `/api/tags?include_archived=true` | any user | List all tags, including archived tags |
| POST `/api/tags` `{name,description?}` | admin | Create a tag (201); name is 1-40 characters and unique case-insensitively; description max 500 characters |
| POST `/api/tags/{id}/archive` | admin | Archive a tag; it remains on tickets already using it but cannot be newly assigned |

Passwords: 10-128 characters. Unknown `/api/*` paths return JSON 404.

## Tickets
All endpoints require any signed-in user. Statuses: `archived, completed, in_review, active, up_next, backlog, removed`. Priorities: `normal, high, low`. Ticket key is `TM-<id>`.

| Endpoint | Description |
|---|---|
| GET `/api/tickets` | Board: array of `{status, tickets[]}` in order completed, in_review, active, up_next, backlog; tickets ordered by id; archived/removed excluded |
| GET `/api/tickets/search?q=` | Search up to 20 non-archived tickets by key or title |
| GET `/api/tickets/archived?page=&per_page=&search=&assignee_id=&tag_ids=` | Paginated `{items,page,per_page,total}` (default 25/page, max 100); optional case-insensitive ID/title substring, reviewer/implementer user ID and comma-separated tag ID filters |
| GET `/api/tickets/removed?page=&per_page=&search=&assignee_id=&tag_ids=` | Same for removed tickets |
| POST `/api/tickets` `{title,details?,business_requirements?,technical_requirements?,implementation_notes?,status?,priority?,reviewer_id?,implementer_id?}` | Create (201); defaults backlog/normal; 400 on invalid title or inactive/unknown assignee |
| GET `/api/tickets/{id}` | Full ticket with creator, assignees, `tags[]`, and status/priority `history` |
| PATCH `/api/tickets/{id}` (any create field; `reviewer_id`/`implementer_id` accept `null` to clear) | Update; status/priority changes are recorded in history; 409 for any non-status change while archived |
| POST `/api/tickets/{id}/move` `{direction: "up"\|"down"}` | Move to adjacent board list; 400 at the ends or if not on the board |
| GET `/api/users/assignable` | Active users (`id,email,display_name`) for assignee pickers |
| POST `/api/tickets/{id}/links` `{url,label?}` | Link a webpage (201, returns the ticket). `url` must be http(s), max 2000 chars, no whitespace; `label` max 200; max 50 links per ticket. 409 if the URL is already linked or the ticket is archived |
| DELETE `/api/tickets/{id}/links/{link_id}` | Remove a link (returns the ticket); 404 if not on that ticket; 409 if archived |
| POST `/api/tickets/{id}/block` `{reason}` | Flag the ticket as blocked (returns the ticket); any signed-in user; `reason` trimmed, 1-500 chars (400 otherwise); 409 if already blocked or archived |
| POST `/api/tickets/{id}/unblock` | Clear the blocked flag (returns the ticket); any signed-in user; 409 if not blocked or archived |
| GET `/api/tickets/{id}/comments` | List comments in creation order, including deleted-comment markers |
| POST `/api/tickets/{id}/comments` `{body}` | Add a comment (201); trimmed body must be 1-5000 characters |
| PATCH `/api/tickets/{id}/comments/{comment_id}` `{body}` | Edit an undeleted comment; only its author or an admin may edit; 409 if already deleted |
| DELETE `/api/tickets/{id}/comments/{comment_id}` | Soft-delete a comment; only its author or an admin may delete; 204 |
| POST `/api/tickets/{id}/related` `{related_ticket_id}` | Relate two tickets symmetrically (201, returns the ticket); any signed-in user may do this; 400 for a self-link, 404 for an unknown ticket, 409 if already related or either ticket is archived |
| DELETE `/api/tickets/{id}/related/{related_id}` | Remove a ticket relation (returns the ticket); the relation is removed in both directions; 404 if not related; 409 if either ticket is archived |
| POST `/api/tickets/{id}/tags` `{tag_id}` | Associate an active tag (returns the ticket); any signed-in user may add tags, including on archived tickets; 409 if the tag is archived |
| DELETE `/api/tickets/{id}/tags/{tag_id}` | Remove a tag association (returns the ticket); any signed-in user may remove tags |

Tickets include `tags[]` (`id,name,description,archived`), `links[]` (`id,url,label,added_by,created_at`), and `related_tickets[]` (ticket summaries). Archived tags remain in ticket responses and are not removed from existing ticket associations. Comma-separated `tag_ids` in list queries filters with AND semantics: a ticket must have every selected tag. Link adds/removals are recorded in `history` with `field` = `link_added` (`new_value` = URL) or `link_removed` (`old_value` = URL, `new_value` null). Ticket summaries and tickets include `blocked` (bool) and `blocked_reason` (string or null). Blocking is independent of status: status changes (PATCH/move) are allowed while blocked and the flag persists until explicitly unblocked. Blocking/unblocking is recorded in `history` with `field` = `blocked` (`new_value` = reason) or `unblocked` (`old_value` = reason, `new_value` null).

Comments are visible to any signed-in user. Responses include `id,ticket_id,user_id,author_name,author_email,body,created_at,updated_at,deleted_at`; deleted comments remain in the thread with `body: null` and a `deleted_at` timestamp. Comment text is retained in storage but hidden from API responses after deletion. Deleted comments cannot be edited or deleted again.
