//! UMOD Codex GUI — лаунчер для запуску Codex CLI з профілем umod.
//!
//! Вікно: вибрав модель → «Запустити» → відкрився Codex.
//! Припускає, що UMOD вже налаштований (config.toml, credential-файл).

mod codex;
mod proxy;

use eframe::egui;
use std::sync::mpsc;

/// Моделі UMOD (з umod-models.json).
const MODELS: &[&str] = &[
    "glm-5.2",
    "gpt-5.6-sol",
    "gpt-5.6-terra",
    "gpt-5.6-luna",
    "gpt-6-astra",
    "deepseek-v4.1-flash",
];

/// Запис у журналі.
#[derive(Clone)]
struct LogEntry {
    time: String,
    msg: String,
    level: LogLevel,
}

#[derive(Clone, PartialEq)]
enum LogLevel {
    Info,
    Ok,
    Warn,
    Error,
}

impl LogLevel {
    fn color(&self) -> egui::Color32 {
        match self {
            LogLevel::Info => egui::Color32::from_gray(180),
            LogLevel::Ok => egui::Color32::from_rgb(100, 200, 100),
            LogLevel::Warn => egui::Color32::from_rgb(220, 180, 60),
            LogLevel::Error => egui::Color32::from_rgb(220, 80, 80),
        }
    }
}

/// Стан GUI.
struct App {
    /// Вибрана модель.
    model: String,
    /// Порт проксі.
    port: String,
    /// Шлях до credential-файлу.
    cred_path: String,
    /// Журнал подій.
    log: Vec<LogEntry>,
    /// Стан проксі.
    proxy_running: bool,
    /// Чи триває операція.
    busy: bool,
    /// Канал для повідомлень з фонових потоків.
    rx: mpsc::Receiver<LogEntry>,
    tx: mpsc::Sender<LogEntry>,
}

impl App {
    fn new() -> Self {
        let (tx, rx) = mpsc::channel();

        // Автозаповнення credential
        let cred = proxy::ProxyState::find_credential()
            .map(|p| p.display().to_string())
            .unwrap_or_default();

        let port = std::env::var("UMOD_PROXY_PORT")
            .unwrap_or_else(|_| "8787".to_string());

        Self {
            model: MODELS[0].to_string(),
            port,
            cred_path: cred,
            log: vec![LogEntry {
                time: now_str(),
                msg: "UMOD Codex GUI готовий".to_string(),
                level: LogLevel::Info,
            }],
            proxy_running: false,
            busy: false,
            rx,
            tx,
        }
    }

    fn add_log(&mut self, msg: &str, level: LogLevel) {
        self.log.push(LogEntry {
            time: now_str(),
            msg: msg.to_string(),
            level,
        });
    }

    /// Перевіряє вхідні повідомлення з фонових потоків.
    fn poll_log(&mut self) {
        while let Ok(entry) = self.rx.try_recv() {
            if entry.msg.contains("проксі запущено") || entry.msg.contains("proxy started") {
                self.proxy_running = true;
                self.busy = false;
            }
            if entry.msg.contains("помилка") || entry.msg.contains("Error") || entry.level == LogLevel::Error {
                self.busy = false;
            }
            self.log.push(entry);
        }
    }

    /// Запускає проксі у фоновому потоці.
    fn start_proxy_bg(&mut self) {
        if self.busy {
            return;
        }
        self.busy = true;
        let tx = self.tx.clone();
        let port: u16 = self.port.parse().unwrap_or(8787);
        let cred = self.cred_path.clone();

        self.add_log("Запускаю проксі...", LogLevel::Info);

        std::thread::spawn(move || {
            let mut state = proxy::ProxyState::new(port);
            if !cred.is_empty() {
                state.cred_path = Some(std::path::PathBuf::from(&cred));
            }

            if state.is_listening() {
                let _ = tx.send(LogEntry {
                    time: now_str(),
                    msg: format!("Проксі вже працює на 127.0.0.1:{}", port),
                    level: LogLevel::Ok,
                });
                return;
            }

            match state.start() {
                Ok(()) => {
                    let _ = tx.send(LogEntry {
                        time: now_str(),
                        msg: format!("Проксі запущено на 127.0.0.1:{}", port),
                        level: LogLevel::Ok,
                    });
                    // Не даємо state померти — leak, бо проксі має працювати
                    // після завершення потоку.
                    std::mem::forget(state);
                }
                Err(e) => {
                    let _ = tx.send(LogEntry {
                        time: now_str(),
                        msg: format!("Помилка: {}", e),
                        level: LogLevel::Error,
                    });
                }
            }
        });
    }

    /// Запускає Codex (попередньо переконавшись, що проксі працює).
    fn launch_codex(&mut self) {
        if self.busy {
            return;
        }

        let port: u16 = self.port.parse().unwrap_or(8787);

        // Перевіряємо проксі синхронно
        let state = proxy::ProxyState::new(port);
        if !state.is_listening() {
            self.add_log("Проксі не працює — запускаю спочатку", LogLevel::Warn);
            self.start_proxy_bg();
            return;
        }

        let model = self.model.clone();
        self.add_log(&format!("Запускаю Codex з моделлю {}...", model), LogLevel::Info);

        match codex::launch(&model, port) {
            Ok(()) => {
                self.add_log(&format!("Codex запущено (модель: {})", model), LogLevel::Ok);
            }
            Err(e) => {
                self.add_log(&format!("Помилка запуску Codex: {}", e), LogLevel::Error);
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_log();

        // Авто-refresh — щоб фонові повідомлення приходили
        ctx.request_repaint_after(std::time::Duration::from_millis(200));

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("UMOD Codex");
            ui.add_space(8.0);

            // Модель
            ui.horizontal(|ui| {
                ui.label("Модель:");
                egui::ComboBox::from_id_salt("model_combo")
                    .selected_text(&self.model)
                    .show_ui(ui, |ui| {
                        for m in MODELS {
                            ui.selectable_value(&mut self.model, m.to_string(), *m);
                        }
                    });
            });

            // Credential
            ui.horizontal(|ui| {
                ui.label("Credential:");
                ui.text_edit_singleline(&mut self.cred_path);
                if ui.button("Огляд...").clicked() {
                    if let Some(path) = rfd_file_dialog() {
                        self.cred_path = path.display().to_string();
                    }
                }
            });

            // Порт
            ui.horizontal(|ui| {
                ui.label("Порт проксі:");
                ui.add(egui::TextEdit::singleline(&mut self.port).desired_width(80.0));
            });

            ui.add_space(8.0);

            // Статус проксі
            ui.horizontal(|ui| {
                let (dot, text) = if self.busy {
                    (egui::Color32::from_rgb(220, 180, 60), "запускається...")
                } else if self.proxy_running {
                    (egui::Color32::from_rgb(100, 200, 100), format!("працює на 127.0.0.1:{}", self.port))
                } else {
                    (egui::Color32::from_rgb(200, 80, 80), "не працює")
                };
                ui.colored_label(dot, "●");
                ui.label(text);
            });

            ui.add_space(8.0);

            // Кнопки
            ui.horizontal(|ui| {
                let launch_btn = ui.add_sized(
                    [120.0, 32.0],
                    egui::Button::new("▶ Запустити Codex"),
                );
                if launch_btn.clicked() {
                    self.launch_codex();
                }

                let proxy_btn = ui.add_sized(
                    [120.0, 32.0],
                    egui::Button::new("⟳ Пуск проксі"),
                );
                if proxy_btn.clicked() {
                    self.start_proxy_bg();
                }
            });

            ui.add_space(8.0);

            // Журнал
            ui.separator();
            ui.label("Журнал:");
            egui::ScrollArea::vertical()
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    for entry in &self.log {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(&entry.time).color(egui::Color32::from_gray(120)).small());
                            ui.label(egui::RichText::new(&entry.msg).color(entry.level.color()));
                        });
                    }
                    if let Some(last) = ui.min_rect().max.y.checked_sub(1) {
                        ui.scroll_to_rect(egui::Rect::everything_above(last), Some(egui::Align::BOTTOM));
                    }
                });
        });
    }
}

fn now_str() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let h = (secs / 3600) % 24;
    let m = (secs / 60) % 60;
    let s = secs % 60;
    format!("{:02}:{:02}:{:02}", h, m, s)
}

#[cfg(target_os = "windows")]
fn rfd_file_dialog() -> Option<std::path::PathBuf> {
    // Простий win32 діалог без зовнішніх крейтів
    // PowerShell GetOpenFileName
    let script = "Add-Type -AssemblyName System.Windows.Forms; $f = New-Object System.Windows.Forms.OpenFileDialog; $f.Filter = 'Credential files (*.env)|*.env|All files (*.*)|*.*'; $f.InitialDirectory = $env:USERPROFILE + '\\.umod'; if ($f.ShowDialog() -eq 'OK') { $f.FileName }";
    if let Ok(output) = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", script])
        .output()
    {
        if output.status.success() {
            let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !s.is_empty() {
                return Some(std::path::PathBuf::from(s));
            }
        }
    }
    None
}

#[cfg(not(target_os = "windows"))]
fn rfd_file_dialog() -> Option<std::path::PathBuf> {
    None
}

fn main() -> eframe::Result<()> {
    // env_logger не використовується — log іде у GUI-журнал

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([480.0, 520.0])
            .with_min_inner_size([400.0, 400.0])
            .with_title("UMOD Codex"),
        ..Default::default()
    };

    eframe::run_native(
        "UMOD Codex",
        options,
        Box::new(|_cc| Ok(Box::new(App::new()))),
    )
}

