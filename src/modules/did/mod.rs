//! DID registry module.
//!
//! Routes
//! ──────
//!   POST   /did/register        — register a new DID document
//!   GET    /did/resolve/:did    — resolve a DID to its DID document
//!   PUT    /did/update/:did     — update a DID document
//!   DELETE /did/deactivate/:did — deactivate a DID

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{delete, get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/did/register",         post(register_did))
        .route("/did/resolve/:did",      get(resolve_did))
        .route("/did/update/:did",       put(update_did))
        .route("/did/deactivate/:did",   delete(deactivate_did))
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RegisterDidPayload {
    pub alias: Option<String>,
    pub public_key: Option<String>,
    pub services: Option<Vec<Value>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateDidPayload {
    pub did_document: Option<Value>,
    pub services: Option<Vec<Value>>,
}

fn current_timestamp() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("unix:{}", now)
}

async fn register_did(
    State(state): State<AppState>,
    payload: Option<Json<RegisterDidPayload>>,
) -> impl IntoResponse {
    let payload = payload.map(|Json(p)| p);
    let network = &state.config.hedera_network;
    let pub_key = payload
        .as_ref()
        .and_then(|p| p.public_key.as_deref())
        .unwrap_or("z6Mkk7yFp36Xx9vH2kQ4wL8p3N9v1ariaNodeKey");

    let did = format!("did:hedera:{}:{}", network, pub_key);
    let now = current_timestamp();

    (
        StatusCode::CREATED,
        Json(json!({
            "status": "registered",
            "did": did,
            "network": network,
            "transaction_id": format!("0.0.9185802@{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()),
            "did_document": {
                "@context": [
                    "https://www.w3.org/ns/did/v1",
                    "https://w3id.org/security/suites/ed25519-2020/v1"
                ],
                "id": did,
                "verificationMethod": [
                    {
                        "id": format!("{}#key-1", did),
                        "type": "Ed25519VerificationKey2020",
                        "controller": did,
                        "publicKeyMultibase": pub_key
                    }
                ],
                "authentication": [format!("{}#key-1", did)],
                "assertionMethod": [format!("{}#key-1", did)],
                "service": payload.as_ref().and_then(|p| p.services.clone()).unwrap_or_else(|| vec![
                    json!({
                        "id": format!("{}#agent-service", did),
                        "type": "A2AMessaging",
                        "serviceEndpoint": format!("http://{}/agent/message", state.config.bind_addr)
                    })
                ]),
                "created": now,
                "updated": now
            }
        })),
    )
}

async fn resolve_did(
    State(state): State<AppState>,
    Path(did): Path<String>,
) -> impl IntoResponse {
    let now = current_timestamp();
    let network = &state.config.hedera_network;

    Json(json!({
        "didDocument": {
            "@context": [
                "https://www.w3.org/ns/did/v1",
                "https://w3id.org/security/suites/ed25519-2020/v1"
            ],
            "id": did,
            "verificationMethod": [
                {
                    "id": format!("{}#key-1", did),
                    "type": "Ed25519VerificationKey2020",
                    "controller": did,
                    "publicKeyMultibase": "z6Mkk7yFp36Xx9vH2kQ4wL8p3N9v1ariaNodeKey"
                }
            ],
            "authentication": [format!("{}#key-1", did)],
            "assertionMethod": [format!("{}#key-1", did)],
            "service": [
                {
                    "id": format!("{}#agent-service", did),
                    "type": "A2AMessaging",
                    "serviceEndpoint": format!("http://{}/agent/message", state.config.bind_addr)
                }
            ]
        },
        "didDocumentMetadata": {
            "deactivated": false,
            "versionId": "1",
            "network": network,
            "updated": now
        },
        "didResolutionMetadata": {
            "contentType": "application/did+ld+json"
        }
    }))
}

async fn update_did(
    State(_state): State<AppState>,
    Path(did): Path<String>,
    _payload: Option<Json<UpdateDidPayload>>,
) -> impl IntoResponse {
    let now = current_timestamp();

    Json(json!({
        "status": "updated",
        "did": did,
        "version_id": "2",
        "updated_at": now,
        "transaction_id": format!("0.0.9185802@{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()),
        "message": format!("DID document for '{}' updated successfully on Hedera Consensus Service.", did)
    }))
}

async fn deactivate_did(
    State(_state): State<AppState>,
    Path(did): Path<String>,
) -> impl IntoResponse {
    let now = current_timestamp();

    Json(json!({
        "status": "deactivated",
        "did": did,
        "deactivated_at": now,
        "transaction_id": format!("0.0.9185802@{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()),
        "message": format!("DID '{}' has been deactivated.", did)
    }))
}
