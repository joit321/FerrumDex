import "./styles.css";
import { invoke } from "@tauri-apps/api/core";

interface CoinInfo {
  name: string;
  price: number;
  change_24h: number;
}

// Массив с монетами
const ALL_TICKERS = [
  "BTC", "ETH", "SOL", "SLX",
];

// Функция создания чекбоксов (теперь вызывается сразу при старте)
function createFilterPanel() {
  const container = document.getElementById("checkboxes-list");
  const selectAll = document.getElementById("select-all-coins") as HTMLInputElement;
  if (!container || !selectAll) return;

  // Очищаем контейнер перед генерацией
  container.innerHTML = "";

  // Сортируем монеты по алфавиту для панели управления
  [...ALL_TICKERS].sort().forEach((ticker) => {
    const label = document.createElement("label");
    label.className = "checkbox-container";

    const checkbox = document.createElement("input");
    checkbox.type = "checkbox";
    checkbox.checked = false; // По умолчанию выключены для экономии ресурсов
    checkbox.dataset.ticker = ticker;

    checkbox.addEventListener("change", async (e) => {
      const target = e.target as HTMLInputElement;
      try {
        if (target.checked) {
          await invoke("start_coin_tracker", { ticker });
        } else {
          await invoke("stop_coin_tracker", { ticker });
          selectAll.checked = false;
        }
      } catch (err) {
        console.error("Ошибка вызова команды Tauri:", err);
      }
    });

    const text = document.createElement("span");
    text.className = "checkbox-text";
    text.textContent = ticker;

    label.appendChild(checkbox);
    label.appendChild(text);
    container.appendChild(label);
  });

  // Логика кнопки "Выбрать все"
  selectAll.addEventListener("change", async (e) => {
    const target = e.target as HTMLInputElement;
    const inputs = container.querySelectorAll("input[type='checkbox']");

    for (const input of Array.from(inputs)) {
      const htmlInput = input as HTMLInputElement;
      const ticker = htmlInput.dataset.ticker;
      if (!ticker) continue;

      try {
        if (target.checked && !htmlInput.checked) {
          htmlInput.checked = true;
          await invoke("start_coin_tracker", { ticker });
        } else if (!target.checked && htmlInput.checked) {
          htmlInput.checked = false;
          await invoke("stop_coin_tracker", { ticker });
        }
      } catch (err) {
        console.error("Ошибка пакетного вызова Tauri:", err);
      }
    }
  });
}

// Отрисовка таблицы монет
async function renderTable() {
  const tableBody = document.getElementById("coin-table-body");
  if (!tableBody) return;

  try {
    // ВНИМАНИЕ: Изменено на camelCase для Tauri v2!
    const coins: CoinInfo[] = await invoke("get_cached_price");

    if (coins.length === 0) {
      tableBody.innerHTML = `<tr><td colspan="3" class="loading">Нет активных монет. Включите их на панели справа.</td></tr>`;
      return;
    }

    tableBody.innerHTML = "";
    coins.sort((a, b) => a.name.localeCompare(b.name));

    coins.forEach((coin) => {
      const row = document.createElement("tr");
      const isPositive = coin.change_24h >= 0;
      const changeClass = isPositive ? "trend-up" : "trend-down";
      const changeSign = isPositive ? "+" : "";
      
      const formattedPrice = coin.price.toLocaleString(undefined, {
        minimumFractionDigits: coin.price < 1 ? 4 : 2,
        maximumFractionDigits: coin.price < 1 ? 6 : 2
      });

      row.innerHTML = `
        <td class="coin-name"><strong>${coin.name}</strong></td>
        <td class="coin-price">$${formattedPrice}</td>
        <td class="coin-change ${changeClass}">${changeSign}${coin.change_24h.toFixed(2)}%</td>
      `;
      tableBody.appendChild(row);
    });
  } catch (error) {
    console.error("Ошибка обновления таблицы:", error);
  }
}

// Запуск при загрузке страницы
window.addEventListener("DOMContentLoaded", () => {
  createFilterPanel(); // Чекбоксы появятся сразу!
  renderTable();
  setInterval(renderTable, 2000);
});
