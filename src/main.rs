//! ARIA Node — entry point.
//!
//! Starts the Axum HTTP server with all module routers attached.
//! CORS is open (for local testing); tighten in production.

mod config;
mod modules;
mod state;

use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() {
    // ── Logging ───────────────────────────────────────────────────────────────
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "aria_node=debug,tower_http=info".parse().unwrap()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // ── Config ────────────────────────────────────────────────────────────────
    dotenvy::dotenv().ok();
    let config  = config::Config::from_env();
    let addr    = config.bind_addr;
    let state   = state::AppState::new(config);

    // ── Router ────────────────────────────────────────────────────────────────
    let app = modules::all_routes()
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );

    // ── Startup banner ────────────────────────────────────────────────────────
    info!("");
    info!("╔══════════════════════════════════════════════╗");
    info!("║            ARIA Node  v0.1.0                 ║");
    info!("╠══════════════════════════════════════════════╣");
    info!("║  Public Base   →  GET /                      ║");
    info!("║  x402 gateway  →  GET /protected             ║");
    info!("║  DID registry  →  /did/*    [active]         ║");
    info!("║  A2A protocol  →  /agent/*  [active]         ║");
    info!("║  Health        →  GET /health                ║");
    info!("╚══════════════════════════════════════════════╝");
    info!("Listening on http://{}", addr);

    // ── Serve ─────────────────────────────────────────────────────────────────
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind");

    axum::serve(listener, app)
        .await
        .expect("server error");
}
