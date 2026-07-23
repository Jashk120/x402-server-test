//! x402 gateway route handlers.
//!
//! Routes
//! ──────
//!   GET  /protected          — gated resource; demands a PAYMENT-SIGNATURE
//!   GET  /protected/:path    — same, arbitrary sub-paths for future use
//!   GET  /health             — liveness probe (no payment required)
//!
//! Flow
//! ────
//!  1. No PAYMENT-SIGNATURE header → 402 + requirements JSON
//!  2. Token present → base64-decode → JSON-decode PaymentPayload
//!  3. POST payload to facilitator /verify
//!  4. Verified → 200 with resource content

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json, Response},
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde_json::json;
use tracing::{info, warn};

use crate::state::AppState;
use super::types::{PaymentPayload, PaymentRequiredBody, PaymentRequirements, VerifyResponse};

// ── Health ────────────────────────────────────────────────────────────────────

pub async fn health() -> impl IntoResponse {
    Json(json!({ "status": "ok", "node": "aria-node" }))
}

// ── Gated resource (root) ─────────────────────────────────────────────────────

pub async fn protected_root(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Response {
    protected_inner(state, headers, "index".to_string()).await
}

// ── Gated resource (sub-path) ─────────────────────────────────────────────────

pub async fn protected_path(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(path): Path<String>,
) -> Response {
    protected_inner(state, headers, path).await
}

// ── Core logic ────────────────────────────────────────────────────────────────

async fn protected_inner(state: AppState, headers: HeaderMap, resource: String) -> Response {
    // 1. Check for PAYMENT-SIGNATURE header
    let token = headers
        .get("PAYMENT-SIGNATURE")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    match token {
        None => payment_required(&state, &resource),
        Some(token) => attempt_access(&state, &token, &resource).await,
    }
}

// ── 402 response ──────────────────────────────────────────────────────────────

fn payment_required(state: &AppState, resource: &str) -> Response {
    let cfg = &state.config;
    let requirements = PaymentRequirements {
        scheme:              "exact".to_string(),
        network:             cfg.x402_network.clone(),
        amount:              cfg.x402_amount.clone(),
        asset:               cfg.x402_asset.clone(),
        pay_to:              cfg.x402_pay_to.clone(),
        max_timeout_seconds: 60,
        extra:               serde_json::Value::Null,
    };

    let body = PaymentRequiredBody {
        x402_version: 2,
        accepts:      vec![requirements],
        error:        "Payment required",
    };

    info!("→ 402 for resource '{}'", resource);

    // x402 spec: respond with HTTP 402 + JSON body
    (StatusCode::PAYMENT_REQUIRED, Json(body)).into_response()
}

// ── Token verification + resource delivery ────────────────────────────────────

async fn attempt_access(state: &AppState, token: &str, resource: &str) -> Response {
    // Decode base64 → JSON
    let decoded = match STANDARD.decode(token) {
        Ok(b) => b,
        Err(e) => {
            warn!("PAYMENT-SIGNATURE base64 decode failed: {}", e);
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Invalid PAYMENT-SIGNATURE encoding" })),
            ).into_response();
        }
    };

    let payload: PaymentPayload = match serde_json::from_slice(&decoded) {
        Ok(p) => p,
        Err(e) => {
            warn!("PAYMENT-SIGNATURE JSON decode failed: {}", e);
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Invalid PAYMENT-SIGNATURE structure" })),
            ).into_response();
        }
    };

    // Call facilitator /verify
    let cfg = &state.config;
    let requirements = PaymentRequirements {
        scheme:              "exact".to_string(),
        network:             cfg.x402_network.clone(),
        amount:              cfg.x402_amount.clone(),
        asset:               cfg.x402_asset.clone(),
        pay_to:              cfg.x402_pay_to.clone(),
        max_timeout_seconds: 60,
        extra:               serde_json::Value::Null,
    };

    let verify_url = format!("{}/verify", cfg.facilitator_url.trim_end_matches('/'));
    let verify_body = json!({
        "paymentPayload": payload,
        "paymentRequirements": requirements,
    });

    let verify_res = state.http
        .post(&verify_url)
        .json(&verify_body)
        .send()
        .await;

    let resp = match verify_res {
        Ok(r) => r,
        Err(e) => {
            warn!("Facilitator /verify request failed: {}", e);
            return (
                StatusCode::BAD_GATEWAY,
                Json(json!({ "error": "Could not reach payment facilitator" })),
            ).into_response();
        }
    };

    let verify: VerifyResponse = match resp.json().await {
        Ok(v) => v,
        Err(e) => {
            warn!("Facilitator /verify response decode failed: {}", e);
            return (
                StatusCode::BAD_GATEWAY,
                Json(json!({ "error": "Invalid facilitator response" })),
            ).into_response();
        }
    };

    if !verify.is_valid {
        let reason = verify.invalid_reason.as_deref().unwrap_or("unknown");
        warn!("Payment verification failed: {}", reason);
        return (
            StatusCode::PAYMENT_REQUIRED,
            Json(json!({ "error": "Payment verification failed", "reason": reason })),
        ).into_response();
    }

    // Payment verified — deliver the resource
    let payer = verify.payer.as_deref().unwrap_or("unknown");
    info!("✓ Payment verified from {} — delivering '{}'", payer, resource);

    (StatusCode::OK, Json(json!({
        "status":   "PAID",
        "resource": resource,
        "payer":    payer,
        "message":  format!("Access granted to '{}'. Payment confirmed on {}.", resource, cfg.x402_network),
        "network":  cfg.x402_network,
        // TODO: replace with real resource content per route
        "data": {
            "content": format!("This is the protected content for '{}'.", resource),
            "unlocked_at": chrono_now(),
        }
    }))).into_response()
}

fn chrono_now() -> String {
    // Simple ISO-8601 without pulling in chrono — good enough for stub
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| format!("unix:{}", d.as_secs()))
        .unwrap_or_else(|_| "unknown".to_string())
}
