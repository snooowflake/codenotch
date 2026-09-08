//! Balance only. The key is read from one Windows Credential Manager entry,
//! never sent to the WebView, logged, persisted to files, or sent to another host.
use serde::Serialize;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};

const ENDPOINT: &str = "https://api.deepseek.com/user/balance";

#[derive(Clone, Default, Serialize)]
pub struct Balance {
    pub status: String,
    pub note: String,
    pub fetched_at: u64,
    pub balances: Vec<Amount>,
}

#[derive(Clone, Serialize)]
pub struct Amount {
    pub currency: String,
    pub total: String,
    pub granted: String,
    pub topped_up: String,
}

#[cfg(windows)]
fn read_key() -> Option<String> {
    use windows::core::PCWSTR;
    use windows::Win32::Security::Credentials::{CredFree, CredReadW, CREDENTIALW, CRED_TYPE_GENERIC};
    let target: Vec<u16> = "codenotch:deepseek".encode_utf16().chain(Some(0)).collect();
    let mut ptr: *mut CREDENTIALW = std::ptr::null_mut();
    unsafe {
        if CredReadW(PCWSTR(target.as_ptr()), CRED_TYPE_GENERIC, 0, &mut ptr).is_err() || ptr.is_null() {
            return None;
        }
        let c = &*ptr;
        let key = if !c.CredentialBlob.is_null() && (1..=4096).contains(&c.CredentialBlobSize) {
            String::from_utf8(std::slice::from_raw_parts(c.CredentialBlob, c.CredentialBlobSize as usize).to_vec()).ok()
        } else { None };
        CredFree(ptr as *const core::ffi::c_void);
        key.filter(|k| k.starts_with("sk-") && k.len() <= 512 && k.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'))
    }
}
#[cfg(not(windows))]
fn read_key() -> Option<String> { None }


fn amount(v: &serde_json::Value, field: &str) -> Result<String, &'static str> {
    let s = v.get(field).and_then(|x| x.as_str()).ok_or("Invalid balance response")?;
    if s.len() > 32 || !s.bytes().all(|b| b.is_ascii_digit() || b == b'.' || b == b'-') || s.parse::<f64>().map(|n| !n.is_finite()).unwrap_or(true) {
        return Err("Invalid balance amount");
    }
    Ok(s.into())
}

fn parse(v: &serde_json::Value) -> Result<Vec<Amount>, &'static str> {
    let rows = v.get("balance_infos").and_then(|x| x.as_array()).ok_or("Invalid balance response")?;
    if rows.is_empty() || rows.len() > 2 { return Err("Invalid balance response"); }
    rows.iter().map(|row| {
        let currency = row.get("currency").and_then(|x| x.as_str()).ok_or("Missing currency")?;
        if currency != "USD" && currency != "CNY" { return Err("Unsupported currency"); }
        Ok(Amount { currency: currency.into(), total: amount(row, "total_balance")?, granted: amount(row, "granted_balance")?, topped_up: amount(row, "topped_up_balance")? })
    }).collect()
}

pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        let mut last = Balance::default();
        loop {
            if let Some(key) = read_key() {
                let response = ureq::AgentBuilder::new().redirects(0).timeout(Duration::from_secs(15)).build()
                    .get(ENDPOINT).set("Authorization", &format!("Bearer {key}"))
                    .set("Accept", "application/json").call();
                let result = match response {
                    Ok(r) if r.status() == 200 => {
                        use std::io::Read;
                        let mut body = String::new();
                        r.into_reader().take(65536).read_to_string(&mut body)
                            .map_err(|_| "Unreadable balance response")
                            .and_then(|_| serde_json::from_str::<serde_json::Value>(&body).map_err(|_| "Invalid balance response"))
                            .and_then(|v| parse(&v))
                    },
                    Err(ureq::Error::Status(401 | 403, _)) => Err("DeepSeek API key refused; update it locally"),
                    Err(ureq::Error::Status(429, _)) => Err("DeepSeek rate limit; next attempt in five minutes"),
                    _ => Err("DeepSeek balance unavailable"),
                };
                match result {
                    Ok(balances) => last = Balance { status: "ok".into(), note: String::new(), fetched_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64, balances },
                    Err(note) => { last.status = if last.balances.is_empty() { "error" } else { "stale" }.into(); last.note = note.into(); }
                }
            } else {
                last = Balance { status: "needsAuth".into(), note: "Configure the API key locally with Setup-DeepSeek.ps1".into(), ..Default::default() };
            }
            *app.state::<crate::AppState>().deepseek.lock().unwrap() = last.clone();
            let _ = app.emit("deepseek", &last);
            // Keep a full five-minute minimum between API calls, including failures.
            for _ in 0..300 { std::thread::sleep(Duration::from_secs(1)); }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_balance_without_inventing_usage_percent() {
        let rows = parse(&serde_json::json!({"balance_infos":[{"currency":"USD","total_balance":"12.34","granted_balance":"2.00","topped_up_balance":"10.34"}]})).unwrap();
        assert_eq!(rows[0].total, "12.34");
    }
    #[test]
    fn rejects_html_and_nonfinite_amounts() {
        for bad in ["<img src=x onerror=alert(1)>", "NaN", "Infinity", "1e999"] {
            assert!(amount(&serde_json::json!({"total_balance":bad}), "total_balance").is_err());
        }
    }
}
