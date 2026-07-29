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
use super::types::{
    PaymentPayload, PaymentRequiredBody, PaymentRequirements, SettleResponse,
    SupportedKindsResponse, VerifyResponse,
};

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
        None => payment_required(&state, &resource).await,
        Some(token) => attempt_access(&state, &token, &resource).await,
    }
}

// ── Shared requirements builder ───────────────────────────────────────────────
//
// The client (daemon) fetches /supported before building its transaction and
// embeds the facilitator's Hedera feePayer into `extra` — because Hedera's
// x402 scheme uses a partially-signed-transaction model where the facilitator
// itself submits and pays gas, so the signed transaction is built against a
// SPECIFIC feePayer account. If the PaymentRequirements we hand back to the
// facilitator during /verify and /settle don't carry that same feePayer in
// `extra`, the facilitator has no way to know which account the transaction
// was built against — this was silently causing malformed/non-JSON responses
// from the facilitator (the earlier "response decode failed" 502s).
//
// This must exactly match what the client used, so we fetch /supported the
// same way it does and build `extra` identically for the 402 challenge, and
// again for /verify + /settle (kept as a single fresh fetch per request
// rather than cached, since it's a cheap GET and avoids any staleness bugs
// if the facilitator's fee payer ever rotates).
async fn build_requirements(state: &AppState) -> Result<PaymentRequirements, Response> {
    let cfg = &state.config;
    let supported_url = format!("{}/supported", cfg.facilitator_url.trim_end_matches('/'));

    let resp = state.http.get(&supported_url).send().await.map_err(|e| {
        warn!("Facilitator /supported request failed: {}", e);
        (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": "Could not reach payment facilitator" })),
        ).into_response()
    })?;

    let supported: SupportedKindsResponse = resp.json().await.map_err(|e| {
        warn!("Facilitator /supported response decode failed: {}", e);
        (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": "Invalid facilitator /supported response" })),
        ).into_response()
    })?;

    let fee_payer = supported
        .kinds
        .iter()
        .find(|k| k.network == cfg.x402_network)
        .and_then(|k| k.extra.as_ref())
        .and_then(|extra| extra.get("feePayer"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let extra = match fee_payer {
        Some(fp) => {
            let mut obj = serde_json::Map::new();
            obj.insert("feePayer".to_string(), serde_json::Value::String(fp));
            serde_json::Value::Object(obj)
        }
        None => {
            warn!(
                "Facilitator /supported has no feePayer for network '{}' — proceeding with extra: null",
                cfg.x402_network
            );
            serde_json::Value::Null
        }
    };

    Ok(PaymentRequirements {
        scheme:              "exact".to_string(),
        network:             cfg.x402_network.clone(),
        amount:              cfg.x402_amount.clone(),
        asset:               cfg.x402_asset.clone(),
        pay_to:              cfg.x402_pay_to.clone(),
        max_timeout_seconds: 60,
        extra,
    })
}

// ── 402 response ──────────────────────────────────────────────────────────────

async fn payment_required(state: &AppState, resource: &str) -> Response {
    let requirements = match build_requirements(state).await {
        Ok(r) => r,
        Err(resp) => return resp,
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
    let requirements = match build_requirements(state).await {
        Ok(r) => r,
        Err(resp) => return resp,
    };

    let verify_url = format!("{}/verify", cfg.facilitator_url.trim_end_matches('/'));
    // IMPORTANT: verify against OUR OWN requirements, not the client's
    // self-reported `payload.accepted`. Trusting the client's claim here
    // means a client could assert any network/amount/payTo it likes and
    // the facilitator would be asked to validate against that instead of
    // what this server actually demands.
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

    // /verify only confirms the payload is well-formed and signed — it does
    // NOT submit anything on-chain. The facilitator only actually broadcasts
    // and pays gas for the transaction during /settle. Serving content after
    // /verify alone means content can be granted for payments that never
    // actually land on Hedera. Call /settle before delivering the resource.
    let settle_url = format!("{}/settle", cfg.facilitator_url.trim_end_matches('/'));
    let settle_body = json!({
        "paymentPayload": payload,
        "paymentRequirements": requirements,
    });

    let settle_res = state.http
        .post(&settle_url)
        .json(&settle_body)
        .send()
        .await;

    let settle_resp = match settle_res {
        Ok(r) => r,
        Err(e) => {
            warn!("Facilitator /settle request failed: {}", e);
            return (
                StatusCode::BAD_GATEWAY,
                Json(json!({ "error": "Could not reach payment facilitator for settlement" })),
            ).into_response();
        }
    };

    let settle: SettleResponse = match settle_resp.json().await {
        Ok(s) => s,
        Err(e) => {
            warn!("Facilitator /settle response decode failed: {}", e);
            return (
                StatusCode::BAD_GATEWAY,
                Json(json!({ "error": "Invalid facilitator settlement response" })),
            ).into_response();
        }
    };

    if !settle.success {
        let reason = settle.error_reason.as_deref().unwrap_or("unknown");
        warn!("Payment settlement failed: {}", reason);
        return (
            StatusCode::PAYMENT_REQUIRED,
            Json(json!({ "error": "Payment settlement failed", "reason": reason })),
        ).into_response();
    }

    // Payment verified AND settled — deliver the resource
    let payer = settle.payer.as_deref().or(verify.payer.as_deref()).unwrap_or("unknown");
    let tx = settle.transaction.as_deref().unwrap_or("unknown");
    info!("✓ Payment verified + settled from {} (tx: {}) — delivering '{}'", payer, tx, resource);

    (StatusCode::OK, Json(json!({
        "status":      "PAID",
        "resource":    resource,
        "payer":       payer,
        "transaction": tx,
        "message":     format!("Access granted to '{}'. Payment settled on {}.", resource, cfg.x402_network),
        "network":     cfg.x402_network,
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
