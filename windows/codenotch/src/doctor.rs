//! Privacy-preserving diagnosis: no conversation or other-account reads.
pub fn run() -> String {
    format!("Codenotch: providers controlled in Connections; no hooks or conversation watcher.\n{}\n", crate::codex::probe())
}