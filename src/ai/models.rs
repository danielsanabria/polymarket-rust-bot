use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum AiAction {
    WAIT,
    TRADE,
    HALT,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiDecision {
    pub action: AiAction,
    pub confidence: u8,
}

impl Default for AiDecision {
    fn default() -> Self {
        Self {
            action: AiAction::WAIT, // Conservative fallback
            confidence: 0,
        }
    }
}

// Ollama API Models
#[derive(Debug, Serialize)]
pub struct OllamaRequest {
    pub model: String,
    pub prompt: String,
    pub format: String,
    pub stream: bool,
}

#[derive(Debug, Deserialize)]
pub struct OllamaResponse {
    pub response: String,
}

#[derive(Debug, Clone)]
pub struct AiContext {
    pub asset: String,
    pub straddle_cost: f64,
    pub time_remaining_secs: i64,
    pub btc_volatility: f64,
}
