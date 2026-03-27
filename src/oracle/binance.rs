use futures_util::{StreamExt, SinkExt};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use serde_json::Value;
use log::{info, error, debug};
use url::Url;

pub struct BinanceOracle {
    prices: Arc<RwLock<HashMap<String, f64>>>,
    btc_history: Arc<RwLock<Vec<(u64, f64)>>>, // (timestamp_ms, price)
    assets: Vec<String>,
}

impl BinanceOracle {
    pub fn new(assets: Vec<String>) -> Self {
        Self {
            prices: Arc::new(RwLock::new(HashMap::new())),
            btc_history: Arc::new(RwLock::new(Vec::with_capacity(100))),
            assets,
        }
    }

    pub async fn get_price(&self, asset: &str) -> Option<f64> {
        let prices = self.prices.read().await;
        prices.get(asset).copied()
    }

    pub async fn get_btc_volatility(&self, window_ms: u64) -> Option<f64> {
        let history = self.btc_history.read().await;
        if history.len() < 2 { return None; }
        
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
        let start_time = now - window_ms;
        
        let recent_prices: Vec<f64> = history.iter()
            .filter(|(t, _)| *t >= start_time)
            .map(|(_, p)| *p)
            .collect();
            
        if recent_prices.len() < 2 { return None; }
        
        let first = recent_prices.first()?;
        let last = recent_prices.last()?;
        Some((last - first) / first)
    }

    pub async fn run(&self) {
        let assets_lower: Vec<String> = self.assets.iter()
            .map(|a| format!("{}usdt", a.to_lowercase()))
            .collect();
        
        let streams = assets_lower.iter()
            .map(|s| format!("{}@aggTrade", s))
            .collect::<Vec<_>>()
            .join("/");

        let url_str = format!("wss://stream.binance.com:9443/stream?streams={}", streams);
        let url = Url::parse(&url_str).unwrap();

        loop {
            info!("Connecting to Binance WebSocket: {}", url_str);
            
            match connect_async(&url).await {
                Ok((mut ws_stream, _)) => {
                    info!("Successfully connected to Binance Oracle");
                    
                    while let Some(msg) = ws_stream.next().await {
                        match msg {
                            Ok(Message::Text(text)) => {
                                if let Ok(v) = serde_json::from_str::<Value>(&text) {
                                    if let Some(stream) = v["stream"].as_str() {
                                        let asset = stream.split('@').next().unwrap_or("").to_uppercase().replace("USDT", "");
                                        if let Some(price_str) = v["data"]["p"].as_str() {
                                            if let Ok(price) = price_str.parse::<f64>() {
                                                {
                                                    let mut prices = self.prices.write().await;
                                                    prices.insert(asset.clone(), price);
                                                }

                                                if asset == "BTC" {
                                                    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
                                                    let mut history = self.btc_history.write().await;
                                                    history.push((now, price));
                                                    // Keep only last 10 seconds of history (roughly 100-200 updates)
                                                    if history.len() > 200 {
                                                        history.remove(0);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Ok(Message::Ping(_)) => {
                                let _ = ws_stream.send(Message::Pong(vec![])).await;
                            }
                            Err(e) => {
                                error!("Binance WS Error: {}", e);
                                break;
                            }
                            _ => {}
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to connect to Binance: {}. Retrying in 5 seconds...", e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }
            }
        }
    }
}
