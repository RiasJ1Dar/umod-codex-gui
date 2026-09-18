//! UMOD Codex GUI — лаунчер для запуску Codex CLI з профілем umod.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod codex;
mod proxy;

use eframe::egui;
use std::sync::{mpsc, Arc, Mutex};

const MODELS: &[&str] = &[
    "glm-5.2",
    "gpt-5.6-sol",
    "gpt-5.6-terra",
    "gpt-5.6-luna",
    "gpt-6-astra",
    "deepseek-v4.1-flash",
];

/// Повідомлення з фонового потоку в GUI.
#[derive(Clone)]
enum ProxyMsg {
    Started(u16),
    AlreadyRunning(u16),
    Stopped,
    Failed(String),
    Log(String),
}

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

struct App {
    model: String,
    cred_path: String,
    cred_warning: Option<String>,
    log: Vec<LogEntry>,
    proxy_running: bool,
    busy: bool,
    /// Handle проксі — дозволяє зупинити.
    proxy_handle: Arc<Mutex<Option<proxy::ProxyState>>>,
    rx: mpsc::Receiver<ProxyMsg>,
    tx: mpsc::Sender<ProxyMsg>,
}

impl App {
    fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let (cred, warning) = proxy::ProxyState::find_credential();
        let cred_str = cred.as_ref().map(|p| p.display().to_string()).unwrap_or_default();
        Self {
            model: MODELS[0].to_string(),
            cred_path: cred_str,
            cred_warning: warning,
            log: vec![LogEntry {
                time: now_str(),
                msg: "UMOD Codex GUI готовий".to_string(),
                level: LogLevel::Info,
            }],
            proxy_running: false,
            busy: false,
            proxy_handle: Arc::new(Mutex::new(None)),
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

    fn poll_log(&mut self) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                ProxyMsg::Started(port) => {
                    self.proxy_running = true;
                    self.busy = false;
                    self.add_log(&format!("Проксі запущено на 127.0.0.1:{}", port), LogLevel::Ok);
                }
                ProxyMsg::AlreadyRunning(port) => {
                    self.proxy_running = true;
                    self.busy = false;
                    self.add_log(&format!("Проксі вже працює на 127.0.0.1:{}", port), LogLevel::Ok);
                }
                ProxyMsg::Stopped => {
                    self.proxy_running = false;
                    self.busy = false;
                    self.add_log("Проксі зупинено", LogLevel::Info);
                }
                ProxyMsg::Failed(e) => {
                    self.busy = false;
                    self.add_log(&format!("Помилка: {}", e), LogLevel::Error);
                }
                ProxyMsg::Log(msg) => {
                    self.add_log(&msg, LogLevel::Info);
                }
            }
        }
    }

    fn start_proxy_bg(&mut self) {
        if self.busy { return; }
        self.busy = true;
        let tx = self.tx.clone();
        let handle = self.proxy_handle.clone();
        let cred = self.cred_path.clone();
        self.add_log("Запускаю проксі...", LogLevel::Info);

        std::thread::spawn(move || {
            let port: u16 = 8787;
            let mut state = proxy::ProxyState::new(port);
            if !cred.is_empty() {
                state.cred_path = Some(std::path::PathBuf::from(&cred));
            }

            if state.is_listening() {
                let _ = tx.send(ProxyMsg::AlreadyRunning(port));
                return;
            }

            match state.start() {
                Ok(()) => {
                    let _ = tx.send(ProxyMsg::Started(port));
                    // Зберігаємо handle у Arc<Mutex> — НЕ forget
                    if let Ok(mut guard) = handle.lock() {
                        *guard = Some(state);
                    } else {
                        // Не вдалося зберегти — зупиняємо
                        let _ = state.stop();
                        let _ = tx.send(ProxyMsg::Failed("Не вдалося зберегти handle проксі".to_string()));
                    }
                }
                Err(e) => {
                    let _ = tx.send(ProxyMsg::Failed(e));
                }
            }
        });
    }

    fn stop_proxy(&mut self) {
        if self.busy { return; }
        let tx = self.tx.clone();
        let handle = self.proxy_handle.clone();
        self.busy = true;
        self.add_log("Зупиняю проксі...", LogLevel::Info);

        std::thread::spawn(move || {
            if let Ok(mut guard) = handle.lock() {
                if let Some(mut state) = guard.take() {
                    match state.stop() {
                        Ok(()) => { let _ = tx.send(ProxyMsg::Stopped); }
                        Err(e) => { let _ = tx.send(ProxyMsg::Failed(e)); }
                    }
                } else {
                    let _ = tx.send(ProxyMsg::Log("Проксі не було запущено".to_string()));
                }
            } else {
                let _ = tx.send(ProxyMsg::Failed("Не вдалося отримати доступ до handle".to_string()));
            }
        });
    }

    fn launch_codex(&mut self) {
        if self.busy { return; }
        let port: u16 = 8787;
        let state = proxy::ProxyState::new(port);
        if !state.is_listening() {
            self.add_log("Проксі не працює — запускаю спочатку", LogLevel::Warn);
            self.start_proxy_bg();
            return;
        }
        let model = self.model.clone();
        self.add_log(&format!("Запускаю Codex з моделлю {}...", model), LogLevel::Info);
        match codex::launch(&model) {
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
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll_log();
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(200));

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
                    self.cred_warning = None;
                }
            }
        });

        // Warning про credential
        if let Some(w) = &self.cred_warning {
            ui.horizontal(|ui| {
                ui.colored_label(egui::Color32::from_rgb(220, 180, 60), "⚠");
                ui.label(egui::RichText::new(w).small().color(egui::Color32::from_rgb(220, 180, 60)));
            });
        }

        ui.add_space(8.0);

        // Статус проксі
        ui.horizontal(|ui| {
            let (dot, text) = if self.busy {
                (egui::Color32::from_rgb(220, 180, 60), "запускається...".to_string())
            } else if self.proxy_running {
                (egui::Color32::from_rgb(100, 200, 100), "працює на 127.0.0.1:8787".to_string())
            } else {
                (egui::Color32::from_rgb(200, 80, 80), "не працює".to_string())
            };
            ui.colored_label(dot, "●");
            ui.label(text);
        });

        ui.add_space(8.0);

        // Кнопки
        ui.horizontal(|ui| {
            if ui.add_sized([140.0, 32.0], egui::Button::new("▶ Запустити Codex")).clicked() {
                self.launch_codex();
            }
            if ui.add_sized([100.0, 32.0], egui::Button::new("⟳ Пуск")).clicked() {
                self.start_proxy_bg();
            }
            if ui.add_sized([100.0, 32.0], egui::Button::new("■ Стоп")).clicked() {
                self.stop_proxy();
            }
        });

        ui.add_space(8.0);
        ui.separator();
        ui.label("Журнал:");
        egui::ScrollArea::vertical()
            .auto_shrink([false, true])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for entry in &self.log {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(&entry.time).color(egui::Color32::from_gray(120)).small());
                        ui.label(egui::RichText::new(&entry.msg).color(entry.level.color()));
                    });
                }
            });
    }
}

fn now_str() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    // Локальний час: UTC + 3 (Europe/Kiev)
    let h = ((secs / 3600) + 3) % 24;
    let m = (secs / 60) % 60;
    let s = secs % 60;
    format!("{:02}:{:02}:{:02}", h, m, s)
}

#[cfg(target_os = "windows")]
fn rfd_file_dialog() -> Option<std::path::PathBuf> {
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
