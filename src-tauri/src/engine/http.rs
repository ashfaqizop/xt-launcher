use anyhow::{anyhow, Context, Result};
use futures_util::StreamExt;
use serde::de::DeserializeOwned;
use sha1::{Digest, Sha1};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;
use tokio::{fs, io::AsyncWriteExt};

pub fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .user_agent(concat!("XT-Launcher/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(600))
            .build()
            .expect("failed to build the HTTP client")
    })
}

pub async fn get_json<T: DeserializeOwned>(url: &str) -> Result<T> {
    let resp = client()
        .get(url)
        .send()
        .await
        .with_context(|| format!("request to {url}"))?
        .error_for_status()
        .with_context(|| format!("bad status from {url}"))?;
    resp.json::<T>()
        .await
        .with_context(|| format!("decoding {url}"))
}

pub async fn get_bytes(url: &str) -> Result<Vec<u8>> {
    let resp = client()
        .get(url)
        .send()
        .await
        .with_context(|| format!("request to {url}"))?
        .error_for_status()
        .with_context(|| format!("bad status from {url}"))?;
    Ok(resp.bytes().await?.to_vec())
}

/// A file counts as present when it exists and, if a size is known, matches it.
/// Checksums are verified when a file is downloaded, not on every launch (fast starts).
pub async fn is_current(path: &Path, size: Option<u64>) -> bool {
    match fs::metadata(path).await {
        Ok(meta) if meta.is_file() => match size {
            Some(expected) => expected == meta.len(),
            None => true,
        },
        _ => false,
    }
}

/// Download `url` to `dest` (skipped when already current), verifying SHA-1 when given.
/// Writes to a `.part` file first and retries a few times.
pub async fn download(
    url: &str,
    dest: &Path,
    sha1: Option<&str>,
    size: Option<u64>,
) -> Result<()> {
    if is_current(dest, size).await {
        return Ok(());
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).await?;
    }

    let mut last_err: Option<anyhow::Error> = None;
    for attempt in 0..3u64 {
        match try_download(url, dest, sha1).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                last_err = Some(e);
                tokio::time::sleep(Duration::from_millis(400 * (attempt + 1))).await;
            }
        }
    }
    Err(last_err
        .unwrap_or_else(|| anyhow!("download failed"))
        .context(format!("downloading {url}")))
}

async fn try_download(url: &str, dest: &Path, sha1: Option<&str>) -> Result<()> {
    let resp = client().get(url).send().await?.error_for_status()?;

    let mut tmp_name = dest.as_os_str().to_owned();
    tmp_name.push(".part");
    let tmp = PathBuf::from(tmp_name);

    let mut file = fs::File::create(&tmp).await?;
    let mut hasher = Sha1::new();
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        hasher.update(&chunk);
        file.write_all(&chunk).await?;
    }
    file.flush().await?;
    drop(file);

    if let Some(expected) = sha1 {
        let got = hex::encode(hasher.finalize());
        if !got.eq_ignore_ascii_case(expected) {
            let _ = fs::remove_file(&tmp).await;
            return Err(anyhow!("checksum mismatch (expected {expected}, got {got})"));
        }
    }
    fs::rename(&tmp, dest).await?;
    Ok(())
}
