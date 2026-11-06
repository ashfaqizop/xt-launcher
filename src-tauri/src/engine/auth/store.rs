//! Secrets (refresh and access tokens) live in the Windows Credential Manager, never in
//! the app's own files. Credential Manager caps one entry at about 2.5 KB (UTF-16), and
//! Microsoft refresh tokens can be longer, so a secret is split into small chunks.

use anyhow::{anyhow, Result};

const SERVICE: &str = "XT Launcher";
const CHUNK_CHARS: usize = 800;

fn entry(name: &str) -> Result<keyring::Entry> {
    keyring::Entry::new(SERVICE, name).map_err(|e| anyhow!("credential store: {e}"))
}

pub fn save(id: &str, secret: &str) -> Result<()> {
    delete(id);

    let chars: Vec<char> = secret.chars().collect();
    let chunks: Vec<String> = chars
        .chunks(CHUNK_CHARS)
        .map(|part| part.iter().collect::<String>())
        .collect();

    for (i, chunk) in chunks.iter().enumerate() {
        entry(&format!("{id}#{i}"))?
            .set_password(chunk)
            .map_err(|e| anyhow!("credential store: {e}"))?;
    }
    entry(&format!("{id}#n"))?
        .set_password(&chunks.len().to_string())
        .map_err(|e| anyhow!("credential store: {e}"))?;
    Ok(())
}

pub fn load(id: &str) -> Result<Option<String>> {
    let count = match entry(&format!("{id}#n"))?.get_password() {
        Ok(text) => text.parse::<usize>().unwrap_or(0),
        Err(keyring::Error::NoEntry) => return Ok(None),
        Err(e) => return Err(anyhow!("credential store: {e}")),
    };

    let mut secret = String::new();
    for i in 0..count {
        let part = entry(&format!("{id}#{i}"))?
            .get_password()
            .map_err(|e| anyhow!("credential store: {e}"))?;
        secret.push_str(&part);
    }
    Ok(Some(secret))
}

pub fn delete(id: &str) {
    let Ok(count_entry) = entry(&format!("{id}#n")) else {
        return;
    };
    if let Ok(text) = count_entry.get_password() {
        let count = text.parse::<usize>().unwrap_or(0);
        for i in 0..count {
            if let Ok(part) = entry(&format!("{id}#{i}")) {
                let _ = part.delete_credential();
            }
        }
    }
    let _ = count_entry.delete_credential();
}
