//! Запуск Codex CLI з профілем umod.

use std::process::Command;
use std::path::PathBuf;

const PROXY_KEY_PLACEHOLDER: &str = "local-proxy-injects-real-auth";

/// Шукає codex виконувач.
pub fn find_codex() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("UMOD_CODEX_BIN") {
        let pb = PathBuf::from(&p);
        if pb.is_file() {
            return Some(pb);
        }
    }
    for name in &["codex.cmd", "codex", "codex.exe"] {
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

/// Запускає Codex у новому вікні термінала.
///
/// Codex не має прапорця --port: адреса проксі береться з config.toml.
/// Порт у GUI керує лише проксі, не Codex.
pub fn launch(model: &str) -> Result<(), String> {
    let codex = find_codex()
        .ok_or_else(|| "Codex не знайдено на PATH".to_string())?;

    let proxy_key = std::env::var("UMOD_PROXY_KEY")
        .unwrap_or_else(|_| PROXY_KEY_PLACEHOLDER.to_string());

    #[cfg(target_os = "windows")]
    {
        let cmd_str = format!(
            "start \"Codex UMOD\" \"{}\" --profile umod --model {}",
            codex.display(),
            model
        );
        Command::new("cmd")
            .args(["/c", &cmd_str])
            .env("UMOD_PROXY_KEY", &proxy_key)
            .spawn()
            .map_err(|e| format!("Не вдалося запустити Codex: {}", e))?;
        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .args(["-a", "Terminal", "--"])
            .arg(&codex)
            .args(["--profile", "umod", "--model", model])
            .env("UMOD_PROXY_KEY", &proxy_key)
            .spawn()
            .map_err(|e| format!("Не вдалося запустити Codex: {}", e))?;
        Ok(())
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Command::new("x-terminal-emulator")
            .arg("-e")
            .arg(&codex)
            .args(["--profile", "umod", "--model", model])
            .env("UMOD_PROXY_KEY", &proxy_key)
            .spawn()
            .map_err(|e| format!("Не вдалося запустити Codex: {}", e))?;
        Ok(())
    }
}
