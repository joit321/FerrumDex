use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::sync::{Arc, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

/// Постоянно держит WebSocket открытым и обновляет общую переменную цены
pub async fn start_price_tracker(symbol: &str, shared_price: Arc<RwLock<f64>>) {
    let url = "wss://stream.bybit.kz/v5/public/linear";

    loop {
        println!("[WS] Подключение к Bybit...");
        match connect_async(url).await {
            Ok((ws_stream, _)) => {
                let (mut write, mut read) = ws_stream.split();

                let subscribe_msg = json!({
                    "op": "subscribe",
                    "args": [format!("tickers.{}", symbol)]
                });

                if write
                    .send(Message::Text(subscribe_msg.to_string().into()))
                    .await
                    .is_err()
                {
                    continue;
                }

                while let Some(Ok(msg)) = read.next().await {
                    if let Message::Text(text) = msg {
                        if let Ok(v) = serde_json::from_str::<Value>(&text) {
                            if let Some(last_price_str) = v
                                .get("data")
                                .and_then(|d| d.get("lastPrice"))
                                .and_then(|p| p.as_str())
                            {
                                if let Ok(price) = last_price_str.parse::<f64>() {
                                    if let Ok(mut lock) = shared_price.write() {
                                        *lock = price;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                // ЭТА СТРОКА НАПЕЧАТАЕТ РЕАЛЬНУЮ ПРИЧИНУ, ЕСЛИ ПОДКЛЮЧЕНИЕ СОРВЕТСЯ
                println!("[WS] Ошибка подключения: {:?}", e);
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}

