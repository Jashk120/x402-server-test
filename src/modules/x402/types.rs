//! x402 payment gateway types.
//!
//! Typed mirrors of the x402 v2 wire format used by the x402.org facilitator.
//! Keep in sync with the daemon's `src/payments/x402_types.rs`.

use serde::{Deserialize, Serialize};

// ── Inbound: what an agent sends us in the PAYMENT-SIGNATURE header ───────────────

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PaymentPayload {
    #[serde(rename = "x402Version")]
    pub x402_version: u32,
    pub resource:   PaymentResource,
    pub accepted:   PaymentRequirements,
    pub payload:    PaymentPayloadData,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PaymentResource {
    pub url:         String,
    pub description: String,
    #[serde(rename = "mimeType")]
    pub mime_type:   String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PaymentRequirements {
    pub scheme:   String,
    pub network:  String,
    pub amount:   String,
    pub asset:    String,
    #[serde(rename = "payTo")]
    pub pay_to:   String,
    #[serde(rename = "maxTimeoutSeconds")]
    pub max_timeout_seconds: u64,
    #[serde(default)]
    pub extra:    serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PaymentPayloadData {
    pub transaction: String,
}

// ── x402.org facilitator response types ──────────────────────────────────────

#[derive(Deserialize, Debug)]
pub struct VerifyResponse {
    #[serde(rename = "isValid")]
    pub is_valid:       bool,
    #[serde(rename = "invalidReason")]
    pub invalid_reason: Option<String>,
    pub payer:          Option<String>,
}

// ── Outbound: 402 body we send to the agent ───────────────────────────────────

/// Full 402 response body an agent's `pay.x402` skill reads to construct
/// the payment payload.
#[derive(Serialize, Debug)]
pub struct PaymentRequiredBody {
    pub x402_version: u32,
    pub accepts: Vec<PaymentRequirements>,
    pub error:   &'static str,
}
