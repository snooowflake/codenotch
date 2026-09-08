//! Privacy-preserving diagnosis: no conversation or other-account reads.
pub fn run() -> String {
    format!("Codenotch privacy build: Codex and DeepSeek only; no hooks or conversation watcher.\n{}\n", crate::codex::probe())
}