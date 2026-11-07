//! Ely.by sign-in with OAuth 2.0 (authorization code + PKCE, public client `xt-launcher`).
//!
//! The launcher opens the browser, the user logs in and authorizes, Ely.by redirects to a
//! one-shot loopback server on 127.0.0.1, and the code is exchanged for tokens. The access
//! token carries the `minecraft_server_session` scope, so it is also the game's session
//! token (used through authlib-injector).

use crate::engine::http;
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

pub const CLIENT_ID: &str = "xt-launcher";
pub const API_ROOT: &str = "https://authserver.ely.by/api/authlib-injector";
const AUTHORIZE_URL: &str = "https://account.ely.by/oauth2/v1";
const TOKEN_URL: &str = "https://account.ely.by/api/oauth2/v1/token";
const INFO_URL: &str = "https://account.ely.by/api/account/v1/info";
const SCOPE: &str = "account_info offline_access minecraft_server_session";
const LOGIN_TIMEOUT: Duration = Duration::from_secs(300);

pub struct ElySession {
    pub access_token: String,
    pub refresh_token: String,
    pub uuid: String,
    pub name: String,
}

fn random_string(bytes: usize) -> Result<String> {
    let mut buf = vec![0u8; bytes];
    getrandom::getrandom(&mut buf).map_err(|e| anyhow!("random number generator: {e}"))?;
    Ok(URL_SAFE_NO_PAD.encode(&buf))
}

/// One pending sign-in: PKCE secrets, the loopback listener and the URL to open.
pub struct Pending {
    pub url: String,
    listener: TcpListener,
    redirect_uri: String,
    verifier: String,
    state: String,
}

pub async fn begin() -> Result<Pending> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    let redirect_uri = format!("http://127.0.0.1:{port}/callback");

    let verifier = random_string(48)?;
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let state = random_string(16)?;

    let url = reqwest::Url::parse_with_params(
        AUTHORIZE_URL,
        &[
            ("client_id", CLIENT_ID),
            ("redirect_uri", redirect_uri.as_str()),
            ("response_type", "code"),
            ("scope", SCOPE),
            ("state", state.as_str()),
            ("code_challenge", challenge.as_str()),
            ("code_challenge_method", "S256"),
        ],
    )?
    .to_string();

    Ok(Pending {
        url,
        listener,
        redirect_uri,
        verifier,
        state,
    })
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
                match u8::from_str_radix(hex, 16) {
                    Ok(b) => {
                        out.push(b);
                        i += 3;
                    }
                    Err(_) => {
                        out.push(b'%');
                        i += 1;
                    }
                }
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// `None` for requests that are not the OAuth callback (favicon etc.).
fn parse_callback(target: &str, expected_state: &str) -> Option<Result<String>> {
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    if path != "/callback" {
        return None;
    }

    let mut code = None;
    let mut state = None;
    let mut error = None;
    let mut error_description = None;
    for pair in query.split('&') {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        let value = percent_decode(value);
        match key {
            "code" => code = Some(value),
            "state" => state = Some(value),
            "error" => error = Some(value),
            "error_description" => error_description = Some(value),
            _ => {}
        }
    }

    if let Some(e) = error {
        let detail = error_description.unwrap_or(e);
        return Some(Err(anyhow!("Ely.by refused the sign-in: {detail}")));
    }
    if state.as_deref() != Some(expected_state) {
        return Some(Err(anyhow!("The sign-in response didn't match this request")));
    }
    Some(code.ok_or_else(|| anyhow!("Ely.by returned no authorization code")))
}

/// The page the browser lands on after Ely.by redirects back (shown once, then the
/// loopback server stops). Self-contained: no network requests, light and dark aware.
fn callback_page(ok: bool) -> String {
    const TEMPLATE: &str = include_str!("callback.html");
    const LOGO: &[u8] = include_bytes!("../../../../src/assets/flame.png");
    // Bricolage Grotesque (SIL Open Font License), the launcher's own typeface
    const FONT: &[u8] = include_bytes!("bricolage.woff2");

    let (state, heading, message, color, icon) = if ok {
        (
            "ok",
            "You're connected",
            "Ely.by is linked to XT Launcher. Head back to the launcher, your account is ready to play.",
            "#3ccf7a",
            "M5.5 12.5l4.2 4.2L18.5 7.5",
        )
    } else {
        (
            "fail",
            "Sign-in didn't finish",
            "Nothing was changed. Go back to XT Launcher and try signing in again.",
            "#ef5350",
            "M7 7l10 10M17 7L7 17",
        )
    };
    let font = format!("data:font/woff2;base64,{}", base64::engine::general_purpose::STANDARD.encode(FONT));
    let logo = format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(LOGO));
    TEMPLATE
        .replace("{{STATE_COLOR}}", color)
        .replace("{{STATE}}", state)
        .replace("{{FONT}}", &font)
        .replace("{{HEADING}}", heading)
        .replace("{{MESSAGE}}", message)
        .replace("{{ICON_PATH}}", icon)
        .replace("{{LOGO}}", &logo)
}

async fn respond(stream: &mut TcpStream, ok: bool) {
    let body = callback_page(ok);
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nCache-Control: no-store\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.shutdown().await;
}

/// Wait for the browser to come back, then trade the code for tokens and read the profile.
pub async fn complete(pending: Pending, cancel: &AtomicBool) -> Result<ElySession> {
    let deadline = Instant::now() + LOGIN_TIMEOUT;

    let code = loop {
        if cancel.load(Ordering::SeqCst) {
            return Err(anyhow!("Sign-in was cancelled"));
        }
        if Instant::now() > deadline {
            return Err(anyhow!("Sign-in timed out. Try again."));
        }

        let accepted =
            tokio::time::timeout(Duration::from_millis(500), pending.listener.accept()).await;
        let mut stream = match accepted {
            Ok(Ok((stream, _))) => stream,
            Ok(Err(e)) => return Err(e.into()),
            Err(_) => continue,
        };

        let mut buf = vec![0u8; 8192];
        let read = tokio::time::timeout(Duration::from_secs(5), stream.read(&mut buf)).await;
        let n = match read {
            Ok(Ok(n)) => n,
            _ => 0,
        };
        let request = String::from_utf8_lossy(&buf[..n]).into_owned();
        let target = request
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .unwrap_or("")
            .to_string();

        match parse_callback(&target, &pending.state) {
            Some(result) => {
                respond(&mut stream, result.is_ok()).await;
                break result?;
            }
            None => {
                let _ = stream.shutdown().await;
            }
        }
    };

    let (status, body) = post_form(
        TOKEN_URL,
        &[
            ("grant_type", "authorization_code"),
            ("client_id", CLIENT_ID),
            ("redirect_uri", pending.redirect_uri.as_str()),
            ("code", code.as_str()),
            ("code_verifier", pending.verifier.as_str()),
        ],
    )
    .await?;
    if status != 200 {
        return Err(anyhow!("Ely.by token exchange failed: {}", error_text(&body)));
    }

    let tokens: Value = serde_json::from_str(&body)?;
    let access_token = tokens
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Ely.by returned no access token"))?
        .to_string();
    let refresh_token = tokens
        .get("refresh_token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Ely.by did not issue a refresh token"))?
        .to_string();

    let (uuid, name) = profile(&access_token).await?;
    Ok(ElySession {
        access_token,
        refresh_token,
        uuid,
        name,
    })
}

async fn post_form(url: &str, params: &[(&str, &str)]) -> Result<(u16, String)> {
    let resp = http::client().post(url).form(params).send().await?;
    let status = resp.status().as_u16();
    Ok((status, resp.text().await?))
}

fn error_text(body: &str) -> String {
    serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|v| {
            v.get("error_description")
                .or_else(|| v.get("error"))
                .and_then(|e| e.as_str())
                .map(str::to_string)
        })
        .unwrap_or_else(|| "unknown error".to_string())
}

/// The Minecraft profile (uuid without dashes, username) behind an access token.
async fn profile(access_token: &str) -> Result<(String, String)> {
    let resp = http::client()
        .get(INFO_URL)
        .bearer_auth(access_token)
        .send()
        .await?
        .error_for_status()?;
    let info: Value = resp.json().await?;
    let uuid = info
        .get("uuid")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Ely.by returned no profile id"))?
        .replace('-', "");
    let name = info
        .get("username")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Ely.by returned no username"))?
        .to_string();
    Ok((uuid, name))
}

/// Trade a stored refresh token for a fresh access token (used at every launch).
pub async fn refresh(refresh_token: &str) -> Result<ElySession> {
    let (status, body) = post_form(
        TOKEN_URL,
        &[
            ("grant_type", "refresh_token"),
            ("client_id", CLIENT_ID),
            ("refresh_token", refresh_token),
            ("scope", SCOPE),
        ],
    )
    .await?;
    if status != 200 {
        return Err(anyhow!(
            "Your Ely.by session expired. Sign in again ({}).",
            error_text(&body)
        ));
    }

    let tokens: Value = serde_json::from_str(&body)?;
    let access_token = tokens
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Ely.by returned no access token"))?
        .to_string();
    let refresh_token = tokens
        .get("refresh_token")
        .and_then(|v| v.as_str())
        .unwrap_or(refresh_token)
        .to_string();

    let (uuid, name) = profile(&access_token).await?;
    Ok(ElySession {
        access_token,
        refresh_token,
        uuid,
        name,
    })
}
