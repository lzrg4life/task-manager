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
| POST `/api/tags/{id}/archive` | admin | Archive a tag; it remains on tasks already using it but cannot be newly assigned |

Passwords: 10-128 characters. Unknown `/api/*` paths return JSON 404.

## Tasks
All endpoints require any signed-in user. Statuses: `archived, completed, in_review, active, up_next, backlog, removed`. Priorities: `normal, high, low`. Task key is `TM-<id>`.

| Endpoint | Description |
|---|---|
| GET `/api/tasks` | Board: array of `{status, tasks[]}` in order completed, in_review, active, up_next, backlog; tasks ordered by id; archived/removed excluded |
| GET `/api/tasks/search?q=` | Search up to 20 non-archived tasks by key or title |
| GET `/api/tasks/archived?page=&per_page=&search=&assignee_id=&tag_ids=` | Paginated `{items,page,per_page,total}` (default 25/page, max 100); optional case-insensitive ID/title substring, reviewer/implementer user ID and comma-separated tag ID filters |
| GET `/api/tasks/removed?page=&per_page=&search=&assignee_id=&tag_ids=` | Same for removed tasks |
| POST `/api/tasks` `{title,expected_by_date?,details?,business_requirements?,technical_requirements?,implementation_notes?,status?,priority?,reviewer_id?,implementer_id?}` | Create (201); defaults backlog/normal; optional `expected_by_date` is `YYYY-MM-DD` or `null`; 400 on an invalid date, title, or inactive/unknown assignee |
| GET `/api/tasks/{id}` | Full task with creator, assignees, optional `expected_by_date`, `tags[]`, and status/priority `history` |
| POST `/api/tasks/{id}/edit-lock` | Acquire the task edit lock; 409 if another user holds it or the task is archived. Re-acquiring your own lock is allowed |
| DELETE `/api/tasks/{id}/edit-lock` | Release your own edit lock; admins may clear any user's lock |
| PATCH `/api/tasks/{id}` (any create field; `expected_by_date`, `reviewer_id`, and `implementer_id` accept `null` to clear) | Save only while holding the edit lock; a successful save releases it. Status-only archive and restore operations remain available when the task is unlocked. Status/priority changes are recorded in history; 409 for any non-status change while archived |
| POST `/api/tasks/{id}/move` `{direction: "up"\|"down"}` | Move to adjacent board list; 400 at the ends or if not on the board; 409 while another user holds the edit lock |
| GET `/api/users/assignable` | Active users (`id,email,display_name`) for assignee pickers |
| POST `/api/tasks/{id}/links` `{url,label?}` | Link a webpage (201, returns the task). `url` must be http(s), max 2000 chars, no whitespace; `label` max 200; max 50 links per task. 409 if the URL is already linked or the task is archived |
| DELETE `/api/tasks/{id}/links/{link_id}` | Remove a link (returns the task); 404 if not on that task; 409 if archived |
| POST `/api/tasks/{id}/block` `{reason}` | Flag the task as blocked (returns the task); any signed-in user; `reason` trimmed, 1-500 chars (400 otherwise); 409 if already blocked or archived |
| POST `/api/tasks/{id}/unblock` | Clear the blocked flag (returns the task); any signed-in user; 409 if not blocked or archived |
| GET `/api/tasks/{id}/comments` | List comments in creation order, including deleted-comment markers |
| POST `/api/tasks/{id}/comments` `{body}` | Add a comment (201); trimmed body must be 1-5000 characters |
| PATCH `/api/tasks/{id}/comments/{comment_id}` `{body}` | Edit an undeleted comment; only its author or an admin may edit; 409 if already deleted |
| DELETE `/api/tasks/{id}/comments/{comment_id}` | Soft-delete a comment; only its author or an admin may delete; 204 |
| POST `/api/tasks/{id}/related` `{related_task_id}` | Relate two tasks symmetrically (201, returns the task); any signed-in user may do this; 400 for a self-link, 404 for an unknown task, 409 if already related or either task is archived |
| DELETE `/api/tasks/{id}/related/{related_id}` | Remove a task relation (returns the task); the relation is removed in both directions; 404 if not related; 409 if either task is archived |
| POST `/api/tasks/{id}/tags` `{tag_id}` | Associate an active tag (returns the task); any signed-in user may add tags, including on archived tasks; 409 if the tag is archived |
| DELETE `/api/tasks/{id}/tags/{tag_id}` | Remove a tag association (returns the task); any signed-in user may remove tags |

Tasks and task summaries include `expected_by_date` (a nullable calendar date in `YYYY-MM-DD` format). The field is optional at creation and can be cleared by sending `null` in a PATCH. Detailed task responses also include `editing_user` (a user reference or `null`). Edit locks persist until the holder saves/releases them or an admin clears them. Tasks include `tags[]` (`id,name,description,archived`), `links[]` (`id,url,label,added_by,created_at`), and `related_tasks[]` (task summaries). Archived tags remain in task responses and are not removed from existing task associations. Comma-separated `tag_ids` in list queries filters with AND semantics: a task must have every selected tag. Link adds/removals are recorded in `history` with `field` = `link_added` (`new_value` = URL) or `link_removed` (`old_value` = URL, `new_value` null). Task summaries and tasks include `blocked` (bool) and `blocked_reason` (string or null). Blocking is independent of status: status changes (PATCH/move) are allowed while blocked and the flag persists until explicitly unblocked. Blocking/unblocking is recorded in `history` with `field` = `blocked` (`new_value` = reason) or `unblocked` (`old_value` = reason, `new_value` null).

Comments are visible to any signed-in user. Responses include `id,task_id,user_id,author_name,author_email,body,created_at,updated_at,deleted_at`; deleted comments remain in the thread with `body: null` and a `deleted_at` timestamp. Comment text is retained in storage but hidden from API responses after deletion. Deleted comments cannot be edited or deleted again.
