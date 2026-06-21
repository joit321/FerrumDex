import { invoke } from "@tauri-apps/api/core";

let greetInputEl: HTMLInputElement | null;
let greetMsgEl: HTMLElement | null;
let cryptoPriceEl: HTMLElement | null;

async function greet() {
  if (greetMsgEl && greetInputEl) {
    greetMsgEl.textContent = await invoke("greet", {
      name: greetInputEl.value,
    });
  }
}

async function updateCryptoPrice() {
  if (cryptoPriceEl) {
    try {
      const price: number = await invoke("get_cached_price");
      cryptoPriceEl.textContent = price > 0 ? `$${price.toFixed(2)}` : "Загрузка...";
    } catch (error) {
      console.error("Ошибка при получении цены:", error);
      cryptoPriceEl.textContent = "Ошибка сети";
    }
  }
}

window.addEventListener("DOMContentLoaded", () => {
  greetInputEl = document.querySelector("#greet-input");
  greetMsgEl = document.querySelector("#greet-msg");
  cryptoPriceEl = document.querySelector("#crypto-price");

  document.querySelector("#greet-form")?.addEventListener("submit", (e) => {
    e.preventDefault();
    greet();
  });

  updateCryptoPrice();
  setInterval(updateCryptoPrice, 1000);
});
