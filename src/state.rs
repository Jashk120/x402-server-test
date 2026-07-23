//! Shared Axum application state, threaded through every handler.

use std::sync::Arc;
use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub http:   reqwest::Client,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        Self {
            config: Arc::new(config),
            http:   reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .expect("failed to build reqwest client"),
        }
    }
}
