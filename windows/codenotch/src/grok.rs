//! Balance only. The key is read from one Windows Credential Manager entry,
//! never sent to the WebView, logged, persisted to files, or sent to another host.
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};

const ENDPOINT: &str = "https://management-api.x.ai/v1/billing/teams";

pub use crate::deepseek::Balance;
use crate::deepseek::Amount;

static REFRESH: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
pub fn request_refresh() { REFRESH.store(true, std::sync::atomic::Ordering::SeqCst); }
pub fn credentials_changed() { GENERATION.fetch_add(1, std::sync::atomic::Ordering::SeqCst); request_refresh(); }

// xAI uses an inverted ledger in string USD cents (a $10 top-up is -1000).
// Show posted credits, not a fabricated live remaining balance or subscription quota.
fn parse(v: &serde_json::Value) -> Result<Vec<Amount>, &'static str> {
    let raw = v.pointer("/total/val").and_then(|x| x.as_str()).ok_or("Invalid Grok balance response")?;
    let digits = raw.strip_prefix('-').unwrap_or(raw);
    if digits.is_empty() || digits.len() > 18 || !digits.bytes().all(|b| b.is_ascii_digit()) { return Err("Invalid Grok balance amount"); }
    let cents = -raw.parse::<i128>().map_err(|_| "Invalid Grok balance amount")?;
    let abs = cents.abs();
    let total = format!("{}{}.{:02}", if cents < 0 { "-" } else { "" }, abs / 100, abs % 100);
    Ok(vec![Amount { currency: "USD".into(), total, granted: String::new(), topped_up: String::new() }])
}

pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        let mut last = Balance::default();
        let mut seen_generation = 0;
        let mut retry_at = std::time::Instant::now();
        loop {
            let generation = GENERATION.load(std::sync::atomic::Ordering::SeqCst);
            if generation != seen_generation { last = Balance::default(); seen_generation = generation; }
            if !crate::accounts::enabled("grok") {
                last = Balance { status: "disabled".into(), ..Default::default() };
            } else if std::time::Instant::now() < retry_at {
                last.status = "backoff".into();
                last.note = "Grok rate limit; next attempt in five minutes".into();
            } else if let Some(packed) = crate::grok_vault::read() {
                let Some((team, key)) = packed.split_once('\n') else { continue; };
                let endpoint = format!("{ENDPOINT}/{team}/prepaid/balance");
                let response = ureq::AgentBuilder::new().redirects(0).timeout(Duration::from_secs(15)).build()
                    .get(&endpoint).set("Authorization", &format!("Bearer {key}"))
                    .set("Accept", "application/json").call();
                let result = match response {
                    Ok(r) if r.status() == 200 => {
                        use std::io::Read;
                        let mut body = String::new();
                        r.into_reader().take(1048576).read_to_string(&mut body)
                            .map_err(|_| "Unreadable balance response")
                            .and_then(|_| serde_json::from_str::<serde_json::Value>(&body).map_err(|_| "Invalid balance response"))
                            .and_then(|v| parse(&v))
                    },
                    Err(ureq::Error::Status(401 | 403, _)) => Err("Clé Management Grok refusée ; vérifiez les droits de lecture de facturation et l’équipe"),
                    Err(ureq::Error::Status(429, _)) => { retry_at = std::time::Instant::now() + Duration::from_secs(300); Err("Grok rate limit; next attempt in five minutes") },
                    _ => Err("Grok balance unavailable"),
                };
                if GENERATION.load(std::sync::atomic::Ordering::SeqCst) != generation { continue; }
                match result {
                    Ok(balances) => last = Balance { status: "ok".into(), note: "Crédits prépayés comptabilisés ; les dépenses récentes peuvent ne pas encore être déduites.".into(), fetched_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64, balances },
                    Err(note) => { last.status = if last.balances.is_empty() { "error" } else { "stale" }.into(); last.note = note.into(); }
                }
            } else {
                last = Balance { status: "needsAuth".into(), note: "Ouvrez Connexions pour enregistrer votre clé Management xAI et votre identifiant d’équipe".into(), ..Default::default() };
            }
            {
                let st = app.state::<crate::AppState>();
                let mut current = st.grok.lock().unwrap();
                if GENERATION.load(std::sync::atomic::Ordering::SeqCst) != generation { continue; }
                if !crate::accounts::enabled("grok") { last = Balance { status: "disabled".into(), ..Default::default() }; }
                *current = last.clone();
                let _ = app.emit("grok", &last);
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
    #[test]
    fn converts_inverted_usd_cents_exactly() {
        for (raw, expected) in [("-1000","10.00"),("-1","0.01"),("0","0.00"),("101","-1.01"),("-123456789012345678","1234567890123456.78")] {
            let rows = super::parse(&serde_json::json!({"total":{"val":raw}})).unwrap();
            assert_eq!(rows[0].total, expected);
        }
    }
    #[test]
    fn rejects_missing_or_untrusted_amounts() {
        assert!(super::parse(&serde_json::json!({})).is_err());
        for raw in ["", "NaN", "1.25", "<script>", "1e9", "--1", "999999999999999999999"] {
            assert!(super::parse(&serde_json::json!({"total":{"val":raw}})).is_err());
        }
    }
}
