use axum::{
    Router,
    routing::{delete, get, post},
};

use crate::{AppState, error::AppError};

pub mod auth;
pub mod roles;
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
        .route("/roles", get(roles::list).post(roles::create))
        .fallback(|| async { AppError::NotFound })
}
