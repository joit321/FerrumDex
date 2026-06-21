mod parser;
use std::sync::{Arc, RwLock};

pub struct CryptoPriceState {
    pub price: Arc<RwLock<f64>>,
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn get_cached_price(state: tauri::State<'_, CryptoPriceState>) -> Result<f64, String> {
    let price = *state.price.read().unwrap();
    Ok(price)
}

// Сборка приложения
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let price_arc = Arc::new(RwLock::new(0.0));
    let price_state = CryptoPriceState {
        price: price_arc.clone(),
    };

    let parser_price_link = price_arc.clone();

    // Запускаем фоновый процесс трекера всего одной строчкой
    tokio::spawn(async move {
        println!("ЗАПУСК ОБНОВЛЕНИЯ ЦЕН ИЗ ФОНА");
        parser::start_price_tracker("BTCUSDT", parser_price_link).await;
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(price_state)
        .invoke_handler(tauri::generate_handler![greet, get_cached_price])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
