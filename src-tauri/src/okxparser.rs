use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::sync::{Arc, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

// Постоянно держит WebSocket открытым и обновляет общую переменную цены для OKX.
#[allow(dead_code)]
pub async fn start_price_tracker(symbol: &mut String, shared_price: Arc<RwLock<f64>>) {
    // Публичный стрим OKX v5
    let url = "wss://://okx.com";
    symbol.push_str("-USDT");

    loop {
        println!("[WS] Подключение к OKX...");
        
        match connect_async(url).await {
            Ok((ws_stream, _)) => {
                let (mut write, mut read) = ws_stream.split();
                
                // Формируем JSON-сообщение подписки на канал tickers для OKX
                let subscribe_msg = json!({
                    "op": "subscribe",
                    "args": [{
                        "channel": "tickers",
                        "instId": symbol
                    }]
                });

                if write
                    .send(Message::Text(subscribe_msg.to_string().into()))
                    .await
                    .is_err()
                {
                    continue;
                }

                // Читаем входящие сообщения
                while let Some(Ok(msg)) = read.next().await {
                    if let Message::Text(text) = msg {
                        if let Ok(v) = serde_json::from_str::<Value>(&text) {
                            // OKX присылает массив объектов в поле "data"
                            if let Some(data_array) = v.get("data").and_then(|d| d.as_array()) {
                                if let Some(ticker) = data_array.first() {
                                    // Извлекаем текущую цену из поля "last"
                                    if let Some(last_price_str) = ticker.get("last").and_then(|p| p.as_str()) {
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
                }
            }
            Err(e) => {
                println!("[WS] Ошибка подключения: {:?}", e);
            }
        }
        
        // Пауза перед реконнектом
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}