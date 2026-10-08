use axum::{
    Router,
    routing::{get, post},
};

use crate::{AppState, error::AppError};

pub mod auth;
pub mod roles;
pub mod users;

pub fn api_router() -> Router<AppState> {
    Router::new()
        .route("/auth/login", post(auth::login))
        .route("/auth/refresh", post(auth::refresh))
        .route("/auth/logout", post(auth::logout))
        .route("/me", get(auth::me))
        .route("/users", get(users::list).post(users::create))
        .route("/users/{id}", get(users::get_one).patch(users::update))
        .route("/roles", get(roles::list).post(roles::create))
        .fallback(|| async { AppError::NotFound })
}
