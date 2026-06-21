#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Импортируем модуль парсера, чтобы он был доступен внутри src-tauri
mod parser; 

#[tokio::main]
async fn main() {
    // Запуск программы. Вся магия теперь происходит внутри lib.rs
    ferrumdex_lib::run();
}
