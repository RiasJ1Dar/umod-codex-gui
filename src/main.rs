//! UMOD Codex GUI — лаунчер для запуску Codex CLI з профілем umod.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod codex;
mod proxy;
mod theme;
mod ui;

use eframe::egui;
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};
use ui::Icon;

const MODELS: &[&str] = &[
    "glm-5.2",
    "gpt-5.6-sol",
    "gpt-5.6-terra",
    "gpt-5.6-luna",
    "gpt-6-astra",
    "deepseek-v4.1-flash",
];

/// Порт проксі: змінна середовища або 8787 за замовчуванням.
fn proxy_port() -> u16 {
    std::env::var("UMOD_PROXY_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8787)
}

/// Повідомлення з фонового потоку в GUI.
#[derive(Clone)]
enum ProxyMsg {
    Started(u16),
    AlreadyRunning(u16),
    Stopped,
    Failed(String),
    Log(String),
    /// Результат разової проби порту при старті вікна.
    Probe(bool),
}

#[derive(Clone)]
struct LogEntry {
    time: String,
    msg: String,
    level: LogLevel,
}

#[derive(Clone, Copy, PartialEq)]
enum LogLevel {
    Info,
    Ok,
    Warn,
    Error,
}

impl LogLevel {
    fn color(self) -> egui::Color32 {
        match self {
            LogLevel::Info => theme::TEXT_2,
            LogLevel::Ok => theme::ACCENT_TEXT,
            LogLevel::Warn => theme::WARN,
            LogLevel::Error => theme::ERR_TEXT,
        }
    }
}

/// Стан проксі так, як його бачить вікно. Визначає картку статусу
/// і те, які кнопки активні.
#[derive(Clone, Copy, PartialEq)]
enum Status {
    Running,
    Starting,
    Stopped,
}

struct App {
    model: String,
    cred_path: String,
    cred_warning: Option<String>,
    log: Vec<LogEntry>,
    proxy_running: bool,
    busy: bool,
    proxy_handle: Arc<Mutex<Option<proxy::ProxyState>>>,
    port: u16,
    /// Момент, коли проксі піднявся — для аптайму в картці статусу.
    proxy_since: Option<Instant>,
    /// Час останньої проби порту — показується, коли проксі не працює.
    last_probe: Option<String>,
    rx: mpsc::Receiver<ProxyMsg>,
    tx: mpsc::Sender<ProxyMsg>,
}

impl App {
    fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let (cred, warning) = proxy::ProxyState::find_credential();
        let cred_str = cred
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        let port = proxy_port();

        // Разова проба: інакше вікно показувало б «не працює» навіть тоді,
        // коли проксі вже піднятий попереднім запуском.
        {
            let tx = tx.clone();
            std::thread::spawn(move || {
                let alive = proxy::ProxyState::new(port).is_listening();
                let _ = tx.send(ProxyMsg::Probe(alive));
            });
        }

        let mut log = vec![LogEntry {
            time: now_str(),
            msg: "UMOD Codex GUI готовий".to_string(),
            level: LogLevel::Info,
        }];
        if let Some(name) = cred.as_ref().and_then(|p| p.file_name()) {
            log.push(LogEntry {
                time: now_str(),
                msg: format!("Credential: {}", name.to_string_lossy()),
                level: LogLevel::Info,
            });
        }
        if let Some(w) = &warning {
            log.push(LogEntry {
                time: now_str(),
                msg: w.clone(),
                level: LogLevel::Warn,
            });
        }

        Self {
            model: MODELS[0].to_string(),
            cred_path: cred_str,
            cred_warning: warning,
            log,
            proxy_running: false,
            busy: false,
            proxy_handle: Arc::new(Mutex::new(None)),
            port,
            proxy_since: None,
            last_probe: None,
            rx,
            tx,
        }
    }

    fn status(&self) -> Status {
        if self.busy {
            Status::Starting
        } else if self.proxy_running {
            Status::Running
        } else {
            Status::Stopped
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
                    self.proxy_since = Some(Instant::now());
                    self.add_log(&format!("Проксі запущено на 127.0.0.1:{}", port), LogLevel::Ok);
                }
                ProxyMsg::AlreadyRunning(port) => {
                    self.proxy_running = true;
                    self.busy = false;
                    self.proxy_since.get_or_insert_with(Instant::now);
                    self.add_log(
                        &format!("Проксі вже працює на 127.0.0.1:{}", port),
                        LogLevel::Ok,
                    );
                }
                ProxyMsg::Stopped => {
                    self.proxy_running = false;
                    self.busy = false;
                    self.proxy_since = None;
                    self.last_probe = Some(now_str());
                    self.add_log("Проксі зупинено", LogLevel::Info);
                }
                ProxyMsg::Failed(e) => {
                    self.busy = false;
                    self.add_log(&format!("Помилка: {}", e), LogLevel::Error);
                }
                ProxyMsg::Log(msg) => {
                    self.add_log(&msg, LogLevel::Info);
                }
                ProxyMsg::Probe(alive) => {
                    self.last_probe = Some(now_str());
                    if alive && !self.proxy_running {
                        self.proxy_running = true;
                        self.proxy_since = Some(Instant::now());
                        self.add_log(
                            &format!("Проксі вже працює на 127.0.0.1:{}", self.port),
                            LogLevel::Ok,
                        );
                    } else if !alive {
                        self.add_log(
                            &format!("Проксі 127.0.0.1:{} не відповідає", self.port),
                            LogLevel::Warn,
                        );
                    }
                }
            }
        }
    }

    fn start_proxy_bg(&mut self) {
        if self.busy {
            return;
        }
        self.busy = true;
        let tx = self.tx.clone();
        let handle = self.proxy_handle.clone();
        let cred = self.cred_path.clone();
        let port = self.port;
        self.add_log("Запускаю проксі…", LogLevel::Info);

        std::thread::spawn(move || {
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
                    if let Ok(mut guard) = handle.lock() {
                        *guard = Some(state);
                    } else {
                        let _ = state.stop();
                        let _ = tx.send(ProxyMsg::Failed(
                            "Не вдалося зберегти handle проксі".to_string(),
                        ));
                    }
                }
                Err(e) => {
                    let _ = tx.send(ProxyMsg::Failed(e));
                }
            }
        });
    }

    fn stop_proxy(&mut self) {
        if self.busy {
            return;
        }
        let tx = self.tx.clone();
        let handle = self.proxy_handle.clone();
        self.busy = true;
        self.add_log("Зупиняю проксі…", LogLevel::Info);

        std::thread::spawn(move || {
            if let Ok(mut guard) = handle.lock() {
                if let Some(mut state) = guard.take() {
                    match state.stop() {
                        Ok(()) => {
                            let _ = tx.send(ProxyMsg::Stopped);
                        }
                        Err(e) => {
                            let _ = tx.send(ProxyMsg::Failed(e));
                        }
                    }
                } else {
                    let _ = tx.send(ProxyMsg::Log("Проксі не було запущено".to_string()));
                }
            } else {
                let _ = tx.send(ProxyMsg::Failed(
                    "Не вдалося отримати доступ до handle".to_string(),
                ));
            }
        });
    }

    fn launch_codex(&mut self) {
        if self.busy {
            return;
        }
        let state = proxy::ProxyState::new(self.port);
        if !state.is_listening() {
            self.proxy_running = false;
            self.proxy_since = None;
            self.add_log("Проксі не працює — запускаю спочатку", LogLevel::Warn);
            self.start_proxy_bg();
            return;
        }
        let model = self.model.clone();
        self.add_log(&format!("Запускаю Codex з моделлю {}…", model), LogLevel::Info);
        match codex::launch(&model) {
            Ok(()) => {
                self.add_log(&format!("Codex запущено (модель: {})", model), LogLevel::Ok);
            }
            Err(e) => {
                self.add_log(&format!("Помилка запуску Codex: {}", e), LogLevel::Error);
            }
        }
    }

    fn log_as_text(&self) -> String {
        self.log
            .iter()
            .map(|e| format!("{} {}", e.time, e.msg))
            .collect::<Vec<_>>()
            .join("\n")
    }

    // --- частини вікна -----------------------------------------------------

    /// Смуга з марком і назвою. Стоїть впритул до країв, тому малюється
    /// до того, як вміст отримає свої поля.
    fn header(&self, ui: &mut egui::Ui) {
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), theme::HEADER_H),
            egui::Sense::hover(),
        );
        let painter = ui.painter();
        painter.rect_filled(rect, egui::CornerRadius::ZERO, theme::HEADER);
        painter.hline(
            rect.x_range(),
            rect.max.y - 0.5,
            egui::Stroke::new(1.0, theme::LINE_SOFT),
        );

        let pad = theme::PAD as f32;
        ui::paint_icon(
            painter,
            egui::pos2(rect.min.x + pad + 9.0, rect.center().y),
            18.0,
            Icon::Mark,
            theme::ACCENT,
        );
        theme::tracked_text(
            painter,
            egui::pos2(rect.min.x + pad + 28.0, rect.center().y),
            "UMOD CODEX",
            egui::FontId::proportional(13.0),
            theme::TEXT,
            1.6,
        );
        painter.text(
            egui::pos2(rect.max.x - pad, rect.center().y),
            egui::Align2::RIGHT_CENTER,
            env!("CARGO_PKG_VERSION"),
            egui::FontId::monospace(11.0),
            theme::TEXT_DIM,
        );
    }

    /// Головне, що видно з одного погляду: чи піднятий проксі.
    fn status_card(&self, ui: &mut egui::Ui) {
        let status = self.status();
        let (dot, title, tint, fill, line, sub_tint) = match status {
            Status::Running => (
                theme::ACCENT,
                "ПРОКСІ ПРАЦЮЄ",
                theme::ACCENT_TEXT,
                theme::ACCENT_BG,
                theme::ACCENT_LINE,
                theme::ACCENT_DIM,
            ),
            Status::Starting => (
                theme::WARN,
                "ЗАПУСКАЄТЬСЯ…",
                theme::WARN,
                theme::CARD,
                theme::LINE,
                theme::TEXT_DIM,
            ),
            Status::Stopped => (
                theme::ERR,
                "ПРОКСІ НЕ ПРАЦЮЄ",
                theme::ERR,
                theme::CARD,
                theme::LINE,
                theme::TEXT_DIM,
            ),
        };

        let sub = match status {
            Status::Starting => format!("127.0.0.1:{} — чекаю до 20 с", self.port),
            _ => format!("127.0.0.1:{}", self.port),
        };
        let aside = match status {
            Status::Running => self.proxy_since.map(|t| uptime_str(t.elapsed())),
            Status::Stopped => self
                .last_probe
                .as_ref()
                .map(|t| format!("перевірено {}", t)),
            Status::Starting => None,
        };

        theme::card(fill, line, theme::R_CARD, 16, 14).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let (dot_rect, _) =
                    ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                ui.painter().circle_filled(dot_rect.center(), 5.0, dot);
                ui.add_space(4.0);

                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 3.0;
                    let (t_rect, _) = ui.allocate_exact_size(
                        egui::vec2(220.0, 16.0),
                        egui::Sense::hover(),
                    );
                    theme::tracked_text(
                        ui.painter(),
                        t_rect.left_center(),
                        title,
                        egui::FontId::proportional(13.0),
                        tint,
                        1.0,
                    );
                    ui.label(
                        egui::RichText::new(sub)
                            .monospace()
                            .size(12.0)
                            .color(sub_tint),
                    );
                });

                ui.with_layout(
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        if status == Status::Starting {
                            ui.add(egui::Spinner::new().size(16.0).color(theme::WARN));
                        } else if let Some(aside) = aside {
                            ui.label(
                                egui::RichText::new(aside).size(12.0).color(sub_tint),
                            );
                        }
                    },
                );
            });
        });
    }

    fn model_row(&mut self, ui: &mut egui::Ui) {
        theme::section_label(ui, "МОДЕЛЬ");
        ui.add_space(4.0);
        egui::ComboBox::from_id_salt("model_combo")
            .width(ui.available_width())
            .selected_text(
                egui::RichText::new(&self.model)
                    .size(14.0)
                    .color(theme::TEXT),
            )
            .show_ui(ui, |ui| {
                ui.set_min_width(200.0);
                for m in MODELS {
                    ui.selectable_value(&mut self.model, m.to_string(), *m);
                }
            });
    }

    fn credential_row(&mut self, ui: &mut egui::Ui) {
        theme::section_label(ui, "CREDENTIAL");
        ui.add_space(4.0);

        let warned = self.cred_warning.is_some();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let field_w = ui.available_width() - 52.0;
            ui.scope(|ui| {
                if warned {
                    ui.visuals_mut().widgets.inactive.bg_stroke =
                        egui::Stroke::new(1.0, theme::WARN_LINE);
                    ui.visuals_mut().widgets.hovered.bg_stroke =
                        egui::Stroke::new(1.0, theme::WARN_LINE);
                }
                ui.add(
                    egui::TextEdit::singleline(&mut self.cred_path)
                        .desired_width(field_w)
                        .margin(egui::Margin::symmetric(12, 10))
                        .font(egui::TextStyle::Monospace)
                        .hint_text("шлях до .env не вибрано"),
                );
            });

            let frame = if warned {
                Some((theme::WARN_BG, theme::WARN_LINE))
            } else {
                Some((theme::CARD, theme::LINE))
            };
            let tint = if warned {
                theme::WARN_ICON
            } else {
                theme::TEXT_2
            };
            if ui::icon_button(
                ui,
                egui::vec2(44.0, theme::CONTROL_H),
                Icon::Folder,
                tint,
                frame,
                "Обрати credential-файл",
            ) {
                if let Some(path) = rfd_file_dialog() {
                    self.cred_path = path.display().to_string();
                    self.cred_warning = None;
                }
            }
        });

        // Підказка стоїть біля поля, а не лише в журналі — інакше причина
        // неактивної кнопки лишається невидимою.
        if let Some(w) = self.cred_warning.clone() {
            ui.add_space(5.0);
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 7.0;
                let (r, _) = ui.allocate_exact_size(egui::vec2(13.0, 15.0), egui::Sense::hover());
                ui::paint_icon(ui.painter(), r.center(), 13.0, Icon::Warn, theme::WARN);
                ui.add(
                    egui::Label::new(egui::RichText::new(w).size(11.0).color(theme::WARN)).wrap(),
                );
            });
        } else {
            ui.add_space(5.0);
            let hint = if self.proxy_running {
                "Токен оновлюється за 5 хв до закінчення"
            } else {
                "Знайдено автоматично в ~/.umod/"
            };
            ui.label(egui::RichText::new(hint).size(11.0).color(theme::TEXT_DIM));
        }
    }

    fn actions(&mut self, ui: &mut egui::Ui) {
        let full = ui.available_width();
        let can_launch = !self.busy && !self.cred_path.trim().is_empty();
        if ui::action_button(
            ui,
            can_launch,
            true,
            Icon::Play,
            "Запустити Codex",
            full,
        ) {
            self.launch_codex();
        }

        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            let half = (full - 10.0) / 2.0;
            if ui::action_button(
                ui,
                !self.busy && !self.proxy_running,
                false,
                Icon::Restart,
                "Пуск проксі",
                half,
            ) {
                self.start_proxy_bg();
            }
            if ui::action_button(
                ui,
                !self.busy && self.proxy_running,
                false,
                Icon::Stop,
                "Стоп проксі",
                half,
            ) {
                self.stop_proxy();
            }
        });
    }

    fn journal(&mut self, ui: &mut egui::Ui) {
        // Кнопка копіювання лягає на правий край смуги підпису.
        let head = theme::section_label(ui, "ЖУРНАЛ");
        let copy_rect =
            egui::Rect::from_center_size(egui::pos2(head.max.x - 13.0, head.center().y), egui::vec2(26.0, 20.0));
        let copy = ui.allocate_rect(copy_rect, egui::Sense::click());
        let tint = if copy.hovered() {
            theme::TEXT
        } else {
            theme::TEXT_DIM
        };
        ui::paint_icon(ui.painter(), copy_rect.center(), 14.0, Icon::Copy, tint);
        if copy.on_hover_text("Скопіювати журнал").clicked() {
            ui.ctx().copy_text(self.log_as_text());
        }

        ui.add_space(4.0);
        let box_h = (ui.available_height() - 20.0).max(60.0);
        theme::card(theme::LOG_BG, theme::LINE_SOFT, theme::R_CONTROL, 12, 10).show(ui, |ui| {
            ui.set_min_size(egui::vec2(ui.available_width(), box_h));
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    for entry in &self.log {
                        ui.horizontal_top(|ui| {
                            ui.spacing_mut().item_spacing.x = 10.0;
                            ui.label(
                                egui::RichText::new(&entry.time)
                                    .monospace()
                                    .size(11.0)
                                    .color(theme::TEXT_DIM),
                            );
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(&entry.msg)
                                        .monospace()
                                        .size(12.0)
                                        .color(entry.level.color()),
                                )
                                .wrap(),
                            );
                        });
                    }
                });
        });
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll_log();
        ui.ctx()
            .request_repaint_after(Duration::from_millis(if self.busy { 100 } else { 1000 }));

        // eframe віддає Ui без тла і полів — усе малюємо самі.
        ui.painter()
            .rect_filled(ui.max_rect(), egui::CornerRadius::ZERO, theme::BG);

        self.header(ui);

        egui::Frame::new()
            .inner_margin(egui::Margin::same(theme::PAD))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = theme::GAP;
                self.status_card(ui);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    self.model_row(ui);
                });
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    self.credential_row(ui);
                });
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    self.actions(ui);
                });
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    self.journal(ui);
                });
            });
    }
}

/// «щойно» / «7 хв» / «1 год 12 хв» — аптайм проксі в картці статусу.
fn uptime_str(d: Duration) -> String {
    let secs = d.as_secs();
    if secs < 60 {
        "щойно піднято".to_string()
    } else if secs < 3600 {
        format!("аптайм {} хв", secs / 60)
    } else {
        format!("аптайм {} год {} хв", secs / 3600, (secs % 3600) / 60)
    }
}

/// Локальний час у форматі HH:MM:SS.
/// Використовує GetLocalTime на Windows (без додаткових залежностей),
/// fallback на UTC+3 на інших платформах.
#[cfg(target_os = "windows")]
fn now_str() -> String {
    use std::mem::MaybeUninit;
    #[repr(C)]
    struct SystemTime {
        w_year: u16,
        w_month: u16,
        w_day_of_week: u16,
        w_day: u16,
        w_hour: u16,
        w_minute: u16,
        w_second: u16,
        w_milliseconds: u16,
    }
    extern "system" {
        fn GetLocalTime(lpSystemTime: *mut SystemTime);
    }
    unsafe {
        let mut st = MaybeUninit::<SystemTime>::zeroed();
        GetLocalTime(st.as_mut_ptr());
        let st = st.assume_init();
        format!("{:02}:{:02}:{:02}", st.w_hour, st.w_minute, st.w_second)
    }
}

#[cfg(not(target_os = "windows"))]
fn now_str() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let h = ((secs / 3600) + 3) % 24;
    let m = (secs / 60) % 60;
    let s = secs % 60;
    format!("{:02}:{:02}:{:02}", h, m, s)
}

#[cfg(target_os = "windows")]
fn rfd_file_dialog() -> Option<std::path::PathBuf> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let script = "Add-Type -AssemblyName System.Windows.Forms; $f = New-Object System.Windows.Forms.OpenFileDialog; $f.Filter = 'Credential files (*.env)|*.env|All files (*.*)|*.*'; $f.InitialDirectory = $env:USERPROFILE + '\\.umod'; if ($f.ShowDialog() -eq 'OK') { $f.FileName }";
    let mut cmd = std::process::Command::new("powershell");
    cmd.args(["-NoProfile", "-Command", script]);
    cmd.creation_flags(CREATE_NO_WINDOW);
    if let Ok(output) = cmd.output() {
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
            .with_inner_size([520.0, 680.0])
            .with_min_inner_size([440.0, 560.0])
            .with_title("UMOD Codex"),
        ..Default::default()
    };
    eframe::run_native(
        "UMOD Codex",
        options,
        Box::new(|cc| {
            theme::install(&cc.egui_ctx);
            Ok(Box::new(App::new()))
        }),
    )
}
