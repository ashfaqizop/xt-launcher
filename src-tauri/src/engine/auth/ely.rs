//! Ely.by accounts through the launcher API at authserver.ely.by (login with password and
//! optional two-factor code, then refreshable access tokens). Used together with
//! authlib-injector so the game talks to Ely.by's session server.

use crate::engine::http;
use crate::engine::paths::Paths;
use anyhow::{anyhow, Result};
use serde_json::{json, Value};

pub const API_ROOT: &str = "https://authserver.ely.by/api/authlib-injector";
const BASE: &str = "https://authserver.ely.by";

pub struct ElySession {
    pub access_token: String,
    pub uuid: String,
    pub name: String,
}

pub enum ElyError {
    /// The account has two-factor auth enabled; ask for the code and retry.
    TwoFactor,
    Other(anyhow::Error),
}

impl From<anyhow::Error> for ElyError {
    fn from(e: anyhow::Error) -> Self {
        ElyError::Other(e)
    }
}

impl From<reqwest::Error> for ElyError {
    fn from(e: reqwest::Error) -> Self {
        ElyError::Other(e.into())
    }
}

/// A random-looking id for this installation, required by the Yggdrasil-style API.
pub async fn client_token(paths: &Paths) -> Result<String> {
    let file = paths.root.join("client_token");
    if let Ok(existing) = tokio::fs::read_to_string(&file).await {
        let token = existing.trim().to_string();
        if !token.is_empty() {
            return Ok(token);
        }
    }

    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let seed = format!("xt-launcher-{nanos}-{}", std::process::id());
    let token = format!("{:x}", md5::compute(seed));

    tokio::fs::create_dir_all(&paths.root).await?;
    tokio::fs::write(&file, &token).await?;
    Ok(token)
}

async fn post(path: &str, body: Value) -> Result<(u16, Value)> {
    let resp = http::client()
        .post(format!("{BASE}{path}"))
        .json(&body)
        .send()
        .await?;
    let status = resp.status().as_u16();
    let text = resp.text().await?;
    Ok((status, serde_json::from_str(&text).unwrap_or(Value::Null)))
}

fn message(body: &Value) -> String {
    body.get("errorMessage")
        .or_else(|| body.get("error"))
        .and_then(|m| m.as_str())
        .unwrap_or("Ely.by refused the request")
        .to_string()
}

fn read_session(body: &Value) -> Result<ElySession> {
    let access_token = body
        .get("accessToken")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Ely.by returned no access token"))?
        .to_string();
    let profile = body
        .get("selectedProfile")
        .ok_or_else(|| anyhow!("This Ely.by account has no Minecraft profile"))?;
    let uuid = profile
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Ely.by returned no profile id"))?
        .to_string();
    let name = profile
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Ely.by returned no profile name"))?
        .to_string();
    Ok(ElySession {
        access_token,
        uuid,
        name,
    })
}

pub async fn authenticate(
    username: &str,
    password: &str,
    totp: Option<&str>,
    client_token: &str,
) -> Result<ElySession, ElyError> {
    let password = match totp.map(str::trim).filter(|t| !t.is_empty()) {
        Some(code) => format!("{password}:{code}"),
        None => password.to_string(),
    };

    let (status, body) = post(
        "/auth/authenticate",
        json!({
            "username": username,
            "password": password,
            "clientToken": client_token,
            "requestUser": false
        }),
    )
    .await?;

    if status == 200 {
        return Ok(read_session(&body)?);
    }
    let text = message(&body);
    if text.to_lowercase().contains("two factor") {
        return Err(ElyError::TwoFactor);
    }
    Err(ElyError::Other(anyhow!("{text}")))
}

/// Exchange a stored access token for a fresh one (and the profile).
pub async fn refresh(access_token: &str, client_token: &str) -> Result<ElySession> {
    let (status, body) = post(
        "/auth/refresh",
        json!({
            "accessToken": access_token,
            "clientToken": client_token,
            "requestUser": false
        }),
    )
    .await?;
    if status != 200 {
        return Err(anyhow!(
            "Your Ely.by session expired. Sign in again ({}).",
            message(&body)
        ));
    }
    read_session(&body)
}
