use std::sync::Arc;

use axum::Router;
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
        .fallback_service(ServeDir::new(static_dir))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
