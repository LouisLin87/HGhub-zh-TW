//! Optional, bounded playback diagnostics. Never write media URLs, credentials or response bodies.
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;

const MAX_FILE: u64 = 2 * 1024 * 1024;
static ENABLED: AtomicBool = AtomicBool::new(false);
static SEQ: AtomicU64 = AtomicU64::new(0);
static LOG: Lazy<Mutex<Journal>> = Lazy::new(|| Mutex::new(Journal::default()));

#[derive(Default)]
struct Journal {
    dir: Option<PathBuf>,
    enabled: bool,
    error: Option<String>,
}

#[derive(Serialize)]
pub struct Status {
    enabled: bool,
    path: Option<String>,
    error: Option<String>,
}

#[derive(Deserialize)]
pub struct Entry {
    event: String,
    data: Value,
}

pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

pub fn resource_id(raw: &str) -> String {
    let hash = raw
        .as_bytes()
        .iter()
        .fold(2166136261u32, |h, b| (h ^ *b as u32).wrapping_mul(16777619));
    format!("r{hash:08x}")
}

pub fn error_kind(error: &str) -> &'static str {
    if error.contains("禁止")
        || error.contains("僅允許")
        || error.contains("只允许")
        || error.contains("控制字元")
        || error.contains("控制字符")
    {
        if error.contains("降級") || error.contains("降级") {
            "https_downgrade_denied"
        } else if error.contains("內部網路") || error.contains("内网") || error.contains("保留地址")
        {
            "private_address_denied"
        } else {
            "target_policy_denied"
        }
    } else if error.contains("DNS") {
        "dns_error"
    } else if error.contains("逾時")
        || error.contains("超时")
        || error.to_ascii_lowercase().contains("timeout")
    {
        "timeout"
    } else if error.starts_with("HTTP ") {
        "http_error"
    } else if error.contains("重新導向") || error.contains("重定向") {
        "redirect_error"
    } else {
        "transport_error"
    }
}

// Defense in depth for frontend entries: no arbitrary URL/message/header fields reach disk.
fn scrub(value: &Value, depth: usize) -> Value {
    if depth > 5 {
        return Value::Null;
    }
    match value {
        Value::String(s) => {
            if s.contains("://") || s.to_ascii_lowercase().contains("%3a") {
                json!("[redacted]")
            } else {
                json!(s.chars().take(160).collect::<String>())
            }
        }
        Value::Array(items) => {
            Value::Array(items.iter().take(24).map(|v| scrub(v, depth + 1)).collect())
        }
        Value::Object(map) => Value::Object(
            map.iter()
                .take(64)
                .filter_map(|(key, v)| {
                    let k = key.to_ascii_lowercase();
                    if [
                        "url",
                        "src",
                        "token",
                        "password",
                        "cookie",
                        "authorization",
                        "body",
                        "message",
                        "reason",
                    ]
                    .iter()
                    .any(|s| k.contains(s))
                    {
                        None
                    } else {
                        Some((key.chars().take(64).collect(), scrub(v, depth + 1)))
                    }
                })
                .collect(),
        ),
        _ => value.clone(),
    }
}

impl Journal {
    fn status(&self) -> Status {
        Status {
            enabled: self.enabled,
            path: self
                .dir
                .as_ref()
                .map(|d| d.join("playback.log").display().to_string()),
            error: self.error.clone(),
        }
    }

    fn append(&mut self, event: &str, data: Value, limit: u64) -> Result<(), String> {
        let dir = self.dir.as_ref().ok_or("紀錄目錄尚未初始化")?;
        let path = dir.join("playback.log");
        let ts = crate::logging::utc_timestamp()?;
        let line = json!({ "ts": ts, "pid": std::process::id(), "seq": SEQ.fetch_add(1, Ordering::Relaxed), "event": event, "strict": crate::network_policy::strict_security(), "data": scrub(&data, 0) }).to_string() + "\n";
        if fs::metadata(&path).map(|m| m.len()).unwrap_or(0) + line.len() as u64 > limit {
            let previous = dir.join("playback.previous.log");
            if previous.exists() {
                fs::remove_file(&previous).map_err(|_| "無法輪轉診斷紀錄")?;
            }
            if path.exists() {
                fs::rename(&path, previous).map_err(|_| "無法輪轉診斷紀錄")?;
            }
        }
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut f| f.write_all(line.as_bytes()))
            .map_err(|_| "無法寫入診斷紀錄".to_string())
    }

    fn set(&mut self, on: bool) -> Result<(), String> {
        let dir = self.dir.as_ref().ok_or("紀錄目錄尚未初始化")?;
        if on {
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(dir.join("playback.log"))
                .map_err(|_| "無法建立診斷紀錄")?;
        }
        let temp = dir.join("playback-diagnostics.json.tmp");
        fs::write(&temp, json!({"enabled": on}).to_string())
            .and_then(|_| fs::rename(&temp, dir.join("playback-diagnostics.json")))
            .map_err(|_| "無法儲存紀錄開關")?;
        self.enabled = on;
        self.error = None;
        if let Err(e) = self.append("diagnostics.toggle", json!({"enabled": on, "version": env!("CARGO_PKG_VERSION"), "os": std::env::consts::OS}), MAX_FILE) { self.error = Some(e); }
        Ok(())
    }
}

pub fn bind(dir: &Path) {
    if let Ok(mut log) = LOG.lock() {
        log.dir = Some(dir.to_path_buf());
        log.enabled = fs::read_to_string(dir.join("playback-diagnostics.json"))
            .ok()
            .and_then(|s| serde_json::from_str::<Value>(&s).ok())
            .and_then(|v| v["enabled"].as_bool())
            .unwrap_or(false);
        ENABLED.store(log.enabled, Ordering::Relaxed);
        if log.enabled {
            if let Err(e) = log.append(
                "process.start",
                json!({"version": env!("CARGO_PKG_VERSION"), "os": std::env::consts::OS}),
                MAX_FILE,
            ) {
                log.error = Some(e);
            }
        }
    }
}

pub fn record(event: &str, data: Value) {
    if !enabled() {
        return;
    }
    if let Ok(mut log) = LOG.lock() {
        if log.enabled {
            if let Err(e) = log.append(event, data, MAX_FILE) {
                log.error = Some(e);
            }
        }
    }
}

#[tauri::command]
pub fn playback_log_status() -> Result<Status, String> {
    Ok(LOG.lock().map_err(|_| "紀錄鎖失敗")?.status())
}

#[tauri::command]
pub fn playback_log_set(enabled: bool) -> Result<Status, String> {
    let mut log = LOG.lock().map_err(|_| "紀錄鎖失敗")?;
    let result = log.set(enabled);
    ENABLED.store(log.enabled, Ordering::Relaxed);
    result?;
    Ok(log.status())
}

#[tauri::command]
pub fn playback_log_write(entries: Vec<Entry>) -> Result<(), String> {
    if !enabled() {
        return Ok(());
    }
    if entries.len() > 128 {
        return Err("紀錄批次過大".into());
    }
    let mut log = LOG.lock().map_err(|_| "紀錄鎖失敗")?;
    if !log.enabled {
        return Ok(());
    }
    for entry in entries {
        if entry.event.len() > 80
            || !entry
                .event
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
            || entry.data.to_string().len() > 8192
        {
            return Err("紀錄條目無效".into());
        }
        log.append(&format!("web.{}", entry.event), entry.data, MAX_FILE)?;
    }
    Ok(())
}

#[tauri::command]
pub fn playback_log_read() -> Result<String, String> {
    let log = LOG.lock().map_err(|_| "紀錄鎖失敗")?;
    let dir = log.dir.as_ref().ok_or("紀錄目錄尚未初始化")?;
    let mut text = String::new();
    for name in ["playback.previous.log", "playback.log"] {
        let path = dir.join(name);
        if path.exists() {
            if fs::metadata(&path).map_err(|_| "無法讀取紀錄")?.len() > MAX_FILE + 16384 {
                return Err("紀錄檔案超出大小限制".into());
            }
            text.push_str(&fs::read_to_string(path).map_err(|_| "無法讀取紀錄")?);
        }
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn localized_and_legacy_errors_keep_diagnostic_categories() {
        for (localized, legacy, expected) in [
            (
                "目標僅允許 HTTPS",
                "目标只允许 HTTPS",
                "target_policy_denied",
            ),
            ("目標含控制字元", "目标含控制字符", "target_policy_denied"),
            (
                "禁止 HTTPS 降級重新導向",
                "禁止 HTTPS 降级重定向",
                "https_downgrade_denied",
            ),
            ("禁止存取內部網路", "禁止访问内网", "private_address_denied"),
            ("請求逾時", "请求超时", "timeout"),
            (
                "重新導向次數超過限制",
                "重定向次数超过限制",
                "redirect_error",
            ),
        ] {
            assert_eq!(error_kind(localized), expected);
            assert_eq!(error_kind(legacy), expected);
        }
    }

    #[test]
    fn commands_stop_writes_when_disabled_and_read_retained_logs() {
        let dir = std::env::temp_dir().join(format!(
            "hghub-playback-commands-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).unwrap();
        bind(&dir);
        assert!(!playback_log_status().unwrap().enabled);
        record("test.off", json!({"n": 1}));
        assert!(!dir.join("playback.log").exists());
        playback_log_set(true).unwrap();
        playback_log_write(vec![Entry {
            event: "seek.request".into(),
            data: json!({"player": "p1", "seek": 1, "target": 30}),
        }])
        .unwrap();
        playback_log_set(false).unwrap();
        let size = fs::metadata(dir.join("playback.log")).unwrap().len();
        playback_log_write(vec![Entry {
            event: "seek.request".into(),
            data: json!({"seek": 2}),
        }])
        .unwrap();
        record("test.off", json!({"n": 2}));
        assert_eq!(fs::metadata(dir.join("playback.log")).unwrap().len(), size);
        let text = playback_log_read().unwrap();
        assert!(text.contains("web.seek.request"));
        assert!(!text.contains("test.off"));
        bind(&dir);
        assert!(!enabled());
        let mut log = LOG.lock().unwrap();
        *log = Journal::default();
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn log_redacts_urls_credentials_and_limits_depth() {
        let value = scrub(&json!({"resource": "r123", "url": "https://a/?token=secret", "token": "secret", "nested": {"message": "secret", "detail": "hg://localhost/seg?u=secret"}}), 0).to_string();
        assert!(!value.contains("secret"));
        assert!(value.contains("r123"));
        assert_eq!(resource_id("hello"), "r4f9f2cab");
    }

    #[test]
    fn logging_rotates_and_persists_without_changing_mode_on_save_failure() {
        let dir = std::env::temp_dir().join(format!(
            "hghub-playback-log-{}",
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).unwrap();
        let mut log = Journal {
            dir: Some(dir.clone()),
            ..Default::default()
        };
        log.set(true).unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(
                &fs::read_to_string(dir.join("playback-diagnostics.json")).unwrap()
            )
            .unwrap()["enabled"],
            true
        );
        for _ in 0..5 {
            log.append("test", json!({"count": 1}), 400).unwrap();
        }
        assert!(dir.join("playback.previous.log").exists());
        for name in ["playback.log", "playback.previous.log"] {
            let text = fs::read_to_string(dir.join(name)).unwrap();
            assert!(text.len() <= 400);
            for line in text.lines() {
                assert!(serde_json::from_str::<Value>(line).unwrap()["ts"]
                    .as_str()
                    .unwrap()
                    .ends_with('Z'));
            }
        }
        log.dir = Some(dir.join("missing"));
        assert!(log.set(false).is_err());
        assert!(log.enabled);
        fs::remove_dir_all(dir).unwrap();
    }
}
