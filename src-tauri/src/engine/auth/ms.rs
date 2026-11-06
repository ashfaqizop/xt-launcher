//! Microsoft account sign-in for Minecraft: device-code login, Xbox Live, XSTS, Minecraft services.

use crate::engine::http;
use anyhow::{anyhow, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

const DEVICE_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";
const TOKEN_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
const SCOPE: &str = "XboxLive.signin offline_access";

#[derive(Deserialize, Clone)]
pub struct DeviceCode {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    #[serde(default)]
    pub interval: Option<u64>,
}

pub struct MsTokens {
    pub access_token: String,
    pub refresh_token: String,
}

pub struct McLogin {
    pub access_token: String,
    pub uuid: String,
    pub name: String,
}

async fn post_form(url: &str, params: &[(&str, &str)]) -> Result<(u16, String)> {
    let resp = http::client().post(url).form(params).send().await?;
    let status = resp.status().as_u16();
    Ok((status, resp.text().await?))
}

async fn post_json(url: &str, body: Value) -> Result<(u16, Value)> {
    let resp = http::client()
        .post(url)
        .header("Accept", "application/json")
        .json(&body)
        .send()
        .await?;
    let status = resp.status().as_u16();
    let text = resp.text().await?;
    let value = serde_json::from_str::<Value>(&text).unwrap_or(Value::Null);
    Ok((status, value))
}

fn error_text(body: &str) -> String {
    serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|v| {
            v.get("error_description")
                .or_else(|| v.get("error"))
                .and_then(|e| e.as_str())
                .map(|s| s.lines().next().unwrap_or(s).to_string())
        })
        .unwrap_or_else(|| "unknown error".to_string())
}

fn parse_tokens(body: &str, fallback_refresh: Option<&str>) -> Result<MsTokens> {
    let v: Value = serde_json::from_str(body)?;
    let access_token = v
        .get("access_token")
        .and_then(|t| t.as_str())
        .ok_or_else(|| anyhow!("Microsoft did not return an access token"))?
        .to_string();
    let refresh_token = v
        .get("refresh_token")
        .and_then(|t| t.as_str())
        .or(fallback_refresh)
        .ok_or_else(|| anyhow!("Microsoft did not return a refresh token"))?
        .to_string();
    Ok(MsTokens {
        access_token,
        refresh_token,
    })
}

pub async fn request_device_code(client_id: &str) -> Result<DeviceCode> {
    let (status, body) = post_form(DEVICE_URL, &[("client_id", client_id), ("scope", SCOPE)]).await?;
    if status != 200 {
        return Err(anyhow!(
            "Microsoft refused the sign-in request: {}",
            error_text(&body)
        ));
    }
    Ok(serde_json::from_str(&body)?)
}

/// Wait until the user finishes signing in on the Microsoft page (or cancels / the code expires).
pub async fn poll_for_token(
    client_id: &str,
    code: &DeviceCode,
    cancel: &AtomicBool,
) -> Result<MsTokens> {
    let mut interval = code.interval.unwrap_or(5).max(1);
    let deadline = Instant::now() + Duration::from_secs(code.expires_in);

    loop {
        if cancel.load(Ordering::SeqCst) {
            return Err(anyhow!("Sign-in was cancelled"));
        }
        if Instant::now() > deadline {
            return Err(anyhow!("The sign-in code expired. Try again."));
        }
        tokio::time::sleep(Duration::from_secs(interval)).await;

        let (status, body) = post_form(
            TOKEN_URL,
            &[
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("client_id", client_id),
                ("device_code", code.device_code.as_str()),
            ],
        )
        .await?;

        if status == 200 {
            return parse_tokens(&body, None);
        }

        let error = serde_json::from_str::<Value>(&body)
            .ok()
            .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(str::to_string))
            .unwrap_or_default();
        match error.as_str() {
            "authorization_pending" => {}
            "slow_down" => interval += 5,
            "authorization_declined" => return Err(anyhow!("Sign-in was declined")),
            "expired_token" => return Err(anyhow!("The sign-in code expired. Try again.")),
            _ => return Err(anyhow!("Microsoft sign-in failed: {}", error_text(&body))),
        }
    }
}

pub async fn refresh(client_id: &str, refresh_token: &str) -> Result<MsTokens> {
    let (status, body) = post_form(
        TOKEN_URL,
        &[
            ("grant_type", "refresh_token"),
            ("client_id", client_id),
            ("refresh_token", refresh_token),
            ("scope", SCOPE),
        ],
    )
    .await?;
    if status != 200 {
        return Err(anyhow!(
            "Your Microsoft session expired. Sign in again ({}).",
            error_text(&body)
        ));
    }
    parse_tokens(&body, Some(refresh_token))
}

fn xsts_error(code: u64) -> &'static str {
    match code {
        2148916233 => "This Microsoft account has no Xbox profile. Sign in once at xbox.com to create one.",
        2148916235 => "Xbox Live isn't available in your country.",
        2148916238 => "This is a child account. Add it to a Microsoft family to play online.",
        _ => "Xbox Live refused the sign-in.",
    }
}

/// Microsoft access token -> Xbox Live -> XSTS -> Minecraft access token + profile.
pub async fn minecraft_login(ms_access_token: &str) -> Result<McLogin> {
    // Xbox Live
    let (status, xbl) = post_json(
        "https://user.auth.xboxlive.com/user/authenticate",
        json!({
            "Properties": {
                "AuthMethod": "RPS",
                "SiteName": "user.auth.xboxlive.com",
                "RpsTicket": format!("d={ms_access_token}")
            },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT"
        }),
    )
    .await?;
    if status != 200 {
        return Err(anyhow!("Xbox Live sign-in failed ({status})"));
    }
    let xbl_token = xbl
        .get("Token")
        .and_then(|t| t.as_str())
        .ok_or_else(|| anyhow!("Xbox Live returned no token"))?
        .to_string();
    let user_hash = xbl["DisplayClaims"]["xui"][0]["uhs"]
        .as_str()
        .ok_or_else(|| anyhow!("Xbox Live returned no user hash"))?
        .to_string();

    // XSTS
    let (status, xsts) = post_json(
        "https://xsts.auth.xboxlive.com/xsts/authorize",
        json!({
            "Properties": { "SandboxId": "RETAIL", "UserTokens": [xbl_token] },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT"
        }),
    )
    .await?;
    if status != 200 {
        let code = xsts.get("XErr").and_then(|c| c.as_u64()).unwrap_or(0);
        return Err(anyhow!("{}", xsts_error(code)));
    }
    let xsts_token = xsts
        .get("Token")
        .and_then(|t| t.as_str())
        .ok_or_else(|| anyhow!("Xbox Live returned no XSTS token"))?
        .to_string();

    // Minecraft services
    let (status, login) = post_json(
        "https://api.minecraftservices.com/authentication/login_with_xbox",
        json!({ "identityToken": format!("XBL3.0 x={user_hash};{xsts_token}") }),
    )
    .await?;
    if status != 200 {
        return Err(anyhow!("Minecraft sign-in failed ({status})"));
    }
    let access_token = login
        .get("access_token")
        .and_then(|t| t.as_str())
        .ok_or_else(|| anyhow!("Minecraft returned no access token"))?
        .to_string();

    // Profile (404 means the account does not own the game)
    let resp = http::client()
        .get("https://api.minecraftservices.com/minecraft/profile")
        .bearer_auth(&access_token)
        .send()
        .await?;
    if resp.status().as_u16() == 404 {
        return Err(anyhow!(
            "This Microsoft account doesn't own Minecraft: Java Edition."
        ));
    }
    let profile: Value = resp.error_for_status()?.json().await?;
    let uuid = profile
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Minecraft returned no profile id"))?
        .to_string();
    let name = profile
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Minecraft returned no profile name"))?
        .to_string();

    Ok(McLogin {
        access_token,
        uuid,
        name,
    })
}
