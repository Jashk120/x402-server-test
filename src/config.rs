//! Server configuration loaded from environment variables / .env file.

use std::net::SocketAddr;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct Config {
    /// Bind address (default: 0.0.0.0:3000)
    pub bind_addr: SocketAddr,

    // ── x402 gateway ──────────────────────────────────────────────────────────
    /// Hedera account that receives payments (payTo)
    pub x402_pay_to: String,
    /// Amount in tinybars required per request (default: 100_000_000 = 1 HBAR)
    pub x402_amount: String,
    /// Asset — "0.0.0" for HBAR, or an HTS token ID
    pub x402_asset: String,
    /// CAIP-2 network string
    pub x402_network: String,
    /// x402.org facilitator base URL for /verify
    pub facilitator_url: String,

    // ── DID registry (future) ─────────────────────────────────────────────────
    /// Hedera network the DID registry writes to ("testnet" | "mainnet")
    pub hedera_network: String,

    // ── Agent-to-agent protocol (future) ─────────────────────────────────────
    /// This node's own DID (set after first registration)
    pub node_did: Option<String>,
}

impl Config {
    pub fn from_env() -> Self {
        let port: u16 = env_or("PORT", "3000").parse().expect("PORT must be a number");
        let host = env_or("HOST", "0.0.0.0");
        let bind_addr = SocketAddr::from_str(&format!("{}:{}", host, port))
            .expect("Invalid HOST:PORT");

        Self {
            bind_addr,
            x402_pay_to:      env_or("X402_PAY_TO",      "0.0.9185802"),
            x402_amount:      env_or("X402_AMOUNT",      "100000000"),
            x402_asset:       env_or("X402_ASSET",       "0.0.0"),
            x402_network:     env_or("X402_NETWORK",     "hedera:testnet"),
            facilitator_url:  env_or("FACILITATOR_URL",  "https://api.testnet.blocky402.com"),
            hedera_network:   env_or("HEDERA_NETWORK",   "testnet"),
            node_did:         std::env::var("NODE_DID").ok(),
        }
    }
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}
