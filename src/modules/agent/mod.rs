//! Agent-to-agent protocol module — STUBBED.
//!
//! Planned routes
//! ──────────────
//!   POST /agent/message       — receive a signed A2A message from another agent
//!   GET  /agent/capabilities  — advertise this node's capability manifest
//!   GET  /agent/info          — node DID + public key + supported protocols
//!   WS   /agent/stream        — bidirectional agent message stream (future)
//!
//! Auth model (planned): DIDComm-style — sender signs message envelope with
//! their DID key; recipient verifies via DID resolution before processing.

use axum::{Router, routing::{get, post}};
use axum::response::{IntoResponse, Json};
use axum::http::StatusCode;
use serde_json::json;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/agent/message",      post(stub))
        .route("/agent/capabilities", get(capabilities))
        .route("/agent/info",         get(info))
}

async fn stub() -> impl IntoResponse {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(json!({
            "error":  "not_implemented",
            "module": "agent-protocol",
            "message": "Agent-to-agent protocol is planned but not yet implemented."
        })),
    )
}

/// Returns the node's capability manifest so other agents know what this
/// node supports. This is intentionally live even in stub mode — it's
/// how agents discover what to negotiate.
async fn capabilities() -> impl IntoResponse {
    Json(json!({
        "node": "aria-node",
        "version": "0.1.0",
        "protocols": ["x402/v2"],
        "capabilities": {
            "x402_gateway": true,
            "did_registry":  false,   // stub
            "a2a_messaging": false,   // stub
        },
        "status": "partial — some modules are stubs pending implementation"
    }))
}

async fn info() -> impl IntoResponse {
    Json(json!({
        "node": "aria-node",
        "version": "0.1.0",
        "did": null,   // populated after DID registry is implemented
        "endpoints": {
            "x402":  "/protected",
            "did":   "/did",
            "agent": "/agent"
        }
    }))
}
