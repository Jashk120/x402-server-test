//! DID registry module — STUBBED.
//!
//! Planned routes
//! ──────────────
//!   POST /did/register   — anchor a new DID document on Hedera
//!   GET  /did/resolve/:did — resolve a DID to its document
//!   PUT  /did/update/:did  — update a DID document (requires auth)
//!   DELETE /did/deactivate/:did — deactivate a DID
//!
//! This stub returns 501 Not Implemented on all routes so the router
//! compiles and the module boundary is established for future work.

use axum::{Router, routing::{get, post, put, delete}};
use axum::response::{IntoResponse, Json};
use axum::http::StatusCode;
use serde_json::json;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/did/register",         post(stub))
        .route("/did/resolve/:did",      get(stub))
        .route("/did/update/:did",       put(stub))
        .route("/did/deactivate/:did",   delete(stub))
}

async fn stub() -> impl IntoResponse {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(json!({
            "error":  "not_implemented",
            "module": "did-registry",
            "message": "DID registry is planned but not yet implemented. \
                        See /home/curator/aria-v2/server/src/modules/did/mod.rs"
        })),
    )
}
