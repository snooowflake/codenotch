//! Only Codenotch's Grok credential; never enumerate or return secrets via IPC.
const TARGET: &str = "codenotch:grok";
pub fn valid_team(team: &str) -> bool {
    (1..=128).contains(&team.len()) && team.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}
pub fn valid_key(key: &str) -> bool {
    (8..=2048).contains(&key.len()) && key.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_.=".contains(&b))
}
pub fn valid(packed: &str) -> bool {
    packed.split_once('\n').map(|(team,key)| valid_team(team) && valid_key(key)).unwrap_or(false)
}
#[cfg(windows)]
pub fn read() -> Option<String> {
    use windows::core::PCWSTR;
    use windows::Win32::Security::Credentials::*;
    let target: Vec<u16> = TARGET.encode_utf16().chain(Some(0)).collect();
    let mut ptr: *mut CREDENTIALW = std::ptr::null_mut();
    unsafe {
        if CredReadW(PCWSTR(target.as_ptr()), CRED_TYPE_GENERIC, 0, &mut ptr).is_err() || ptr.is_null() { return None; }
        let c = &*ptr;
        let result = if !c.CredentialBlob.is_null() && (1..=2560).contains(&c.CredentialBlobSize) {
            String::from_utf8(std::slice::from_raw_parts(c.CredentialBlob,c.CredentialBlobSize as usize).to_vec()).ok().filter(|k| valid(k))
        } else { None };
        CredFree(ptr as *const core::ffi::c_void);
        result
    }
}
#[cfg(windows)]
pub fn exists() -> bool {
    // Metadata only: do not copy the credential value to check presence.
    use windows::core::PCWSTR;
    use windows::Win32::Security::Credentials::*;
    let target: Vec<u16> = TARGET.encode_utf16().chain(Some(0)).collect();
    let mut ptr: *mut CREDENTIALW = std::ptr::null_mut();
    unsafe {
        if CredReadW(PCWSTR(target.as_ptr()), CRED_TYPE_GENERIC, 0, &mut ptr).is_err() || ptr.is_null() { return false; }
        CredFree(ptr as *const core::ffi::c_void);
        true
    }
}
#[cfg(windows)]
pub fn save(key: &str) -> Result<(), String> {
    if !valid(key) { return Err("Format de clé Grok invalide".into()); }
    use windows::core::PWSTR;
    use windows::Win32::Security::Credentials::*;
    let mut target: Vec<u16> = TARGET.encode_utf16().chain(Some(0)).collect();
    let mut name: Vec<u16> = "Grok API".encode_utf16().chain(Some(0)).collect();
    let mut bytes = key.as_bytes().to_vec();
    let value = CREDENTIALW {
        Type: CRED_TYPE_GENERIC, TargetName: PWSTR(target.as_mut_ptr()),
        UserName: PWSTR(name.as_mut_ptr()), CredentialBlobSize: bytes.len() as u32,
        CredentialBlob: bytes.as_mut_ptr(), Persist: CRED_PERSIST_LOCAL_MACHINE,
        ..Default::default()
    };
    let result = unsafe { CredWriteW(&value, 0) }.map_err(|_| "Impossible d'enregistrer la clé dans le coffre Windows".into());
    bytes.fill(0);
    result
}
#[cfg(windows)]
pub fn delete() -> Result<(), String> {
    use windows::core::PCWSTR;
    use windows::Win32::Security::Credentials::*;
    if !exists() { return Ok(()); }
    let target: Vec<u16> = TARGET.encode_utf16().chain(Some(0)).collect();
    unsafe { CredDeleteW(PCWSTR(target.as_ptr()), CRED_TYPE_GENERIC, 0) }.map_err(|_| "Impossible de supprimer la clé enregistrée".into())
}
#[cfg(not(windows))]
pub fn read() -> Option<String> { None }
#[cfg(not(windows))]
pub fn exists() -> bool { false }
#[cfg(not(windows))]
pub fn save(_: &str) -> Result<(), String> { Err("Windows required".into()) }
#[cfg(not(windows))]
pub fn delete() -> Result<(), String> { Err("Windows required".into()) }
#[cfg(test)]
mod tests {
    #[test]
    fn rejects_path_and_header_injection() {
        for team in ["", "../auth", "a?x=1", "a/b", "a\n"] { assert!(!super::valid_team(team)); }
        for key in ["", "key\r\nHost: attacker", "<script>"] { assert!(!super::valid_key(key)); }
        assert!(super::valid("team-test\nxai-test-fixture"));
        assert!(!super::valid("team-test\nxai-test-fixture\nextra"));
    }
}
