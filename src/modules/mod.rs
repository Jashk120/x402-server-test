//! Module registry — assembles all sub-routers into a single Axum Router.

pub mod agent;
pub mod did;
pub mod x402;

use axum::Router;
use crate::state::AppState;

/// Merge all module routers into one. Each module owns its own path namespace:
///   x402  → /health, /protected, /protected/:path
///   did   → /did/*
///   agent → /agent/*
pub fn all_routes() -> Router<AppState> {
    Router::new()
        .merge(x402::router())
        .merge(did::router())
        .merge(agent::router())
}
