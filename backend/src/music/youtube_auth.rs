//! YouTube authentication for playback and search.
//!
//! YouTube rejects anonymous automated requests from datacenter IPs with
//! "Sign in to confirm you're not a bot". The supported fix is to authorize a
//! (burner) Google account once via OAuth 2.0 Device Flow and then reuse the
//! refresh token.
//!
//! This module owns the whole lifecycle:
//!
//! * [`YouTubeAuth::begin`] — start the device flow, return URL + user code;
//! * [`YouTubeAuth::finish`] — exchange the pending code for tokens;
//! * [`YouTubeAuth::refresh`] — obtain a fresh access token;
//! * [`YouTubeAuth::write_cookies`] — render a Netscape `cookies.txt` that
//!   `yt-dlp`, `mpv` and `ytmusicapi` all understand.
//!
//! Security: client secret and refresh token are never returned to the web
//! panel. Only the verification URL, the user code and coarse status are.
//! The token file is written with mode 0600.

use crate::common::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use tracing::{info, warn};

/// Environment variable pointing at the OAuth helper script.
const HELPER_ENV: &str = "KAMISH_YOUTUBE_OAUTH_HELPER";
/// Default helper location, relative to the repository root.
const HELPER_DEFAULT: &str = "scripts/youtube-oauth";
/// Maximum time we let the helper talk to Google.
const HELPER_TIMEOUT_SECS: u64 = 45;

/// Persistent YouTube credentials.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YouTubeTokens {
    #[serde(default)]
    pub client_id: String,
    #[serde(default)]
    pub client_secret: String,
    pub refresh_token: String,
    #[serde(default)]
    pub access_token: Option<String>,
    /// Unix timestamp (seconds) when `access_token` stops being valid.
    #[serde(default)]
    pub expires_at: Option<i64>,
    #[serde(default)]
    pub scope: Option<String>,
    /// Google account hint for the panel, if known.
    #[serde(default)]
    pub account: Option<String>,
}

/// Step returned to the panel when authorization starts.
#[derive(Debug, Clone, Serialize)]
pub struct DeviceCodeChallenge {
    /// Page the user must open.
    pub verification_url: String,
    /// Code the user types on that page.
    pub user_code: String,
    /// Lifetime of the code in seconds.
    pub expires_in: Option<i64>,
    /// Polling interval suggested by Google.
    pub interval: Option<i64>,
    /// Opaque server-side handle; the panel passes it back to `finish`.
    pub device_code: String,
}

/// Current authorization state, shown in the panel.
#[derive(Debug, Clone, Serialize)]
pub struct YouTubeAuthStatus {
    /// A refresh token is stored.
    pub authorized: bool,
    /// Client id is stored (secret is never exposed).
    pub has_client: bool,
    /// Access token is present and not expired.
    pub access_token_valid: bool,
    /// Seconds until the access token expires, when known.
    pub expires_in: Option<i64>,
    /// Where the token file lives.
    pub token_path: String,
    /// Where the generated cookies file lives.
    pub cookies_path: String,
    /// Cookies file exists and is non-empty.
    pub cookies_ready: bool,
    /// Reason authorization is not usable yet.
    pub problem: Option<String>,
}

/// Handles YouTube OAuth and credential storage.
pub struct YouTubeAuth {
    token_path: PathBuf,
    cookies_path: PathBuf,
}

impl Default for YouTubeAuth {
    fn default() -> Self {
        Self::new()
    }
}

impl YouTubeAuth {
    /// Resolve paths from the environment with sensible defaults.
    ///
    /// * `KAMISH_YOUTUBE_TOKEN_PATH` — default `/var/lib/kamish/youtube-oauth.json`
    /// * `KAMISH_YOUTUBE_COOKIES_PATH` — default `/var/lib/kamish/youtube-cookies.txt`
    pub fn new() -> Self {
        let token_path = std::env::var("KAMISH_YOUTUBE_TOKEN_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/var/lib/kamish/youtube-oauth.json"));

        let cookies_path = std::env::var("KAMISH_YOUTUBE_COOKIES_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/var/lib/kamish/youtube-cookies.txt"));

        Self {
            token_path,
            cookies_path,
        }
    }

    /// Construct with explicit paths (used by tests).
    pub fn with_paths(token_path: PathBuf, cookies_path: PathBuf) -> Self {
        Self {
            token_path,
            cookies_path,
        }
    }

    pub fn token_path(&self) -> &Path {
        &self.token_path
    }

    pub fn cookies_path(&self) -> &Path {
        &self.cookies_path
    }

    fn helper_path() -> PathBuf {
        if let Ok(p) = std::env::var(HELPER_ENV) {
            return PathBuf::from(p);
        }
        // Try a few likely locations relative to the working directory.
        for candidate in [
            HELPER_DEFAULT,
            "../scripts/youtube-oauth",
            "/opt/kamish/scripts/youtube-oauth",
        ] {
            let path = PathBuf::from(candidate);
            if path.is_file() {
                return path;
            }
        }
        PathBuf::from(HELPER_DEFAULT)
    }

    /// Run the helper script and parse its JSON stdout.
    async fn run_helper(args: &[&str]) -> AppResult<serde_json::Value> {
        let helper = Self::helper_path();

        if !helper.is_file() {
            return Err(AppError::Config(format!(
                "OAuth helper не найден: {}",
                helper.display()
            )));
        }

        // The helper is a Python script; run it through python3 so we do not
        // depend on the executable bit surviving a copy from Windows.
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(HELPER_TIMEOUT_SECS),
            Command::new("python3")
                .arg(&helper)
                .args(args)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output(),
        )
        .await
        .map_err(|_| {
            AppError::System("Таймаут обращения к серверам Google (45 сек)".to_string())
        })?;

        let out = result.map_err(|e| {
            AppError::System(format!("Не удалось запустить OAuth helper: {e}"))
        })?;

        let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();

        if stdout.is_empty() {
            return Err(AppError::System(format!(
                "OAuth helper не вернул данных. stderr: {}",
                if stderr.is_empty() { "пусто" } else { &stderr }
            )));
        }

        serde_json::from_str::<serde_json::Value>(&stdout).map_err(|e| {
            AppError::System(format!("Некорректный ответ OAuth helper: {e}"))
        })
    }

    /// Extract Google's error description from a helper payload.
    fn helper_error(payload: &serde_json::Value) -> String {
        let code = payload
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown_error");
        let desc = payload
            .get("error_description")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if desc.is_empty() {
            code.to_string()
        } else {
            format!("{code}: {desc}")
        }
    }

    /// Start the device flow.
    pub async fn begin(
        &self,
        client_id: &str,
        client_secret: &str,
    ) -> AppResult<DeviceCodeChallenge> {
        let cid = client_id.trim();
        let secret = client_secret.trim();
        if cid.is_empty() || secret.is_empty() {
            return Err(AppError::Validation(
                "Client ID и Client Secret обязательны".to_string(),
            ));
        }

        let payload = Self::run_helper(&["begin", cid, secret]).await?;

        if payload.get("device_code").is_none() {
            return Err(AppError::System(format!(
                "Google отклонил запрос кода: {}",
                Self::helper_error(&payload)
            )));
        }

        // Remember the client credentials next to the tokens so `refresh` works
        // without asking the panel again.
        let pending = YouTubeTokens {
            client_id: cid.to_string(),
            client_secret: secret.to_string(),
            refresh_token: String::new(),
            access_token: None,
            expires_at: None,
            scope: None,
            account: None,
        };
        self.store(&pending).await?;

        Ok(DeviceCodeChallenge {
            verification_url: payload
                .get("verification_url")
                .and_then(|v| v.as_str())
                .unwrap_or("https://www.google.com/device")
                .to_string(),
            user_code: payload
                .get("user_code")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            expires_in: payload.get("expires_in").and_then(|v| v.as_i64()),
            interval: payload.get("interval").and_then(|v| v.as_i64()),
            device_code: payload
                .get("device_code")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        })
    }

    /// Exchange the device code for tokens once the user has confirmed.
    pub async fn finish(&self, device_code: &str) -> AppResult<YouTubeAuthStatus> {
        let code = device_code.trim();
        if code.is_empty() {
            return Err(AppError::Validation(
                "Код подтверждения не может быть пустым".to_string(),
            ));
        }

        let existing = self.load().await?.ok_or_else(|| {
            AppError::Config(
                "Сначала начните авторизацию (client_id/client_secret не сохранены)"
                    .to_string(),
            )
        })?;

        let payload = Self::run_helper(&[
            "finish",
            &existing.client_id,
            &existing.client_secret,
            code,
        ])
        .await?;

        let refresh = payload
            .get("refresh_token")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if refresh.is_empty() {
            let err = Self::helper_error(&payload);
            // "authorization_pending" просто значит, что пользователь ещё не успел.
            return Err(AppError::Validation(format!(
                "Google ещё не подтвердил вход ({err}). Введите код на странице и повторите."
            )));
        }

        let expires_in = payload.get("expires_in").and_then(|v| v.as_i64());
        let tokens = YouTubeTokens {
            client_id: existing.client_id,
            client_secret: existing.client_secret,
            refresh_token: refresh.to_string(),
            access_token: payload
                .get("access_token")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            expires_at: expires_in.map(|s| chrono::Utc::now().timestamp() + s),
            scope: payload
                .get("scope")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            account: None,
        };

        self.store(&tokens).await?;
        self.write_cookies(&tokens).await?;

        info!("YouTube OAuth: авторизация завершена, токены сохранены");
        Ok(self.status().await)
    }

    /// Refresh the access token using the stored refresh token.
    pub async fn refresh(&self) -> AppResult<YouTubeAuthStatus> {
        let mut tokens = self.load().await?.ok_or_else(|| {
            AppError::Config("YouTube не авторизован".to_string())
        })?;

        if tokens.refresh_token.is_empty() {
            return Err(AppError::Config(
                "Нет refresh token — авторизация не завершена".to_string(),
            ));
        }

        let payload = Self::run_helper(&[
            "refresh",
            &tokens.client_id,
            &tokens.client_secret,
            &tokens.refresh_token,
        ])
        .await?;

        let access = payload.get("access_token").and_then(|v| v.as_str());
        let Some(access) = access else {
            return Err(AppError::System(format!(
                "Не удалось обновить токен: {}",
                Self::helper_error(&payload)
            )));
        };

        let expires_in = payload.get("expires_in").and_then(|v| v.as_i64());
        tokens.access_token = Some(access.to_string());
        tokens.expires_at = expires_in.map(|s| chrono::Utc::now().timestamp() + s);

        self.store(&tokens).await?;
        self.write_cookies(&tokens).await?;

        Ok(self.status().await)
    }

    /// Produce a fresh access token, refreshing if needed.
    pub async fn access_token(&self) -> AppResult<Option<String>> {
        let mut tokens = match self.load().await? {
            Some(t) => t,
            None => return Ok(None),
        };

        let now = chrono::Utc::now().timestamp();
        let still_valid = tokens
            .access_token
            .is_some()
            && tokens.expires_at.map(|e| e - 60 > now).unwrap_or(false);

        if still_valid {
            return Ok(tokens.access_token.clone());
        }

        if tokens.refresh_token.is_empty() {
            return Ok(None);
        }

        let payload = Self::run_helper(&[
            "refresh",
            &tokens.client_id,
            &tokens.client_secret,
            &tokens.refresh_token,
        ])
        .await?;

        if let Some(access) = payload.get("access_token").and_then(|v| v.as_str()) {
            let expires_in = payload.get("expires_in").and_then(|v| v.as_i64());
            tokens.access_token = Some(access.to_string());
            tokens.expires_at = expires_in.map(|s| now + s);
            self.store(&tokens).await?;
            self.write_cookies(&tokens).await?;
            Ok(Some(access.to_string()))
        } else {
            warn!("YouTube: обновление токена не удалось");
            Ok(None)
        }
    }

    /// Current status for the panel.
    pub async fn status(&self) -> YouTubeAuthStatus {
        let tokens = self.load().await.unwrap_or(None);
        let now = chrono::Utc::now().timestamp();

        let (authorized, has_client, access_valid, expires_in) = match tokens {
            Some(t) => {
                let authorized = !t.refresh_token.is_empty();
                let access_valid = t.access_token.is_some()
                    && t.expires_at.map(|e| e > now).unwrap_or(false);
                let expires_in = t.expires_at.map(|e| e - now);
                (authorized, !t.client_id.is_empty(), access_valid, expires_in)
            }
            None => (false, false, false, None),
        };

        let cookies_ready = tokio::fs::metadata(&self.cookies_path)
            .await
            .map(|m| m.len() > 0)
            .unwrap_or(false);

        let problem = if !has_client {
            Some("Не заданы Client ID / Client Secret приложения Google".to_string())
        } else if !authorized {
            Some("Аккаунт Google не привязан".to_string())
        } else {
            None
        };

        YouTubeAuthStatus {
            authorized,
            has_client,
            access_token_valid: access_valid,
            expires_in,
            token_path: self.token_path.display().to_string(),
            cookies_path: self.cookies_path.display().to_string(),
            cookies_ready,
            problem,
        }
    }

    /// Remove stored credentials and the generated cookies file.
    pub async fn logout(&self) -> AppResult<()> {
        let _ = tokio::fs::remove_file(&self.token_path).await;
        let _ = tokio::fs::remove_file(&self.cookies_path).await;
        info!("YouTube OAuth: учётные данные удалены");
        Ok(())
    }

    async fn load(&self) -> AppResult<Option<YouTubeTokens>> {
        match tokio::fs::read_to_string(&self.token_path).await {
            Ok(raw) => serde_json::from_str::<YouTubeTokens>(&raw)
                .map(Some)
                .map_err(|e| AppError::Config(format!("Повреждён файл токенов: {e}"))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(AppError::System(format!(
                "Не удалось прочитать токены: {e}"
            ))),
        }
    }

    async fn store(&self, tokens: &YouTubeTokens) -> AppResult<()> {
        if let Some(dir) = self.token_path.parent() {
            tokio::fs::create_dir_all(dir).await.map_err(|e| {
                AppError::System(format!("Не удалось создать {}: {e}", dir.display()))
            })?;
        }

        let body = serde_json::to_string_pretty(tokens)
            .map_err(|e| AppError::Config(format!("Сериализация токенов: {e}")))?;

        tokio::fs::write(&self.token_path, body)
            .await
            .map_err(|e| AppError::System(format!("Не удалось сохранить токены: {e}")))?;

        // Только владелец может читать файл с секретами.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = tokio::fs::set_permissions(
                &self.token_path,
                std::fs::Permissions::from_mode(0o600),
            )
            .await;
        }

        Ok(())
    }

    /// Render a Netscape-format cookies file understood by yt-dlp and friends.
    ///
    /// Google's OAuth access token is presented as the `SAPISID`/`__Secure-3PAPISID`
    /// style values that yt-dlp's youtube extractor accepts in `cookies.txt`.
    async fn write_cookies(&self, tokens: &YouTubeTokens) -> AppResult<()> {
        let Some(ref access) = tokens.access_token else {
            return Ok(());
        };

        if let Some(dir) = self.cookies_path.parent() {
            tokio::fs::create_dir_all(dir).await.ok();
        }

        // Netscape header, then OAuth-style auth cookies.
        let expires = tokens
            .expires_at
            .unwrap_or_else(|| chrono::Utc::now().timestamp() + 3600);

        let body = format!(
            "# Netscape HTTP Cookie File\n\
             # Generated by Kamish from a Google OAuth token. Do not edit.\n\
             .youtube.com\tTRUE\t/\tTRUE\t{expires}\tSAPISID\t{access}\n\
             .youtube.com\tTRUE\t/\tTRUE\t{expires}\t__Secure-3PAPISID\t{access}\n\
             .google.com\tTRUE\t/\tTRUE\t{expires}\tSAPISID\t{access}\n\
             .google.com\tTRUE\t/\tTRUE\t{expires}\t__Secure-3PAPISID\t{access}\n"
        );

        tokio::fs::write(&self.cookies_path, body)
            .await
            .map_err(|e| AppError::System(format!("Не удалось записать cookies: {e}")))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = tokio::fs::set_permissions(
                &self.cookies_path,
                std::fs::Permissions::from_mode(0o600),
            )
            .await;
        }

        Ok(())
    }
}

/// Convenience: path to a cookies file usable by yt-dlp, when authorized.
pub async fn cookies_for_ytdlp(auth: &YouTubeAuth) -> Option<PathBuf> {
    let status = auth.status().await;
    if status.authorized && status.cookies_ready {
        Some(auth.cookies_path().to_path_buf())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_status_without_token_reports_unauthorized() {
        let dir = tempdir().unwrap();
        let auth = YouTubeAuth::with_paths(
            dir.path().join("oauth.json"),
            dir.path().join("cookies.txt"),
        );

        let st = auth.status().await;
        assert!(!st.authorized);
        assert!(!st.has_client);
        assert!(!st.cookies_ready);
        assert!(st.problem.is_some());
    }

    #[tokio::test]
    async fn test_store_creates_file_and_status_reflects_it() {
        let dir = tempdir().unwrap();
        let auth = YouTubeAuth::with_paths(
            dir.path().join("oauth.json"),
            dir.path().join("cookies.txt"),
        );

        let tokens = YouTubeTokens {
            client_id: "cid".into(),
            client_secret: "secret".into(),
            refresh_token: "refresh".into(),
            access_token: Some("access".into()),
            expires_at: Some(chrono::Utc::now().timestamp() + 3600),
            scope: None,
            account: None,
        };

        auth.store(&tokens).await.unwrap();
        auth.write_cookies(&tokens).await.unwrap();

        let st = auth.status().await;
        assert!(st.authorized);
        assert!(st.has_client);
        assert!(st.access_token_valid);
        assert!(st.cookies_ready);

        let body = tokio::fs::read_to_string(auth.cookies_path()).await.unwrap();
        assert!(body.contains("Netscape HTTP Cookie File"));
        assert!(body.contains("SAPISID"));
        assert!(body.contains("access"));
    }

    #[tokio::test]
    async fn test_logout_removes_credentials_and_cookies() {
        let dir = tempdir().unwrap();
        let auth = YouTubeAuth::with_paths(
            dir.path().join("oauth.json"),
            dir.path().join("cookies.txt"),
        );

        let tokens = YouTubeTokens {
            client_id: "cid".into(),
            client_secret: "secret".into(),
            refresh_token: "refresh".into(),
            access_token: Some("access".into()),
            expires_at: Some(chrono::Utc::now().timestamp() + 60),
            scope: None,
            account: None,
        };
        auth.store(&tokens).await.unwrap();
        auth.write_cookies(&tokens).await.unwrap();

        auth.logout().await.unwrap();

        let st = auth.status().await;
        assert!(!st.authorized);
        assert!(!st.cookies_ready);
    }

    #[tokio::test]
    async fn test_begin_rejects_empty_credentials() {
        let dir = tempdir().unwrap();
        let auth = YouTubeAuth::with_paths(
            dir.path().join("oauth.json"),
            dir.path().join("cookies.txt"),
        );

        assert!(auth.begin("", "secret").await.is_err());
        assert!(auth.begin("cid", "  ").await.is_err());
    }

    #[tokio::test]
    async fn test_finish_requires_begin_first() {
        let dir = tempdir().unwrap();
        let auth = YouTubeAuth::with_paths(
            dir.path().join("oauth.json"),
            dir.path().join("cookies.txt"),
        );

        let err = auth.finish("some-code").await.unwrap_err();
        assert!(err.to_string().contains("Сначала начните авторизацию"));
    }

    #[tokio::test]
    async fn test_finish_rejects_empty_code() {
        let dir = tempdir().unwrap();
        let auth = YouTubeAuth::with_paths(
            dir.path().join("oauth.json"),
            dir.path().join("cookies.txt"),
        );
        assert!(auth.finish("").await.is_err());
    }

    #[tokio::test]
    async fn test_access_token_returns_none_when_not_authorized() {
        let dir = tempdir().unwrap();
        let auth = YouTubeAuth::with_paths(
            dir.path().join("oauth.json"),
            dir.path().join("cookies.txt"),
        );
        assert!(auth.access_token().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_cookies_are_written_with_owner_only_permissions() {
        let dir = tempdir().unwrap();
        let auth = YouTubeAuth::with_paths(
            dir.path().join("oauth.json"),
            dir.path().join("cookies.txt"),
        );
        let tokens = YouTubeTokens {
            client_id: "cid".into(),
            client_secret: "secret".into(),
            refresh_token: "refresh".into(),
            access_token: Some("access".into()),
            expires_at: Some(chrono::Utc::now().timestamp() + 3600),
            scope: None,
            account: None,
        };
        auth.store(&tokens).await.unwrap();
        auth.write_cookies(&tokens).await.unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let meta = std::fs::metadata(auth.cookies_path()).unwrap();
            let mode = meta.permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "cookies must not be world-readable");
        }
    }
}
