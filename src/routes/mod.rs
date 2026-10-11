use axum::{
    Router,
    routing::{delete, get, post},
};

use crate::{AppState, error::AppError};

pub mod auth;
pub mod comments;
pub mod tags;
pub mod tasks;
pub mod users;

pub fn api_router() -> Router<AppState> {
    Router::new()
        .route("/auth/login", post(auth::login))
        .route("/auth/refresh", post(auth::refresh))
        .route("/auth/logout", post(auth::logout))
        .route("/me", get(auth::me))
        .route("/users", get(users::list).post(users::create))
        .route("/users/{id}", get(users::get_one).patch(users::update))
        .route("/users/assignable", get(tasks::assignable_users))
        .route("/tags", get(tags::list).post(tags::create))
        .route("/tags/{id}/archive", post(tags::archive))
        .route("/tasks", get(tasks::list).post(tasks::create))
        .route("/tasks/search", get(tasks::search))
        .route("/tasks/archived", get(tasks::list_archived))
        .route("/tasks/removed", get(tasks::list_removed))
        .route("/tasks/{id}", get(tasks::get_one).patch(tasks::update))
        .route(
            "/tasks/{id}/edit-lock",
            post(tasks::acquire_edit_lock).delete(tasks::release_edit_lock),
        )
        .route("/tasks/{id}/move", post(tasks::move_task))
        .route(
            "/tasks/{id}/comments",
            get(comments::list).post(comments::create),
        )
        .route(
            "/tasks/{id}/comments/{comment_id}",
            axum::routing::patch(comments::update).delete(comments::delete),
        )
        .route("/tasks/{id}/block", post(tasks::block))
        .route("/tasks/{id}/unblock", post(tasks::unblock))
        .route("/tasks/{id}/links", post(tasks::add_link))
        .route("/tasks/{id}/links/{link_id}", delete(tasks::remove_link))
        .route("/tasks/{id}/related", post(tasks::add_related_task))
        .route(
            "/tasks/{id}/related/{related_id}",
            delete(tasks::remove_related_task),
        )
        .route("/tasks/{id}/tags", post(tasks::add_tag))
        .route("/tasks/{id}/tags/{tag_id}", delete(tasks::remove_tag))
        .fallback(|| async { AppError::NotFound })
}
