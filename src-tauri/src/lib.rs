mod parser;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tauri::State;
use tokio::task::JoinHandle;

// Структура для фронтенда (название, курс, изменение)
#[derive(Serialize, Clone)]
pub struct CoinInfo {
    pub name: String,
    pub price: f64,
    pub change_24h: f64,
}

pub struct CryptoPriceState {
    pub market_data: Arc<RwLock<HashMap<String, CoinInfo>>>,
    pub active_tasks: Arc<RwLock<HashMap<String, JoinHandle<()>>>>,
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn get_cached_price(state: State<'_, CryptoPriceState>) -> Result<Vec<CoinInfo>, String> {
    let map = state
        .market_data
        .read()
        .map_err(|_| "Ошибка блокировки кэша".to_string())?;

    // Безопасно вытаскиваем все монеты в виде списка
    Ok(map.values().cloned().collect())
}

#[tauri::command]
async fn start_coin_tracker(
    ticker: String,
    state: State<'_, CryptoPriceState>,
) -> Result<(), String> {
    // СРАЗУ выводим лог, пока владеем переменной ticker
    println!("=== РАЦИОНАЛЬНОСТЬ: Включен WebSocket для {}", ticker);

    // Если монета уже парсится — ничего не делаем
    {
        let tasks = state.active_tasks.read().unwrap();
        if tasks.contains_key(&ticker) {
            return Ok(());
        }
    }

    let map_clone = state.market_data.clone();
    let ticker_str = ticker.clone(); // Это клонирование для фонового потока

    // Запускаем выделенный процесс под ОДНУ монету
    let handle = tokio::spawn(async move {
        let local_shared_price = Arc::new(RwLock::new(0.0));
        let local_price_reader = local_shared_price.clone();
        let ticker_for_parser = ticker_str.clone();

        // Запуск WebSocket-парсера
        tokio::spawn(async move {
            parser::start_price_tracker(&ticker_for_parser, local_shared_price).await;
        });

        // Внутренний цикл обновления кэша конкретной монеты
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            let current_price = *local_price_reader.read().unwrap();
            if current_price == 0.0 {
                continue;
            }

            let mut map = map_clone.write().unwrap();
            let change = if let Some(old_coin) = map.get(&ticker_str) {
                if old_coin.price > 0.0 && old_coin.price != current_price {
                    ((current_price - old_coin.price) / old_coin.price) * 100.0
                } else {
                    old_coin.change_24h
                }
            } else {
                0.0
            };

            map.insert(
                ticker_str.clone(),
                CoinInfo {
                    name: ticker_str.replace("USDT", ""),
                    price: current_price,
                    change_24h: change,
                },
            );
        }
    });

    // Сохраняем JoinHandle задачи в менеджер потоков
    let mut tasks = state.active_tasks.write().unwrap();
    tasks.insert(ticker, handle); // Здесь ticker окончательно переходит в HashMap

    Ok(())
}

// Полное отключение парсинга монеты и обрыв соединения
#[tauri::command]
async fn stop_coin_tracker(
    ticker: String,
    state: State<'_, CryptoPriceState>,
) -> Result<(), String> {
    // 1. Убиваем фоновый поток
    let mut tasks = state.active_tasks.write().unwrap();
    if let Some(handle) = tasks.remove(&ticker) {
        handle.abort(); // Жестко прерываем выполнение WebSocket и таймера внутри
        println!(
            "=== РАЦИОНАЛЬНОСТЬ: Отключен и уничтожен сокет для {}",
            ticker
        );
    }

    // 2. Стираем старые котировки из кэша, чтобы они пропали с экрана
    let mut map = state.market_data.write().unwrap();
    map.remove(&ticker);

    Ok(())
}

// Сборка приложения
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let price_state = CryptoPriceState {
        market_data: Arc::new(RwLock::new(HashMap::new())),
        active_tasks: Arc::new(RwLock::new(HashMap::new())),
    };

    // --- ДОБАВЬТЕ ЭТОТ БЛОК ДЛЯ ТЕСТА ПАРСИНГА ПРИ СТАРТЕ ---
    let map_clone = price_state.market_data.clone();
    tokio::spawn(async move {
        println!("=== ТЕСТ: Принудительный запуск парсинга BTCUSDT при старте ===");
        let local_shared_price = Arc::new(RwLock::new(0.0));

        // Передаем в ваш парсер
        parser::start_price_tracker("BTCUSDT", local_shared_price.clone()).await;
    });
    // -------------------------------------------------------

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(price_state)
        .invoke_handler(tauri::generate_handler![
            greet,
            get_cached_price,
            start_coin_tracker,
            stop_coin_tracker
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
