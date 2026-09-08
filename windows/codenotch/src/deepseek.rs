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

static REFRESH: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
pub fn request_refresh() { REFRESH.store(true, std::sync::atomic::Ordering::SeqCst); }
pub fn credentials_changed() { GENERATION.fetch_add(1, std::sync::atomic::Ordering::SeqCst); request_refresh(); }

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
        let mut seen_generation = 0;
        let mut retry_at = std::time::Instant::now();
        loop {
            let generation = GENERATION.load(std::sync::atomic::Ordering::SeqCst);
            if generation != seen_generation { last = Balance::default(); seen_generation = generation; }
            if !crate::accounts::enabled("deepseek") {
                last = Balance { status: "disabled".into(), ..Default::default() };
            } else if std::time::Instant::now() < retry_at {
                last.status = "backoff".into();
                last.note = "DeepSeek rate limit; next attempt in five minutes".into();
            } else if let Some(key) = crate::vault::read() {
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
                    Err(ureq::Error::Status(429, _)) => { retry_at = std::time::Instant::now() + Duration::from_secs(300); Err("DeepSeek rate limit; next attempt in five minutes") },
                    _ => Err("DeepSeek balance unavailable"),
                };
                if GENERATION.load(std::sync::atomic::Ordering::SeqCst) != generation { continue; }
                match result {
                    Ok(balances) => last = Balance { status: "ok".into(), note: String::new(), fetched_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64, balances },
                    Err(note) => { last.status = if last.balances.is_empty() { "error" } else { "stale" }.into(); last.note = note.into(); }
                }
            } else {
                last = Balance { status: "needsAuth".into(), note: "Ouvrez Connexions pour enregistrer votre clé API DeepSeek".into(), ..Default::default() };
            }
            {
                let st = app.state::<crate::AppState>();
                let mut current = st.deepseek.lock().unwrap();
                if GENERATION.load(std::sync::atomic::Ordering::SeqCst) != generation { continue; }
                if !crate::accounts::enabled("deepseek") { last = Balance { status: "disabled".into(), ..Default::default() }; }
                *current = last.clone();
                let _ = app.emit("deepseek", &last);
            }
            // Automatic polling is five minutes; manual refresh waits at least 15 seconds.
            for elapsed in 0..300 {
                std::thread::sleep(Duration::from_secs(1));
                if GENERATION.load(std::sync::atomic::Ordering::SeqCst) != generation { break; }
                // A rate-limit response keeps the full five-minute pause.
                if elapsed >= 14 && !last.note.contains("rate limit") && REFRESH.swap(false, std::sync::atomic::Ordering::SeqCst) { break; }
            }
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
