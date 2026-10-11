use axum::{
    Router,
    routing::{delete, get, post},
};

use crate::{AppState, error::AppError};

pub mod auth;
pub mod tags;
pub mod tickets;
pub mod users;

pub fn api_router() -> Router<AppState> {
    Router::new()
        .route("/auth/login", post(auth::login))
        .route("/auth/refresh", post(auth::refresh))
        .route("/auth/logout", post(auth::logout))
        .route("/me", get(auth::me))
        .route("/users", get(users::list).post(users::create))
        .route("/users/{id}", get(users::get_one).patch(users::update))
        .route("/users/assignable", get(tickets::assignable_users))
        .route("/tags", get(tags::list).post(tags::create))
        .route("/tags/{id}/archive", post(tags::archive))
        .route("/tickets", get(tickets::list).post(tickets::create))
        .route("/tickets/archived", get(tickets::list_archived))
        .route("/tickets/removed", get(tickets::list_removed))
        .route(
            "/tickets/{id}",
            get(tickets::get_one).patch(tickets::update),
        )
        .route("/tickets/{id}/move", post(tickets::move_ticket))
        .route("/tickets/{id}/links", post(tickets::add_link))
        .route(
            "/tickets/{id}/links/{link_id}",
            delete(tickets::remove_link),
        )
        .route("/tickets/{id}/tags", post(tickets::add_tag))
        .route("/tickets/{id}/tags/{tag_id}", delete(tickets::remove_tag))
        .fallback(|| async { AppError::NotFound })
}
