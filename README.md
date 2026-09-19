# UMOD Codex GUI

Rust GUI-лаунчер для запуску [Codex CLI](https://github.com/openai/codex) з профілем `umod` — інфраструктурою Міністерства оборони України ([inference.ai.mod.gov.ua](https://inference.ai.mod.gov.ua)).

Не установник. Припускає, що UMOD вже налаштований через [UMOD Installer](https://github.com/RiasJ1Dar/umod-installer).

## Можливості

- Вибір моделі з випадаючого списку (glm-5.2, gpt-5.6-sol/terra/luna, gpt-6-astra)
- Вибір credential-файлу (автозаповнення з `~/.umod/`, попередження, якщо файлів кілька)
- Картка статусу проксі: працює / запускається / не працює, адреса, аптайм
- Кнопка «Запустити Codex» — стартує проксі (за потреби) + Codex у новому вікні
- Кнопки «Пуск проксі» / «Стоп проксі» — активні лише тоді, коли дія має сенс
- Журнал подій з часовими мітками і кнопкою «скопіювати»

Вигляд вікна описаний у [DESIGN.md](DESIGN.md).

## Як працює

```
Користувач → «Запустити Codex»
   ↓
Перевірити порт 8787 → проксі працює?
   ↓ ні
Запустити umod-token-proxy (фоновий процес)
   ↓
Дочекатися відповіді з порту 8787 (до 20 с)
   ↓
Запустити codex --profile umod --model <вибрана>
   ↓
Codex працює
```

Проксі тримає Entra-токен і оновлює його за 5 хвилин до закінчення, тож довга агentic-сесія не впаде з 401 посеред роботи.

## Збірка

Потребує Rust 1.95+ і кеш крейтів (egui 0.36).

```powershell
cd umod-codex-gui
cargo build --release
```

Готовий `.exe` — у `target/release/umod-codex-gui.exe`.

## Структура

```
umod-codex-gui/
├── Cargo.toml              eframe 0.36 (egui), default-features=false
├── src/
│   ├── main.rs             GUI: вікно, вибір моделі, журнал
│   ├── theme.rs            токени кольору і метрики, стиль egui
│   ├── ui.rs               піктограми і кнопки, намальовані painter'ом
│   ├── proxy.rs            логіка проксі (пошук credential, старт/стоп)
│   └── codex.rs            запуск Codex CLI
├── .cargo/config.toml      vendor source (офлайн-збірка)
└── vendor/                 кеш крейтів (не в git, качається скриптом)
```

## Безпека

- `UMOD_PROXY_KEY` — placeholder, не секрет. Проксі замінює заголовок Authorization на справжній Entra-токен
- Credential-файл — єдиний секрет, GUI не показує вміст
- Проксі слухає лише на `127.0.0.1`
- Бінарник не потребує прав адміністратора

## Повʼязані проєкти

- [umod-installer](https://github.com/RiasJ1Dar/umod-installer) — установник UMOD для Windows/macOS
- [Codex CLI](https://github.com/openai/codex) — офіційний клієнт OpenAI Codex

## Ліцензія

MIT
