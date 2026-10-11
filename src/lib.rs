use std::sync::Arc;

use axum::{Router, response::Redirect, routing::get};
use sqlx::SqlitePool;
use tower_http::{services::ServeDir, trace::TraceLayer};

pub mod auth;
pub mod config;
pub mod db;
pub mod error;
pub mod models;
pub mod routes;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub config: Arc<config::Config>,
}

pub fn app(state: AppState) -> Router {
    let static_dir = state.config.static_dir.clone();
    Router::new()
        .nest("/api", routes::api_router())
        .route(
            "/tickets.html",
            get(|| async { Redirect::temporary("/tasks.html") }),
        )
        .route(
            "/ticket.html",
            get(|| async { Redirect::temporary("/task.html") }),
        )
        .fallback_service(ServeDir::new(static_dir))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
