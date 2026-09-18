//! Логіка umod-token-proxy: перевірка, старт, зупинка.

use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};


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

    /// Перевіряє чи проксі слухає на порту.
    /// HTTP-проба /v1/models: 401/403 доводить, що слухає саме проксі,
    /// а не чужий процес. TCP-connect недостатній.
    pub fn is_listening(&self) -> bool {
        let addr = format!("127.0.0.1:{}", self.port);
        if TcpStream::connect_timeout(
            &addr.parse().unwrap(),
            Duration::from_millis(500),
        ).is_err() {
            return false;
        }
        // HTTP-проба: 401/403 = проксі живий, 200 = теж живий
        let url = format!("http://127.0.0.1:{}/v1/models", self.port);
        let cmd = format!(
            "try {{ $r = Invoke-WebRequest -Uri '{}' -TimeoutSec 2 -UseBasicParsing -Headers @{{Authorization='Bearer probe'}}; Write-Output 'OK' }} catch {{ if ($_.Exception.Response) {{ Write-Output 'OK' }} else {{ Write-Output 'FAIL' }} }}",
            url
        );
        if let Ok(output) = Command::new("powershell")
            .args(["-NoProfile", "-Command", &cmd])
            .output()
        {
            let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
            return s == "OK";
        }
        false
    }

    /// Шукає credential-файл у стандартних місцях.
    ///   1. $UMOD_CRED
    ///   2. ~/.umod/credential.env
    ///   3. ~/.umod/*.env (якщо рівно один)
    /// Повертає (path, warning) — warning якщо файлів >1
    pub fn find_credential() -> (Option<PathBuf>, Option<String>) {
        if let Ok(p) = std::env::var("UMOD_CRED") {
            let pb = PathBuf::from(&p);
            if pb.is_file() {
                return (Some(pb), None);
            }
        }
        let home = match std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME")) {
            Ok(h) => h,
            Err(_) => return (None, Some("Не знайшов HOME".to_string())),
        };
        let umod_dir = PathBuf::from(&home).join(".umod");
        let neutral = umod_dir.join("credential.env");
        if neutral.is_file() {
            return (Some(neutral), None);
        }
        if umod_dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&umod_dir) {
                let envs: Vec<_> = entries
                    .filter_map(|e| e.ok())
                    .filter(|e| {
                        e.path().extension().and_then(|x| x.to_str()) == Some("env")
                    })
                    .map(|e| e.path())
                    .collect();
                if envs.len() == 1 {
                    return (Some(envs.into_iter().next().unwrap()), None);
                }
                if envs.len() > 1 {
                    let names: Vec<_> = envs.iter()
                        .filter_map(|p| p.file_name().and_then(|n| n.to_str()).map(|s| s.to_string()))
                        .collect();
                    return (None, Some(format!(
                        "У ~/.umod/ знайдено кілька .env файлів: {}. Вкажіть вручну.",
                        names.join(", ")
                    )));
                }
            }
        }
        (None, Some("Credential-файл не знайдено (~/.umod/*.env)".to_string()))
    }

    /// Шукає umod-token-proxy скрипт.
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
            if let Ok(output) = Command::new("where").arg(name).output() {
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
    pub fn start(&mut self) -> Result<(), String> {
        if self.is_listening() {
            return Ok(());
        }

        let cred = self.cred_path.clone().or_else(|| Self::find_credential().0)
            .ok_or_else(|| "Credential-файл не знайдено (~/.umod/*.env)".to_string())?;
        self.cred_path = Some(cred.clone());

        let script = Self::find_proxy_script()
            .ok_or_else(|| "umod-token-proxy не знайдено".to_string())?;
        let python = Self::find_python()
            .ok_or_else(|| "Python не знайдено на PATH".to_string())?;

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

