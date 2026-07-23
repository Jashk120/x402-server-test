//! x402 gateway module — registers all payment-related routes.
//!
//! Router
//! ──────
//!   GET  /health
//!   GET  /protected
//!   GET  /protected/:path

pub mod routes;
pub mod types;

use axum::{Router, routing::get};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health",            get(routes::health))
        .route("/protected",         get(routes::protected_root))
        .route("/protected/:path",   get(routes::protected_path))
}
