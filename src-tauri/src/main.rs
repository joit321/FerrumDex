#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Импортируем модули парсеров, чтобы они были доступны внутри src-tauri
mod bingxparser;
mod bybitparser;

#[tokio::main]
async fn main() {
    // Запуск программы. Вся магия теперь происходит внутри lib.rs
    rustls::crypto::ring::default_provider().install_default()
        .expect("Failed to install rustls crypto provider");
    ferrumdex_lib::run();
}
