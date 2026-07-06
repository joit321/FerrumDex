mod bingxparser;
mod bybitparser;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tauri::State;
use tokio::task::JoinHandle;

// Структура данных, которая будет передаваться во фронтенд (название монеты, текущая цена и изменение за 24ч)
#[derive(Serialize, Clone)]
pub struct CoinInfo {
    pub name: String,
    pub price: f64,
    pub change_24h: f64,
}

// Глобальное состояние приложения Tauri
pub struct CryptoPriceState {
    // Кэш с данными по всем отслеживаемым монетам
    pub market_data: Arc<RwLock<HashMap<String, CoinInfo>>>,
    // Хранилище активных задач (потоков) для каждой монеты, чтобы можно было их останавливать
    pub active_tasks: Arc<RwLock<HashMap<String, JoinHandle<()>>>>,
}

// Простая команда для проверки связи между Rust и JS
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

// Команда для получения текущего списка цен из кэша
#[tauri::command]
async fn get_cached_price(state: State<'_, CryptoPriceState>) -> Result<Vec<CoinInfo>, String> {
    // Блокируем чтение хэш-мапы
    let map = state
        .market_data
        .read()
        .map_err(|_| "Ошибка блокировки кэша".to_string())?;
    
    // Преобразуем значения HashMap в вектор и возвращаем
    Ok(map.values().cloned().collect())
}

// Команда для запуска отслеживания конкретной монеты
#[tauri::command]
async fn start_coin_tracker(
    ticker: String,
    state: State<'_, CryptoPriceState>,
) -> Result<(), String> {
    // Логирование для отладки
    println!("=== РАЦИОНАЛЬНОСТЬ: Включен WebSocket для {}", ticker);

    // Проверяем, не запущен ли уже трекер для этой монеты, чтобы избежать дублирования
    {
        let tasks = state.active_tasks.read().unwrap();
        if tasks.contains_key(&ticker) {
            return Ok(());
        }
    }

    let map_clone = state.market_data.clone();
    let ticker_str = ticker.clone(); // Клонируем тикер для использования внутри асинхронного потока

    // Запускаем фоновую задачу (tokio::spawn) для обработки одной монеты
    let handle = tokio::spawn(async move {
        // Локальная переменная для хранения последней цены, полученной из WebSocket
        let local_shared_price = Arc::new(RwLock::new(0.0));
        let local_price_reader = local_shared_price.clone();
        let mut ticker_for_parser = ticker_str.clone();

        // Запускаем сам парсер WebSocket в отдельном потоке внутри этой задачи
        tokio::spawn(async move {
            bybitparser::start_price_tracker(&mut ticker_for_parser, local_shared_price).await;
        });

        // Внутренний цикл обновления кэша конкретной монеты каждые 2 секунды
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            
            // Считываем текущую цену из локального хранилища
            let current_price = *local_price_reader.read().unwrap();
            
            // Если цена еще не получена (равна 0), пропускаем итерацию
            if current_price == 0.0 {
                continue;
            }

            // Обновляем глобальный кэш
            let mut map = map_clone.write().unwrap();
            
            // Рассчитываем процент изменения цены по сравнению с предыдущим значением в кэше
            let change = if let Some(old_coin) = map.get(&ticker_str) {
                if old_coin.price > 0.0 && old_coin.price != current_price {
                    ((current_price - old_coin.price) / old_coin.price) * 100.0
                } else {
                    old_coin.change_24h
                }
            } else {
                0.0
            };

            // Сохраняем обновленную информацию о монете
            map.insert(
                ticker_str.clone(),
                CoinInfo {
                    name: ticker_str.clone(), // Убираем суффикс USDT для красивого отображения имени
                    price: current_price,
                    change_24h: change,
                },
            );
        }
    });

    // Сохраняем Handle (идентификатор) задачи, чтобы потом можно было её остановить
    let mut tasks = state.active_tasks.write().unwrap();
    tasks.insert(ticker, handle); 
    
    Ok(())
}

// Команда для полного отключения парсинга монеты и обрыва соединения
#[tauri::command]
async fn stop_coin_tracker(
    ticker: String,
    state: State<'_, CryptoPriceState>,
) -> Result<(), String> {
    // 1. Находим и убиваем фоновый поток
    let mut tasks = state.active_tasks.write().unwrap();
    if let Some(handle) = tasks.remove(&ticker) {
        handle.abort(); // Жестко прерываем выполнение WebSocket и цикла обновления
        println!(
            "=== РАЦИОНАЛЬНОСТЬ: Отключен и уничтожен сокет для {}",
            ticker
        );
    }

    // 2. Удаляем старые котировки из кэша, чтобы они исчезли с экрана пользователя
    let mut map = state.market_data.write().unwrap();
    map.remove(&ticker);
    
    Ok(())
}

// Точка входа приложения Tauri
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let price_state = CryptoPriceState {
        market_data: Arc::new(RwLock::new(HashMap::new())),
        active_tasks: Arc::new(RwLock::new(HashMap::new())),
    };

    // --- БЛОК ДЛЯ ТЕСТА ПАРСИНГА ПРИ СТАРТЕ ---
    // Принудительно запускаем отслеживание BTCUSDT сразу после запуска приложения
    let map_clone = price_state.market_data.clone();
    tokio::spawn(async move {
        println!("=== ТЕСТ: Принудительный запуск парсинга BTCUSDT при старте ===");
        let local_shared_price = Arc::new(RwLock::new(0.0));
        // Передаем в парсер
        bybitparser::start_price_tracker(&mut format!("BTC"), local_shared_price.clone()).await;
    });
    // -------------------------------------------------------

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(price_state) // Регистрируем глобальное состояние
        .invoke_handler(tauri::generate_handler![
            greet,
            get_cached_price,
            start_coin_tracker,
            stop_coin_tracker
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}