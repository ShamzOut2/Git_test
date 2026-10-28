//! Logging in to Garage 61 from the app (OAuth 2 authorization code with PKCE).
//!
//! "Log in" opens the browser at Garage 61's sign-in page; Garage 61 sends the browser back
//! to this app's own server (`/garage61/callback` on 127.0.0.1), which swaps the code for
//! tokens. The tokens are kept in `<data dir>/garage61-login.json`, apart from the settings,
//! so they are never sent to the settings page or included in exported settings. The
//! access token is renewed with the refresh token when it runs out.

use crate::config::Garage61Config;
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// The app's Garage 61 client ID, set when the release is built (Garage 61 registers apps).
pub const BUILT_IN_CLIENT_ID: &str = match option_env!("APEX_G61_CLIENT_ID") {
    Some(id) => id,
    None => "",
};
pub const CALLBACK: &str = "/garage61/callback";
/// How long a sign-in started in the browser may take.
const PENDING_FOR: Duration = Duration::from_secs(600);

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Login {
    pub access_token: String,
    pub refresh_token: String,
    /// Unix seconds when the access token runs out (0 = unknown).
    pub expires_at: i64,
    /// Who is signed in, for the settings page.
    pub account: String,
}

/// Login state as shown in the app (no secrets).
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LoginOut {
    /// The app can log in (it has a Garage 61 client ID).
    pub available: bool,
    pub logged_in: bool,
    pub account: String,
    /// "login" or "token" (a personal access token from the advanced settings), "" if neither.
    pub via: &'static str,
}

struct Pending {
    state: String,
    verifier: String,
    redirect: String,
    at: Instant,
}

static PENDING: Mutex<Option<Pending>> = Mutex::new(None);

pub fn client_id(cfg: &Garage61Config) -> String {
    let own = cfg.client_id.trim();
    if own.is_empty() {
        BUILT_IN_CLIENT_ID.to_string()
    } else {
        own.to_string()
    }
}

fn file(dir: &Path) -> PathBuf {
    dir.join("garage61-login.json")
}

pub fn load(dir: &Path) -> Option<Login> {
    std::fs::read_to_string(file(dir)).ok().and_then(|s| serde_json::from_str(&s).ok()).filter(|l: &Login| !l.access_token.is_empty())
}

fn store(dir: &Path, l: &Login) {
    let _ = std::fs::write(file(dir), serde_json::to_string(l).unwrap_or_default());
}

pub fn logout(dir: &Path) {
    let _ = std::fs::remove_file(file(dir));
}

fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn random(n: usize) -> Result<String, String> {
    let mut buf = vec![0u8; n];
    getrandom::getrandom(&mut buf).map_err(|e| format!("no random numbers: {e}"))?;
    Ok(b64(&buf))
}

/// PKCE: the challenge sent with the sign-in is the SHA-256 of the verifier sent with the code.
pub fn challenge(verifier: &str) -> String {
    b64(&Sha256::digest(verifier.as_bytes()))
}

fn encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Start a sign-in: the Garage 61 page to open in the browser.
pub fn begin(cfg: &Garage61Config, port: u16) -> Result<String, String> {
    let id = client_id(cfg);
    if id.is_empty() {
        return Err("Logging in needs Apex Overlays to be registered with Garage 61 (no client ID yet)".into());
    }
    let state = random(16)?;
    let verifier = random(48)?;
    let redirect = format!("http://127.0.0.1:{port}{CALLBACK}");
    let url = format!(
        "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&code_challenge={}&code_challenge_method=S256",
        cfg.auth_url.trim(),
        encode(&id),
        encode(&redirect),
        encode(cfg.scope.trim()),
        encode(&state),
        challenge(&verifier),
    );
    *PENDING.lock().unwrap() = Some(Pending { state, verifier, redirect, at: Instant::now() });
    Ok(url)
}

fn token_request(cfg: &Garage61Config, form: &[(&str, &str)]) -> Result<Value, String> {
    match crate::net::agent().post(cfg.token_url.trim()).send_form(form) {
        Ok(r) => r.into_json::<Value>().map_err(|e| e.to_string()),
        Err(ureq::Error::Status(code, r)) => {
            let body = r.into_string().unwrap_or_default();
            let v: Value = serde_json::from_str(&body).unwrap_or_default();
            let why = v.get("error_description").or(v.get("error")).and_then(|e| e.as_str()).unwrap_or(&body);
            Err(format!("Garage 61 said {code}: {}", why.chars().take(200).collect::<String>()))
        }
        Err(e) => Err(e.to_string()),
    }
}

fn login_from(v: &Value, old: Option<&Login>) -> Result<Login, String> {
    let access = v.get("access_token").and_then(|t| t.as_str()).ok_or("Garage 61 sent no access token")?;
    // A new refresh token replaces the old one (rotation); otherwise the old one stays valid.
    let refresh = v.get("refresh_token").and_then(|t| t.as_str()).map(String::from).or_else(|| old.map(|o| o.refresh_token.clone())).unwrap_or_default();
    let expires_in = v.get("expires_in").and_then(|t| t.as_i64()).unwrap_or(3600);
    Ok(Login {
        access_token: access.into(),
        refresh_token: refresh,
        expires_at: now() + expires_in,
        account: old.map(|o| o.account.clone()).unwrap_or_default(),
    })
}

/// The browser came back with a code: swap it for tokens and remember who signed in.
pub fn finish(cfg: &Garage61Config, dir: &Path, code: &str, state: &str) -> Result<Login, String> {
    let pending = PENDING.lock().unwrap().take().ok_or("This sign-in was not started from Apex Overlays (or was already used)")?;
    if pending.state != state {
        return Err("The sign-in did not match the one Apex Overlays started; try again".into());
    }
    if pending.at.elapsed() > PENDING_FOR {
        return Err("The sign-in took too long; try again".into());
    }
    let id = client_id(cfg);
    let v = token_request(
        cfg,
        &[("grant_type", "authorization_code"), ("code", code), ("redirect_uri", &pending.redirect), ("client_id", &id), ("code_verifier", &pending.verifier)],
    )?;
    let mut login = login_from(&v, None)?;
    login.account = account(&login.access_token).unwrap_or_default();
    store(dir, &login);
    Ok(login)
}

/// Name of the signed-in driver, from `GET /me`.
fn account(token: &str) -> Option<String> {
    let me: Value = crate::net::agent().get("https://garage61.net/api/v1/me").set("Authorization", &format!("Bearer {token}")).call().ok()?.into_json().ok()?;
    let name = format!("{} {}", me.get("firstName")?.as_str().unwrap_or(""), me.get("lastName").and_then(|v| v.as_str()).unwrap_or(""));
    Some(name.trim().to_string())
}

/// A valid access token for the signed-in user, renewed when it is about to run out.
/// `Err` means the login no longer works (it is then forgotten); `Ok(None)`: not logged in.
pub fn bearer(cfg: &Garage61Config, dir: &Path) -> Result<Option<String>, String> {
    let Some(login) = load(dir) else { return Ok(None) };
    if login.expires_at == 0 || login.expires_at - 60 > now() {
        return Ok(Some(login.access_token));
    }
    if login.refresh_token.is_empty() {
        logout(dir);
        return Err("Your Garage 61 login ran out: log in again".into());
    }
    let id = client_id(cfg);
    match token_request(cfg, &[("grant_type", "refresh_token"), ("refresh_token", &login.refresh_token), ("client_id", &id)]) {
        Ok(v) => {
            let fresh = login_from(&v, Some(&login))?;
            store(dir, &fresh);
            Ok(Some(fresh.access_token))
        }
        Err(e) if e.contains(" 400") || e.contains(" 401") || e.contains("invalid_grant") => {
            logout(dir);
            Err(format!("Your Garage 61 login ran out ({e}): log in again"))
        }
        // Offline or Garage 61 down: keep the login and try again later.
        Err(e) => Err(e),
    }
}

/// Open a web page in the user's browser.
pub fn open_in_browser(url: &str) -> Result<(), String> {
    #[cfg(windows)]
    let r = std::process::Command::new("rundll32").args(["url.dll,FileProtocolHandler", url]).spawn();
    #[cfg(target_os = "macos")]
    let r = std::process::Command::new("open").arg(url).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let r = std::process::Command::new("xdg-open").arg(url).spawn();
    r.map(|_| ()).map_err(|e| format!("could not open the browser: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_challenge_is_base64url_sha256() {
        // Checked with Python: urlsafe_b64encode(sha256(verifier)) without padding.
        assert_eq!(challenge("apex-overlays-pkce-test-verifier-0123456789abcdefghij"), "nJ5h8jCyQqv-tlPMCOZrwyDDZXyR4Kj6h9_1yvUdRyQ");
    }

    #[test]
    fn sign_in_url_and_state_check() {
        let cfg = Garage61Config { client_id: "apex".into(), ..Default::default() };
        let url = begin(&cfg, 8182).unwrap();
        assert!(
            url.starts_with(
                "https://garage61.net/app/account/oauth?response_type=code&client_id=apex&redirect_uri=http%3A%2F%2F127.0.0.1%3A8182%2Fgarage61%2Fcallback"
            ),
            "{url}"
        );
        assert!(url.contains("code_challenge_method=S256") && url.contains("scope=driving_data"));
        // A callback with someone else's state is refused (and uses up the pending sign-in).
        let dir = std::env::temp_dir();
        assert!(finish(&cfg, &dir, "code", "wrong").unwrap_err().contains("did not match"));
        assert!(finish(&cfg, &dir, "code", "wrong").unwrap_err().contains("not started"));
        // No client ID: no sign-in.
        assert!(begin(&Garage61Config::default(), 8182).is_err() || !BUILT_IN_CLIENT_ID.is_empty());
    }

    #[test]
    fn tokens_rotate() {
        let old = Login { access_token: "a".into(), refresh_token: "r1".into(), expires_at: 0, account: "Max".into() };
        let v = serde_json::json!({ "access_token": "b", "expires_in": 600 });
        let l = login_from(&v, Some(&old)).unwrap();
        assert_eq!((l.access_token.as_str(), l.refresh_token.as_str(), l.account.as_str()), ("b", "r1", "Max"));
        let v = serde_json::json!({ "access_token": "c", "refresh_token": "r2" });
        assert_eq!(login_from(&v, Some(&old)).unwrap().refresh_token, "r2");
    }
}
