use std::io::Read;
use flate2::bufread::MultiGzDecoder;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::sync::{Arc, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

// функция для разжатия gzip ответов от биржи
fn parse_exchange_data(compressed_bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Создаем декодер поверх байт в памяти (они реализуют BufRead)
    let mut decoder = MultiGzDecoder::new(compressed_bytes);
    
    // 2. Стримим распакованные байты напрямую в парсер JSON без промежуточных String/Vec
    let json_data: Value = serde_json::from_reader(&mut decoder)?;
    
    // Выводим, например, цену или любой другой элемент
    println!("Данные успешно распарсены: {:?}", json_data.get("symbol"));
    
    Ok(())
}


