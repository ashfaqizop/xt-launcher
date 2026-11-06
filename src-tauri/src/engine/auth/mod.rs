//! Accounts: Microsoft (device code), Ely.by (launcher API + authlib-injector) and offline.
//! Secrets are kept in the Windows Credential Manager; the UI only ever sees names and ids.

mod ely;
mod ms;
mod store;

use super::http;
use super::launch::{self, Account};
use super::paths::Paths;
use anyhow::{anyhow, Result};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, State};

/// Azure application (client) id for Microsoft sign-in. Fill this in once the app is
/// registered; until then the UI lets the user paste their own id.
pub const BUILTIN_MS_CLIENT_ID: &str = "";

const INJECTOR_VERSION: &str = "1.2.8";
const INJECTOR_URL: &str =
    "https://github.com/yushijinhun/authlib-injector/releases/download/v1.2.8/authlib-injector-1.2.8.jar";

#[derive(Default)]
pub struct AuthState {
    cancel: AtomicBool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AccountResult {
    pub id: String,
    pub name: String,
    pub uuid: String,
    pub kind: String,
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum LoginOutcome {
    Done { account: AccountResult },
    TwoFactor,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct DeviceCodePayload {
    user_code: String,
    verification_uri: String,
}

fn pick_client_id(from_ui: Option<String>) -> Result<String> {
    let id = from_ui
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| BUILTIN_MS_CLIENT_ID.to_string());
    if id.is_empty() {
        return Err(anyhow!(
            "Microsoft sign-in needs an Azure application (client) id. Add it in the Microsoft dialog."
        ));
    }
    Ok(id)
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// Whether this build ships its own Microsoft client id.
#[tauri::command]
pub fn ms_has_builtin_client() -> bool {
    !BUILTIN_MS_CLIENT_ID.is_empty()
}

async fn run_ms_login(
    app: &AppHandle,
    state: &AuthState,
    client_id: Option<String>,
) -> Result<AccountResult> {
    let cid = pick_client_id(client_id)?;
    state.cancel.store(false, Ordering::SeqCst);

    let code = ms::request_device_code(&cid).await?;
    let _ = app.emit(
        "auth-device-code",
        DeviceCodePayload {
            user_code: code.user_code.clone(),
            verification_uri: code.verification_uri.clone(),
        },
    );

    let tokens = ms::poll_for_token(&cid, &code, &state.cancel).await?;
    let mc = ms::minecraft_login(&tokens.access_token).await?;

    let id = format!("ms-{}", mc.uuid);
    store::save(&id, &tokens.refresh_token)?;
    Ok(AccountResult {
        id,
        name: mc.name,
        uuid: mc.uuid,
        kind: "microsoft".to_string(),
    })
}

#[tauri::command]
pub async fn ms_login(
    app: AppHandle,
    state: State<'_, AuthState>,
    client_id: Option<String>,
) -> Result<AccountResult, String> {
    run_ms_login(&app, &state, client_id)
        .await
        .map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub fn ms_cancel(state: State<'_, AuthState>) {
    state.cancel.store(true, Ordering::SeqCst);
}

#[tauri::command]
pub async fn ely_login(
    app: AppHandle,
    username: String,
    password: String,
    totp: Option<String>,
) -> Result<LoginOutcome, String> {
    let paths = Paths::new(&app).map_err(|e| e.to_string())?;
    let client_token = ely::client_token(&paths)
        .await
        .map_err(|e| format!("{e:#}"))?;

    match ely::authenticate(username.trim(), &password, totp.as_deref(), &client_token).await {
        Ok(session) => {
            let id = format!("ely-{}", session.uuid);
            store::save(&id, &session.access_token).map_err(|e| format!("{e:#}"))?;
            Ok(LoginOutcome::Done {
                account: AccountResult {
                    id,
                    name: session.name,
                    uuid: session.uuid,
                    kind: "elyby".to_string(),
                },
            })
        }
        Err(ely::ElyError::TwoFactor) => Ok(LoginOutcome::TwoFactor),
        Err(ely::ElyError::Other(e)) => Err(format!("{e:#}")),
    }
}

/// Forget the stored tokens of a removed account.
#[tauri::command]
pub fn forget_account(id: String) {
    store::delete(&id);
}

// ---------------------------------------------------------------------------
// Launch-time resolution
// ---------------------------------------------------------------------------

pub struct Resolved {
    pub account: Account,
    /// JVM arguments the account type needs (authlib-injector for Ely.by)
    pub extra_jvm: Vec<String>,
}

async fn ensure_injector(paths: &Paths) -> Result<PathBuf> {
    let dest = paths
        .cache()
        .join(format!("authlib-injector-{INJECTOR_VERSION}.jar"));
    http::download(INJECTOR_URL, &dest, None, None).await?;
    Ok(dest)
}

/// Turn a UI account into launch credentials, refreshing tokens where needed.
pub async fn resolve_for_launch(
    paths: &Paths,
    kind: &str,
    id: &str,
    name: &str,
    ms_client_id: Option<String>,
) -> Result<Resolved> {
    match kind {
        "microsoft" => {
            let cid = pick_client_id(ms_client_id)?;
            let refresh_token = store::load(id)?
                .ok_or_else(|| anyhow!("This Microsoft account needs to sign in again."))?;
            let tokens = ms::refresh(&cid, &refresh_token).await?;
            store::save(id, &tokens.refresh_token)?;
            let mc = ms::minecraft_login(&tokens.access_token).await?;
            Ok(Resolved {
                account: Account {
                    name: mc.name,
                    uuid: mc.uuid,
                    access_token: mc.access_token,
                    user_type: "msa".to_string(),
                },
                extra_jvm: Vec::new(),
            })
        }
        "elyby" => {
            let access_token = store::load(id)?
                .ok_or_else(|| anyhow!("This Ely.by account needs to sign in again."))?;
            let client_token = ely::client_token(paths).await?;
            let session = ely::refresh(&access_token, &client_token).await?;
            store::save(id, &session.access_token)?;
            let jar = ensure_injector(paths).await?;
            Ok(Resolved {
                account: Account {
                    name: session.name,
                    uuid: session.uuid,
                    access_token: session.access_token,
                    user_type: "mojang".to_string(),
                },
                extra_jvm: vec![format!("-javaagent:{}={}", jar.display(), ely::API_ROOT)],
            })
        }
        _ => Ok(Resolved {
            account: Account {
                uuid: launch::offline_uuid(name),
                name: name.to_string(),
                access_token: "0".to_string(),
                user_type: "legacy".to_string(),
            },
            extra_jvm: Vec::new(),
        }),
    }
}
