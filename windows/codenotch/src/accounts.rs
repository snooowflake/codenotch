//! Provider switches and a write-only secret form. No secret-returning IPC.
use std::sync::atomic::{AtomicU8, Ordering};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};
const IDS: [&str; 5] = ["claude", "codex", "cursor", "gemini", "deepseek"];
static ENABLED: AtomicU8 = AtomicU8::new(31);

fn bit(id: &str) -> u8 { IDS.iter().position(|p| *p == id).map(|i| 1 << i).unwrap_or(0) }
pub fn enabled(id: &str) -> bool { ENABLED.load(Ordering::SeqCst) & bit(id) != 0 }
pub fn initialize(disabled: &[String]) {
    ENABLED.store(disabled.iter().fold(31, |mask, id| mask & !bit(id)), Ordering::SeqCst);
}
pub fn hidden() -> crate::usage::UsageSnapshot {
    crate::usage::UsageSnapshot { status: "disabled".into(), ..Default::default() }
}
fn settings_only(window: &WebviewWindow) -> Result<(), String> {
    if window.label() == "settings" { Ok(()) } else { Err("Open Connections to change account settings".into()) }
}
#[derive(Serialize)]
pub struct Account {
    id: &'static str,
    enabled: bool,
    status: String,
    key_saved: bool,
}
#[tauri::command]
pub fn get_accounts(app: AppHandle) -> Vec<Account> {
    let st = app.state::<crate::AppState>();
    IDS.iter().map(|&id| {
        let status = match id {
            "claude" => st.usage.lock().unwrap().status.clone(),
            "codex" => st.codex.lock().unwrap().status.clone(),
            "cursor" => st.cursor.lock().unwrap().status.clone(),
            "gemini" => st.antigravity.lock().unwrap().status.clone(),
            _ => st.deepseek.lock().unwrap().status.clone(),
        };
        Account { id, enabled: enabled(id), status, key_saved: id == "deepseek" && crate::vault::exists() }
    }).collect()
}
pub fn wake() {
    crate::usage::request_refresh();
    crate::codex::request_refresh();
    crate::cursor::request_refresh();
    crate::antigravity::request_refresh();
    crate::deepseek::request_refresh();
}
#[tauri::command]
pub fn set_provider_enabled(window: WebviewWindow, app: AppHandle, provider: String, on: bool) -> Result<(), String> {
    settings_only(&window)?;
    let mask = bit(&provider);
    if mask == 0 { return Err("Unknown provider".into()); }
    let st = app.state::<crate::AppState>();
    {
        let mut cfg = st.cfg.lock().unwrap();
        let mut next = cfg.clone();
        next.disabled_providers.retain(|p| p != &provider);
        if !on { next.disabled_providers.push(provider.clone()); }
        let path = crate::config::config_path();
        std::fs::create_dir_all(path.parent().ok_or("Missing configuration directory")?).map_err(|_| "Cannot save settings")?;
        std::fs::write(path, serde_json::to_vec_pretty(&next).map_err(|_| "Cannot encode settings")?).map_err(|_| "Cannot save settings")?;
        *cfg = next;
        if on { ENABLED.fetch_or(mask, Ordering::SeqCst); } else { ENABLED.fetch_and(!mask, Ordering::SeqCst); }
    }
    if !on {
        let (slot, event, file) = match provider.as_str() {
            "claude" => (Some(&st.usage), "usage", "usage.json"),
            "codex" => (Some(&st.codex), "codex", "codex.json"),
            "cursor" => (Some(&st.cursor), "cursor", "cursor.json"),
            "gemini" => (Some(&st.antigravity), "antigravity", "antigravity.json"),
            _ => (None, "deepseek", ""),
        };
        if let Some(slot) = slot {
            let mut value = slot.lock().unwrap();
            // Keep the server backoff even when hiding the previous reading.
            let mut empty = hidden();
            empty.backoff_until = value.backoff_until;
            *value = empty.clone();
            let _ = std::fs::write(crate::config::config_path().with_file_name(file), serde_json::to_vec(&empty).unwrap_or_default());
            let _ = app.emit(event, &empty);
        } else {
            let empty = crate::deepseek::Balance { status: "disabled".into(), ..Default::default() };
            let mut current = st.deepseek.lock().unwrap();
            *current = empty.clone();
            let _ = app.emit("deepseek", &empty);
        }
    }
    wake();
    let _ = app.emit("accounts-changed", ());
    Ok(())
}
#[tauri::command]
pub fn open_settings(app: AppHandle) -> Result<(), String> {
    let w = app.get_webview_window("settings").ok_or("Connections window unavailable")?;
    w.show().map_err(|_| "Cannot show Connections")?;
    w.set_focus().map_err(|_| "Cannot focus Connections")?;
    Ok(())
}
#[tauri::command]
pub fn close_settings(window: WebviewWindow) -> Result<(), String> {
    settings_only(&window)?;
    window.hide().map_err(|_| "Cannot close Connections".into())
}
#[tauri::command]
pub fn refresh_accounts(window: WebviewWindow) -> Result<(), String> {
    settings_only(&window)?;
    wake();
    Ok(())
}
#[tauri::command]
pub fn save_deepseek_key(window: WebviewWindow, app: AppHandle, mut key: String) -> Result<(), String> {
    settings_only(&window)?;
    let result = crate::vault::save(key.trim());
    // Clear the owned UTF-8 buffer after the OS has copied it.
    unsafe { key.as_bytes_mut().fill(0); }
    result?;
    crate::deepseek::credentials_changed();
    let _ = app.emit("accounts-changed", ());
    Ok(())
}
#[tauri::command]
pub fn forget_deepseek_key(window: WebviewWindow, app: AppHandle) -> Result<(), String> {
    settings_only(&window)?;
    crate::vault::delete()?;
    crate::deepseek::credentials_changed();
    let st = app.state::<crate::AppState>();
    let mut current = st.deepseek.lock().unwrap();
    *current = crate::deepseek::Balance::default();
    let _ = app.emit("deepseek", crate::deepseek::Balance::default());
    let _ = app.emit("accounts-changed", ());
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn provider_identifiers_are_allowlisted() {
        assert_eq!(bit("deepseek"), 16);
        assert_eq!(bit("../auth.json"), 0);
        assert_eq!(bit("unknown"), 0);
    }
}
