//! Логіка umod-token-proxy: перевірка, старт, зупинка.

use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};


const DEFAULT_PORT: u16 = 8787;
const PROXY_KEY_PLACEHOLDER: &str = "local-proxy-injects-real-auth";

/// Стан проксі-сервера.
#[derive(Debug)]
pub struct ProxyState {
    pub port: u16,
    pub child: Option<Child>,
    pub cred_path: Option<PathBuf>,
}

impl ProxyState {
    pub fn new(port: u16) -> Self {
        Self { port, child: None, cred_path: None }
    }

    /// Перевіряє чи проксі вже слухає на порту.
    /// 401/403 теж означає що слухає — нам треба лише з'єднання.
    pub fn is_listening(&self) -> bool {
        let addr = format!("127.0.0.1:{}", self.port);
        TcpStream::connect_timeout(
            &addr.parse().unwrap(),
            Duration::from_millis(500),
        ).is_ok()
    }

    /// Шукає credential-файл у стандартних місцях.
    /// Порядок (як у umod-token-proxy):
    ///   1. $UMOD_CRED
    ///   2. ~/.umod/credential.env
    ///   3. ~/.umod/*.env (будь-який один)
    pub fn find_credential() -> Option<PathBuf> {
        if let Ok(p) = std::env::var("UMOD_CRED") {
            let pb = PathBuf::from(&p);
            if pb.is_file() {
                return Some(pb);
            }
        }
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .ok()?;
        let umod_dir = PathBuf::from(&home).join(".umod");
        let neutral = umod_dir.join("credential.env");
        if neutral.is_file() {
            return Some(neutral);
        }
        if umod_dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&umod_dir) {
                let mut envs: Vec<_> = entries
                    .filter_map(|e| e.ok())
                    .filter(|e| {
                        e.path().extension().and_then(|x| x.to_str()) == Some("env")
                    })
                    .map(|e| e.path())
                    .collect();
                if envs.len() == 1 {
                    return Some(envs.pop().unwrap());
                }
            }
        }
        None
    }

    /// Шукає umod-token-proxy скрипт.
    ///   1. $UMOD_PROXY_BIN
    ///   2. ~/Desktop/umod-codex-client/bin/umod-token-proxy
    ///   3. Поруч з exe (./bin/umod-token-proxy)
    pub fn find_proxy_script() -> Option<PathBuf> {
        if let Ok(p) = std::env::var("UMOD_PROXY_BIN") {
            let pb = PathBuf::from(&p);
            if pb.is_file() {
                return Some(pb);
            }
        }
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .ok()?;
        let candidates = [
            PathBuf::from(&home).join("Desktop").join("umod-codex-client").join("bin").join("umod-token-proxy"),
            PathBuf::from(&home).join(".umod").join("bin").join("umod-token-proxy"),
        ];
        for c in &candidates {
            if c.is_file() {
                return Some(c.clone());
            }
        }
        // Поруч з exe
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                let local = dir.join("bin").join("umod-token-proxy");
                if local.is_file() {
                    return Some(local);
                }
            }
        }
        None
    }

    /// Шукає Python виконувач.
    pub fn find_python() -> Option<PathBuf> {
        for name in &["python", "python3", "py"] {
            if let Ok(output) = Command::new("where")
                .arg(name)
                .output()
            {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    if let Some(first) = stdout.lines().next() {
                        let pb = PathBuf::from(first.trim());
                        if pb.is_file() {
                            return Some(pb);
                        }
                    }
                }
            }
        }
        None
    }

    /// Запускає проксі у фоновому процесі.
    /// Повертає Ok(()) якщо проксі піднявся, Err(message) — якщо ні.
    pub fn start(&mut self) -> Result<(), String> {
        if self.is_listening() {
            return Ok(());
        }

        let cred = self.cred_path.clone().or_else(Self::find_credential)
            .ok_or_else(|| "Credential-файл не знайдено (~/.umod/*.env)".to_string())?;
        self.cred_path = Some(cred.clone());

        let script = Self::find_proxy_script()
            .ok_or_else(|| "umod-token-proxy не знайдено".to_string())?;
        let python = Self::find_python()
            .ok_or_else(|| "Python не знайдено на PATH".to_string())?;

        // Лог проксі — у TEMP
        let log_dir = std::env::temp_dir();
        let log_out = log_dir.join("umod-proxy.log");
        let log_err = log_dir.join("umod-proxy.err");

        let stdout = std::fs::File::create(&log_out)
            .map_err(|e| format!("Не вдалося створити {}: {}", log_out.display(), e))?;
        let stderr = std::fs::File::create(&log_err)
            .map_err(|e| format!("Не вдалося створити {}: {}", log_err.display(), e))?;

        let mut cmd = Command::new(&python);
        cmd.arg(&script)
           .env("UMOD_CRED", &cred)
           .env("UMOD_PROXY_PORT", self.port.to_string())
           .env("UMOD_PROXY_KEY", PROXY_KEY_PLACEHOLDER)
           .stdin(Stdio::null())
           .stdout(Stdio::from(stdout))
           .stderr(Stdio::from(stderr));

        let child = cmd.spawn()
            .map_err(|e| format!("Не вдалося запустити проксі: {}", e))?;

        self.child = Some(child);

        // Чекаємо поки проксі почне слухати (до 20с)
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(20) {
            if self.is_listening() {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(500));
        }

        Err("Проксі не піднявся за 20 секунд (див. %TEMP%\\umod-proxy.err)".to_string())
    }

    /// Зупиняє проксі.
    pub fn stop(&mut self) -> Result<(), String> {
        if let Some(mut child) = self.child.take() {
            child.kill().map_err(|e| format!("Не вдалося зупинити проксі: {}", e))?;
            let _ = child.wait();
        }
        Ok(())
    }
}

impl Drop for ProxyState {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

