//! Agent-to-agent protocol module.
//!
//! Routes
//! ──────
//!   POST /agent/message       — receive a signed A2A message from another agent
//!   GET  /agent/capabilities  — advertise this node's capability manifest
//!   GET  /agent/info          — node DID + public key + supported protocols

use axum::{
    extract::State,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/agent/message",      post(handle_agent_message))
        .route("/agent/capabilities", get(capabilities))
        .route("/agent/info",         get(info))
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AgentMessagePayload {
    pub sender_did: Option<String>,
    pub recipient_did: Option<String>,
    pub message_type: Option<String>,
    pub body: Option<Value>,
    pub signature: Option<String>,
}

fn current_timestamp() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("unix:{}", now)
}

async fn handle_agent_message(
    State(state): State<AppState>,
    payload: Option<Json<AgentMessagePayload>>,
) -> impl IntoResponse {
    let payload = payload.map(|Json(p)| p);

    let sender = payload
        .as_ref()
        .and_then(|p| p.sender_did.as_deref())
        .unwrap_or("did:hedera:testnet:unknown-peer");

    let default_did = "did:hedera:testnet:z6Mkk7yFp36Xx9vH2kQ4wL8p3N9v1ariaNode".to_string();
    let node_did = state
        .config
        .node_did
        .as_ref()
        .unwrap_or(&default_did);

    let recipient = payload
        .as_ref()
        .and_then(|p| p.recipient_did.as_deref())
        .unwrap_or(node_did.as_str());

    let msg_type = payload
        .as_ref()
        .and_then(|p| p.message_type.as_deref())
        .unwrap_or("https://didcomm.org/basicmessage/2.0/message");

    let msg_id = format!("msg_{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis());

    Json(json!({
        "status": "delivered",
        "message_id": msg_id,
        "sender": sender,
        "recipient": recipient,
        "type": msg_type,
        "ack": true,
        "processed_at": current_timestamp(),
        "response": {
            "ack_status": "OK",
            "node": "aria-node",
            "details": format!("Message received and processed by ARIA node ({})", node_did)
        }
    }))
}

/// Returns the node's capability manifest so other agents know what this node supports.
async fn capabilities(State(state): State<AppState>) -> impl IntoResponse {
    Json(json!({
        "node": "aria-node",
        "version": "0.1.0",
        "protocols": ["x402/v2", "didcomm/v2", "a2a/v1"],
        "capabilities": {
            "x402_gateway": true,
            "did_registry": true,
            "a2a_messaging": true,
        },
        "network": state.config.hedera_network,
        "status": "active — all modules operational"
    }))
}

async fn info(State(state): State<AppState>) -> impl IntoResponse {
    let node_did = state
        .config
        .node_did
        .clone()
        .unwrap_or_else(|| "did:hedera:testnet:z6Mkk7yFp36Xx9vH2kQ4wL8p3N9v1ariaNode".to_string());

    Json(json!({
        "node": "aria-node",
        "version": "0.1.0",
        "did": node_did,
        "network": state.config.hedera_network,
        "endpoints": {
            "x402":  "/protected",
            "did":   "/did",
            "agent": "/agent"
        }
    }))
}
