pub mod models;

use log::{debug, error, info, warn};
use reqwest::Client;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};

pub use models::{AiAction, AiContext, AiDecision, OllamaRequest, OllamaResponse};

pub type SharedAiState = Arc<RwLock<AiDecision>>;
pub type SharedAiContext = Arc<RwLock<Option<AiContext>>>;

pub struct AiEngine {
    pub state: SharedAiState,
    pub context: SharedAiContext,
}

impl AiEngine {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(AiDecision::default())),
            context: Arc::new(RwLock::new(None)),
        }
    }

    pub fn start_background_task(&self) {
        let state = self.state.clone();
        let context = self.context.clone();

        tokio::spawn(async move {
            let client = Client::builder()
                .timeout(Duration::from_secs(120)) // 2 min timeout for slow CPU inference
                .build()
                .unwrap_or_else(|_| Client::new());
            let url = "http://127.0.0.1:11434/api/generate";

            info!("Local AI Engine (Ollama + Gemma 2B) started. Optimized for low-power CPU.");

            loop {
                let ctx = {
                    let ctx_guard = context.read().await;
                    ctx_guard.clone()
                };

                if let Some(c) = ctx {
                    // Shortened prompt to minimize token processing time on CPU
                    let prompt = format!(
                        "Analyze arbitrage: Asset:{} Cost:${:.4} Time:{}s BTCVol:{:.4}%.\n\
                        If cost > 0.94 or time < 60s, output HALT.\n\
                        Else TRADE or WAIT.\n\
                        Reply ONLY JSON: {{\"action\":\"WAIT|TRADE|HALT\",\"confidence\":0-100}}",
                        c.asset, c.straddle_cost, c.time_remaining_secs, c.btc_volatility * 100.0
                    );

                    let req_body = OllamaRequest {
                        model: "gemma:2b".to_string(),
                        prompt,
                        format: "json".to_string(),
                        stream: false,
                    };

                    match client.post(url).json(&req_body).send().await {
                        Ok(resp) => {
                            if resp.status().is_success() {
                                if let Ok(ollama_resp) = resp.json::<OllamaResponse>().await {
                                    match serde_json::from_str::<AiDecision>(&ollama_resp.response) {
                                        Ok(decision) => {
                                            info!("AI Decision for {}: {:?}", c.asset, decision);
                                            let mut state_guard = state.write().await;
                                            *state_guard = decision;
                                        }
                                        Err(e) => {
                                            warn!("Failed to parse AI JSON response: {}. Response was: {}. Falling back to WAIT.", e, ollama_resp.response);
                                            let mut state_guard = state.write().await;
                                            *state_guard = AiDecision::default();
                                        }
                                    }
                                }
                            } else {
                                warn!("Ollama API returned error status: {:?}", resp.status());
                            }
                        }
                        Err(e) => {
                            error!("Failed to reach local Ollama API (is Ollama running?): {}", e);
                            let mut state_guard = state.write().await;
                            *state_guard = AiDecision::default();
                        }
                    }
                }
                
                // Poll less frequently to avoid pegging the low-power CPU at 100% constantly
                sleep(Duration::from_secs(60)).await;
            }
        });
    }
}
