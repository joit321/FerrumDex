use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::sync::{Arc, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

// Постоянно держит WebSocket открытым и обновляет общую переменную цены.
// Эта функция работает в бесконечном цикле: если соединение разрывается, оно автоматически переподключается через 5 секунд.
#[allow(dead_code)]
pub async fn start_price_tracker(symbol: &mut String, shared_price: Arc<RwLock<f64>>) {
    // Используем публичный стрим Bybit для линейных контрактов (фьючерсы)
    let url = "wss://stream.bybit.kz/v5/public/linear"; // Казахское зеркало
    symbol.push_str("USDT");

    loop {
        println!("[WS] Подключение к Bybit...");
        
        // Пытаемся установить WebSocket-соединение
        match connect_async(url).await {
            Ok((ws_stream, _)) => {
                // Разделяем поток на отправку (write) и получение (read) сообщений
                let (mut write, mut read) = ws_stream.split();
                
                // Формируем JSON-сообщение для подписки на тикер конкретной монеты (например, tickers.BTCUSDT)
                let subscribe_msg = json!({
                    "op": "subscribe",
                    "args": [format!("tickers.{}", symbol)]
                });

                // Отправляем сообщение о подписке
                if write
                    .send(Message::Text(subscribe_msg.to_string().into()))
                    .await
                    .is_err()
                {
                    // Если отправка не удалась, прерываем текущую итерацию и пробуем подключиться снова
                    continue;
                }

                // Читаем входящие сообщения от биржи
                while let Some(Ok(msg)) = read.next().await {
                    if let Message::Text(text) = msg {
                        // Парсим полученный JSON
                        if let Ok(v) = serde_json::from_str::<Value>(&text) {
                            // Извлекаем поле "lastPrice" из вложенной структуры "data"
                            if let Some(last_price_str) = v
                                .get("data")
                                .and_then(|d| d.get("lastPrice"))
                                .and_then(|p| p.as_str())
                            {
                                // Преобразуем строку цены в число f64
                                if let Ok(price) = last_price_str.parse::<f64>() {
                                    // Записываем новую цену в общую переменную (Arc<RwLock<f64>>), защищенную мьютексом
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
                // Выводим реальную причину ошибки, если подключение сорвалось (например, нет интернета или ошибка DNS)
                println!("[WS] Ошибка подключения: {:?}", e);
            }
        }
        
        // Ждем 5 секунд перед следующей попыткой подключения (защита от спама запросами при ошибке)
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}