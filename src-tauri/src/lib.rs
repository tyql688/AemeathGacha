use arboard::Clipboard;
use chrono::{Duration, Local, NaiveDate, NaiveDateTime, NaiveTime};
use regex::Regex;
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use tauri::Emitter;
use winreg::enums::*;
use winreg::RegKey;

const GACHA_URL_PATTERN: &str = r"(https://aki-gm-resources(?:-oversea)?\.aki-game\.(?:net|com)/aki/gacha/index\.html#/record[^\s]*)";
const LOG_FILE_PATH: &str = r"Client\Saved\Logs\Client.log";
const DEBUG_LOG_PATH: &str = r"Client\Binaries\Win64\ThirdParty\KrPcSdk_Global\KRSDKRes\KRSDKWebView\debug.log";
const URL_EXPIRY_MINUTES: i64 = 30;

#[tauri::command]
async fn scan_gacha_url(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;

    let log = |msg: &str| {
        let _ = app.emit("log-message", msg);
    };

    let mut candidates: Vec<(String, NaiveDateTime, PathBuf)> = Vec::new();
    let mut checked_paths: HashSet<PathBuf> = HashSet::new();

    scan_mui_cache(&mut candidates, &mut checked_paths, &log);
    scan_firewall(&mut candidates, &mut checked_paths, &log);
    scan_registry(&mut candidates, &mut checked_paths, &log);
    scan_common_paths(&mut candidates, &mut checked_paths, &log);

    if candidates.is_empty() {
        log("❌ 未找到有效的抽卡链接。请确认：\n1. 已打开过游戏内的【抽卡历史记录】\n2. 翻阅了几页记录以生成日志");
        return Ok(None);
    }

    candidates.sort_by(|a, b| b.1.cmp(&a.1));

    if let Some((url, timestamp, _source_path)) = candidates.first() {
        let now = Local::now().naive_local();
        let elapsed = now.signed_duration_since(*timestamp);

        if elapsed > Duration::minutes(URL_EXPIRY_MINUTES) {
            log(&format!(
                "⚠️ 链接已过期 (生成于 {}，超过{}分钟)\n请在游戏中重新打开抽卡记录",
                timestamp.format("%Y-%m-%d %H:%M"),
                URL_EXPIRY_MINUTES
            ));
        } else {
            log("✅ 抽卡链接已找到并复制到剪贴板");
        }

        if let Err(e) = clipboard.set_text(url.clone()) {
            log(&format!("⚠️ 复制失败: {}", e));
        }

        return Ok(Some(url.clone()));
    }

    Ok(None)
}

fn check_game_path<F>(
    path_str: &str,
    candidates: &mut Vec<(String, NaiveDateTime, PathBuf)>,
    checked_paths: &mut HashSet<PathBuf>,
    log: &F,
) where
    F: Fn(&str),
{
    let path = match PathBuf::from(path_str).canonicalize() {
        Ok(p) => p,
        Err(_) => PathBuf::from(path_str),
    };

    let path_lower = path_str.to_lowercase();
    if path_lower.contains("onedrive") {
        return;
    }

    if !path.exists() || checked_paths.contains(&path) {
        return;
    }
    checked_paths.insert(path.clone());

    let log_files = vec![LOG_FILE_PATH, DEBUG_LOG_PATH];
    let mut found_directory = false;

    for relative_path in log_files {
        let full_path = path.join(relative_path);
        let parent_dir = full_path.parent().unwrap_or(&path);

        if parent_dir.exists() && !found_directory {
            log(&format!("🔎 发现潜在游戏目录: {}", path_str));
            found_directory = true;
        }

        if !full_path.exists() {
            continue;
        }

        let file = match File::open(&full_path) {
            Ok(f) => f,
            Err(_) => continue,
        };

        let reader = BufReader::new(file);
        let url_regex = Regex::new(GACHA_URL_PATTERN).unwrap();
        let time_regex =
            Regex::new(r"^\[(\d{4})\.(\d{2})\.(\d{2})-(\d{2})\.(\d{2})\.(\d{2}):(\d{3})\]")
                .unwrap();

        let mut found_url: Option<String> = None;
        let mut found_time: Option<NaiveDateTime> = None;

        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => continue,
            };

            if let Some(captures) = url_regex.captures(&line) {
                found_url = captures.get(1).map(|m| m.as_str().to_string());

                if let Some(time_caps) = time_regex.captures(&line) {
                    let year = time_caps[1].parse::<i32>().unwrap_or(2024);
                    let month = time_caps[2].parse::<u32>().unwrap_or(1);
                    let day = time_caps[3].parse::<u32>().unwrap_or(1);
                    let hour = time_caps[4].parse::<u32>().unwrap_or(0);
                    let minute = time_caps[5].parse::<u32>().unwrap_or(0);
                    let second = time_caps[6].parse::<u32>().unwrap_or(0);

                    if let Some(date) = NaiveDate::from_ymd_opt(year, month, day) {
                        if let Some(time) = NaiveTime::from_hms_opt(hour, minute, second) {
                            found_time = Some(NaiveDateTime::new(date, time));
                        }
                    }
                }
            }
        }

        if let Some(url) = found_url {
            let timestamp = found_time.unwrap_or_else(|| {
                if let Ok(metadata) = std::fs::metadata(&full_path) {
                    if let Ok(modified) = metadata.modified() {
                        if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
                            let seconds = duration.as_secs() as i64;
                            if let Some(datetime) = chrono::DateTime::from_timestamp(seconds, 0) {
                                return datetime.naive_local();
                            }
                        }
                    }
                }
                chrono::DateTime::UNIX_EPOCH.naive_utc()
            });

            candidates.push((url, timestamp, full_path.clone()));
        }
    }
}

fn scan_mui_cache<F>(
    candidates: &mut Vec<(String, NaiveDateTime, PathBuf)>,
    checked_paths: &mut HashSet<PathBuf>,
    log: &F,
) where
    F: Fn(&str),
{
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key_path =
        r"Software\Classes\Local Settings\Software\Microsoft\Windows\Shell\MuiCache";

    let key = match hkcu.open_subkey(key_path) {
        Ok(k) => k,
        Err(_) => return,
    };

    for item in key.enum_values() {
        let (name, value) = match item {
            Ok((n, v)) => (n, v),
            Err(_) => continue,
        };

        let value_str = value.to_string();
        let name_lower = name.to_lowercase();

        if !value_str.to_lowercase().contains("wuthering")
            || !name_lower.contains("client-win64-shipping.exe")
        {
            continue;
        }

        let regex = Regex::new(r"[\\/]client[\\/]").unwrap();
        let parts: Vec<&str> = regex.split(&name).collect();

        if parts.len() > 1 {
            check_game_path(parts[0], candidates, checked_paths, log);
        }
    }
}

fn scan_firewall<F>(
    candidates: &mut Vec<(String, NaiveDateTime, PathBuf)>,
    checked_paths: &mut HashSet<PathBuf>,
    log: &F,
) where
    F: Fn(&str),
{
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let key_path = r"SYSTEM\CurrentControlSet\Services\SharedAccess\Parameters\FirewallPolicy\FirewallRules";

    let key = match hklm.open_subkey(key_path) {
        Ok(k) => k,
        Err(_) => return,
    };

    for item in key.enum_values() {
        let value = match item {
            Ok((_, v)) => v,
            Err(_) => continue,
        };

        let value_str = value.to_string().to_lowercase();

        if !value_str.contains("wuthering") || !value_str.contains("client-win64-shipping") {
            continue;
        }

        for part in value_str.split('|') {
            if !part.starts_with("app=") {
                continue;
            }

            let app_path = &part[4..];
            let regex = Regex::new(r"[\\/]client[\\/]").unwrap();
            let parts: Vec<&str> = regex.split(app_path).collect();

            if parts.len() > 1 {
                check_game_path(parts[0], candidates, checked_paths, log);
            }
            break;
        }
    }
}

fn scan_registry<F>(
    candidates: &mut Vec<(String, NaiveDateTime, PathBuf)>,
    checked_paths: &mut HashSet<PathBuf>,
    _log: &F,
) where
    F: Fn(&str),
{
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let subkeys = vec![
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
        r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
    ];

    for subkey_path in subkeys {
        let key = match hklm.open_subkey(subkey_path) {
            Ok(k) => k,
            Err(_) => continue,
        };

        for item_result in key.enum_keys() {
            let item = match item_result {
                Ok(i) => i,
                Err(_) => continue,
            };

            let subkey = match key.open_subkey(&item) {
                Ok(s) => s,
                Err(_) => continue,
            };

            let display_name: String = subkey.get_value("DisplayName").unwrap_or_default();
            if !display_name.to_lowercase().contains("wuthering") {
                continue;
            }

            let install_path: String = subkey.get_value("InstallPath").unwrap_or_default();
            if install_path.is_empty() {
                continue;
            }

            check_game_path(&install_path, candidates, checked_paths, _log);
        }
    }
}

fn scan_common_paths<F>(
    candidates: &mut Vec<(String, NaiveDateTime, PathBuf)>,
    checked_paths: &mut HashSet<PathBuf>,
    log: &F,
) where
    F: Fn(&str),
{
    let drives = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let common_paths = vec![
        r"Wuthering Waves Game",
        r"Wuthering Waves\Wuthering Waves Game",
        r"Games\Wuthering Waves Game",
        r"Games\Wuthering Waves\Wuthering Waves Game",
        r"Program Files\Epic Games\WutheringWavesj3oFh",
        r"Program Files\Epic Games\WutheringWavesj3oFh\Wuthering Waves Game",
        r"Games\WeGameApps\rail_apps\Wuthering Waves(2002137)",
        r"WeGameApps\rail_apps\Wuthering Waves(2002137)",
    ];

    for drive in drives.chars() {
        let drive_path = format!("{}:/", drive);
        if !Path::new(&drive_path).exists() {
            continue;
        }

        for subpath in &common_paths {
            let full_path = Path::new(&drive_path).join(subpath);
            if full_path.exists() {
                check_game_path(
                    full_path.to_str().unwrap_or(""),
                    candidates,
                    checked_paths,
                    log,
                );
            }
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![scan_gacha_url])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
