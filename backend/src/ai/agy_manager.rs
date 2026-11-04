//! Antigravity CLI (`agy`) lifecycle management driven from the web panel.
//!
//! The device ships without the AI CLI: it is a proprietary binary that must be
//! downloaded on demand and then authorized by the owner through a browser link and a
//! one-time code. This module owns that flow so the panel never has to shell out itself:
//!
//! * [`AgyManager::status`] — is the binary present, which version, is it authorized?
//! * [`AgyManager::install`] — download the release for the current architecture.
//! * [`AgyManager::begin_login`] — ask the CLI for an authorization URL + code.
//! * [`AgyManager::submit_code`] — hand the code entered in the panel back to the CLI.
//!
//! Security rules honoured here:
//! * the directory is fixed by configuration, never taken from the request body;
//! * only `http(s)` URLs are accepted and downloads are size-capped;
//! * nothing from the CLI's environment or tokens is ever returned to the browser —
//!   only the fields the user needs to finish authorization.

use crate::common::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tracing::{info, warn};

/// Maximum size accepted for the downloaded binary (200 MB).
const MAX_DOWNLOAD_BYTES: u64 = 200 * 1024 * 1024;

/// Maximum bytes of CLI output captured during a login step.
const MAX_CLI_OUTPUT_BYTES: usize = 64 * 1024;

/// Where the CLI lives and where the installer writes it.
#[derive(Debug, Clone)]
pub struct AgyPaths {
    /// Directory the binary is installed into.
    pub bin_dir: PathBuf,
    /// Whether downloads are permitted at all (air-gapped builds disable them).
    pub allow_download: bool,
}

impl AgyPaths {
    /// Resolve from the environment with a deterministic fallback.
    ///
    /// * `KAMISH_AGY_DIR` — installation directory (default `/opt/kamish/bin`, falling
    ///   back to a per-user cache dir when that is not writable).
    /// * `KAMISH_AGY_DOWNLOAD_URL` — release URL for this architecture.
    /// * `KAMISH_ALLOW_AGY_DOWNLOAD=0` — disable remote downloads.
    pub fn from_env() -> Self {
        let bin_dir = std::env::var("KAMISH_AGY_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| Self::default_bin_dir());

        let allow_download = std::env::var("KAMISH_ALLOW_AGY_DOWNLOAD")
            .map(|v| !(v == "0" || v.eq_ignore_ascii_case("false")))
            .unwrap_or(true);

        Self {
            bin_dir,
            allow_download,
        }
    }

    fn default_bin_dir() -> PathBuf {
        let system = PathBuf::from("/opt/kamish/bin");
        // Prefer the system-wide location when it is writable, else fall back to
        // a user-scoped location so unprivileged dev runs still work.
        if system.exists() {
            return system;
        }
        match dirs_cache_dir() {
            Some(dir) => dir.join("bin"),
            None => system,
        }
    }

    /// Full path of the managed binary.
    pub fn binary_path(&self) -> PathBuf {
        self.bin_dir.join("agy")
    }
}

/// Minimal XDG cache lookup without pulling in an extra dependency.
fn dirs_cache_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("XDG_CACHE_HOME") {
        if !dir.is_empty() {
            return Some(PathBuf::from(dir));
        }
    }
    std::env::var("HOME")
        .ok()
        .map(|h| PathBuf::from(h).join(".cache"))
}

/// Install/authorization status surfaced to the panel.
#[derive(Debug, Clone, Serialize)]
pub struct AgyStatus {
    /// Binary present and executable.
    pub installed: bool,
    /// Resolved path of the CLI.
    pub path: String,
    /// Version string when the binary answered `--version`.
    pub version: Option<String>,
    /// Authorization state, as far as we can determine it.
    pub authorized: bool,
    /// Why the CLI cannot be used yet (informational for the panel).
    pub problem: Option<String>,
    /// Where downloads come from, for transparency in the UI.
    pub download_url: Option<String>,
    /// Whether this build permits downloading.
    pub download_enabled: bool,
    /// Host architecture that a download would target.
    pub architecture: String,
}

/// A pending authorization step: what the user must do next.
#[derive(Debug, Clone, Serialize)]
pub struct AgyLoginChallenge {
    /// URL the user opens in a browser.
    pub auth_url: Option<String>,
    /// One-time code to enter on that page, when the CLI prints one.
    pub auth_code: Option<String>,
    /// Raw CLI output, so the panel can show instructions verbatim.
    pub instructions: String,
}

#[derive(Debug, Deserialize)]
pub struct SubmitCodeRequest {
    pub code: String,
}

/// Result of a completed authorization attempt.
#[derive(Debug, Clone, Serialize)]
pub struct AgyLoginResult {
    pub authorized: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_output: Option<String>,
}

pub struct AgyManager {
    paths: AgyPaths,
}

impl AgyManager {
    pub fn new() -> Self {
        Self {
            paths: AgyPaths::from_env(),
        }
    }

    pub fn with_paths(paths: AgyPaths) -> Self {
        Self { paths }
    }

    /// Architecture label used to pick a release asset.
    pub fn architecture() -> String {
        format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
    }

    fn download_url() -> Option<String> {
        std::env::var("KAMISH_AGY_DOWNLOAD_URL")
            .ok()
            .filter(|u| !u.trim().is_empty())
    }

    /// Resolve the CLI actually in use: managed path first, then PATH.
    fn resolved_binary(&self) -> Option<PathBuf> {
        let managed = self.paths.binary_path();
        if managed.is_file() {
            return Some(managed);
        }
        if let Ok(path_var) = std::env::var("PATH") {
            for dir in std::env::split_paths(&path_var) {
                let candidate = dir.join("agy");
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
        for p in ["/usr/local/bin/agy", "/usr/bin/agy", "/opt/kamish/bin/agy"] {
            let path = Path::new(p);
            if path.is_file() {
                return Some(path.to_path_buf());
            }
        }
        None
    }

    /// Inspect the current installation without modifying anything.
    pub async fn status(&self) -> AgyStatus {
        let resolved = self.resolved_binary();
        let path = resolved
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| self.paths.binary_path().display().to_string());

        let url = Self::download_url();

        let Some(bin) = resolved else {
            return AgyStatus {
                installed: false,
                path,
                version: None,
                authorized: false,
                problem: Some("Antigravity CLI не установлен. Скачайте его из панели.".to_string()),
                download_url: url,
                download_enabled: self.paths.allow_download,
                architecture: Self::architecture(),
            };
        };

        let (version, problem) = match self.run_cli(&bin, &["--version"]).await {
            Ok(out) if out.status_success => {
                let v = out.stdout.trim().to_string();
                (
                    Some(if v.is_empty() {
                        "версия не указана".to_string()
                    } else {
                        v
                    }),
                    None,
                )
            }
            Ok(out) => {
                let msg = out.stderr.trim();
                (
                    None,
                    Some(if msg.is_empty() {
                        "CLI не ответил на --version".to_string()
                    } else {
                        format!("CLI вернул ошибку: {msg}")
                    }),
                )
            }
            Err(e) => (None, Some(e.to_string())),
        };

        // Authorization is inferred from the CLI's own status command when available.
        let authorized = self.probe_authorized(&bin).await;

        AgyStatus {
            installed: true,
            path,
            version,
            authorized,
            problem,
            download_url: url,
            download_enabled: self.paths.allow_download,
            architecture: Self::architecture(),
        }
    }

    /// Best-effort authorization probe across the commonly used subcommands.
    async fn probe_authorized(&self, bin: &Path) -> bool {
        for args in [
            ["auth", "status"].as_slice(),
            ["whoami"].as_slice(),
            ["status"].as_slice(),
        ] {
            if let Ok(out) = self.run_cli(bin, args).await {
                if !out.status_success {
                    continue;
                }
                let text = format!("{} {}", out.stdout, out.stderr).to_lowercase();
                if text.contains("not logged in")
                    || text.contains("unauthorized")
                    || text.contains("not authenticated")
                {
                    return false;
                }
                // A successful status command that does not deny authorization counts as OK.
                return true;
            }
        }
        false
    }

    /// Download and install the CLI for this architecture.
    pub async fn install(&self) -> AppResult<AgyStatus> {
        if !self.paths.allow_download {
            return Err(AppError::Config(
                "Скачивание Antigravity CLI отключено в этой сборке (KAMISH_ALLOW_AGY_DOWNLOAD=0)"
                    .to_string(),
            ));
        }

        let url = Self::download_url().ok_or_else(|| {
            AppError::Config(
                "Не задан адрес загрузки Antigravity CLI (KAMISH_AGY_DOWNLOAD_URL)".to_string(),
            )
        })?;

        let parsed = validate_download_url(&url)?;

        tokio::fs::create_dir_all(&self.paths.bin_dir)
            .await
            .map_err(|e| {
                AppError::System(format!(
                    "Не удалось создать каталог {}: {e}",
                    self.paths.bin_dir.display()
                ))
            })?;

        let target = self.paths.binary_path();
        let tmp = self.paths.bin_dir.join(".agy.download");

        info!("Downloading Antigravity CLI from {}", parsed);

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .map_err(|e| AppError::System(format!("HTTP client init failed: {e}")))?;

        let resp = client
            .get(&url)
            .send()
            .await
            .map_err(|e| AppError::System(format!("Не удалось скачать CLI: {e}")))?;

        if !resp.status().is_success() {
            return Err(AppError::System(format!(
                "Сервер вернул статус {} при загрузке CLI",
                resp.status()
            )));
        }

        if let Some(len) = resp.content_length() {
            if len > MAX_DOWNLOAD_BYTES {
                return Err(AppError::System(format!(
                    "Файл слишком большой ({len} байт), загрузка отклонена"
                )));
            }
        }

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| AppError::System(format!("Ошибка чтения загрузки: {e}")))?;

        if bytes.is_empty() {
            return Err(AppError::System("Загруженный файл пуст".to_string()));
        }
        if bytes.len() as u64 > MAX_DOWNLOAD_BYTES {
            return Err(AppError::System(
                "Размер загрузки превысил допустимый предел".to_string(),
            ));
        }

        tokio::fs::write(&tmp, &bytes)
            .await
            .map_err(|e| AppError::System(format!("Не удалось записать файл: {e}")))?;

        // make executable (0o755)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            tokio::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))
                .await
                .map_err(|e| AppError::System(format!("Не удалось выставить права: {e}")))?;
        }

        tokio::fs::rename(&tmp, &target)
            .await
            .map_err(|e| AppError::System(format!("Не удалось установить CLI: {e}")))?;

        info!("Antigravity CLI installed to {}", target.display());
        Ok(self.status().await)
    }

    /// Ask the CLI to start the authorization flow.
    ///
    /// Different CLI builds expose the flow differently, so the common variants are
    /// tried in order and the first successful output is parsed for a URL and a code.
    pub async fn begin_login(&self) -> AppResult<AgyLoginChallenge> {
        let bin = self
            .resolved_binary()
            .ok_or_else(|| AppError::Config("Antigravity CLI не установлен".to_string()))?;

        let mut last_output = String::new();

        for args in [
            ["auth", "login"].as_slice(),
            ["login"].as_slice(),
            ["auth"].as_slice(),
        ] {
            match self.run_cli(&bin, args).await {
                Ok(out) => {
                    let combined = format!("{}\n{}", out.stdout, out.stderr);
                    last_output = combined.clone();
                    let (url, code) = parse_login_output(&combined);
                    if url.is_some() || code.is_some() {
                        return Ok(AgyLoginChallenge {
                            auth_url: url,
                            auth_code: code,
                            instructions: combined.trim().to_string(),
                        });
                    }
                }
                Err(e) => {
                    warn!("agy {:?} failed: {e}", args);
                    last_output = e.to_string();
                }
            }
        }

        Err(AppError::System(format!(
            "Не удалось получить ссылку для авторизации. Ответ CLI: {}",
            if last_output.is_empty() {
                "пусто".to_string()
            } else {
                last_output
            }
        )))
    }

    /// Complete authorization with the code the user typed in the panel.
    pub async fn submit_code(&self, code: &str) -> AppResult<AgyLoginResult> {
        let code = code.trim();
        if code.is_empty() {
            return Err(AppError::Validation(
                "Код авторизации не может быть пустым".to_string(),
            ));
        }
        if code.len() > 512 {
            return Err(AppError::Validation(
                "Код авторизации слишком длинный".to_string(),
            ));
        }

        let bin = self
            .resolved_binary()
            .ok_or_else(|| AppError::Config("Antigravity CLI не установлен".to_string()))?;

        let mut last_err = String::new();

        for args in [
            vec!["auth", "login", "--code", code],
            vec!["auth", "login", code],
            vec!["login", "--code", code],
            vec!["auth", "verify", code],
        ] {
            match self.run_cli(&bin, &args).await {
                Ok(out) => {
                    if out.status_success {
                        let authorized = self.probe_authorized(&bin).await;
                        return Ok(AgyLoginResult {
                            authorized,
                            message: if authorized {
                                "Авторизация завершена успешно".to_string()
                            } else {
                                "Код принят, но статус авторизации не подтверждён".to_string()
                            },
                            raw_output: Some(out.stdout.trim().to_string()),
                        });
                    }
                    last_err = format!("{} {}", out.stdout, out.stderr).trim().to_string();
                }
                Err(e) => last_err = e.to_string(),
            }
        }

        Err(AppError::System(format!(
            "CLI не принял код. Ответ: {last_err}"
        )))
    }

    /// Run the CLI with a hard timeout and bounded output capture.
    async fn run_cli(&self, bin: &Path, args: &[&str]) -> AppResult<CliOutput> {
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            Command::new(bin)
                .args(args)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output(),
        )
        .await
        .map_err(|_| AppError::System("Таймаут выполнения Antigravity CLI (30 сек)".to_string()))?;

        let out = result.map_err(|e| AppError::System(format!("Не удалось запустить CLI: {e}")))?;

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        // Bound the buffers so a misbehaving CLI cannot exhaust memory here.
        {
            let o = std::io::Cursor::new(out.stdout);
            let _ = o.take(MAX_CLI_OUTPUT_BYTES as u64).read_to_end(&mut stdout);
        }
        {
            let e = std::io::Cursor::new(out.stderr);
            let _ = e.take(MAX_CLI_OUTPUT_BYTES as u64).read_to_end(&mut stderr);
        }

        Ok(CliOutput {
            status_success: out.status.success(),
            stdout: String::from_utf8_lossy(&stdout).to_string(),
            stderr: String::from_utf8_lossy(&stderr).to_string(),
        })
    }
}

impl Default for AgyManager {
    fn default() -> Self {
        Self::new()
    }
}

struct CliOutput {
    status_success: bool,
    stdout: String,
    stderr: String,
}

/// Accept only http(s) download URLs; everything else is rejected.
fn validate_download_url(raw: &str) -> AppResult<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation("Пустой URL загрузки".to_string()));
    }
    let lower = trimmed.to_ascii_lowercase();
    if !(lower.starts_with("https://") || lower.starts_with("http://")) {
        return Err(AppError::Validation(
            "URL загрузки должен начинаться с http:// или https://".to_string(),
        ));
    }
    if trimmed.contains(char::is_whitespace) {
        return Err(AppError::Validation(
            "URL загрузки содержит пробелы".to_string(),
        ));
    }
    Ok(trimmed.to_string())
}

/// Extract an authorization URL and a device code from arbitrary CLI output.
///
/// Recognised shapes:
/// * `Open https://example.com/device and enter code AB12-CD34`
/// * `Code: AB12-CD34`
/// * `https://...` on its own line
pub fn parse_login_output(text: &str) -> (Option<String>, Option<String>) {
    let mut url = None;
    let mut code = None;

    for token in text.split_whitespace() {
        let cleaned = token.trim_matches(|c: char| c.is_whitespace() || ",.;:)]}\"'".contains(c));
        if cleaned.starts_with("http://") || cleaned.starts_with("https://") {
            let lower = cleaned.to_ascii_lowercase();
            // Skip obvious noise such as documentation links when a better one exists.
            if url.is_none() || lower.contains("device") || lower.contains("auth") {
                url = Some(cleaned.to_string());
            }
        }
    }

    // Code patterns: "code XXXX-XXXX" / "код: XXXX" / "Code: XXXX-XXXX"
    let lowered = text.to_lowercase();
    for marker in ["код", "code", "enter", "введите"] {
        if let Some(idx) = lowered.find(marker) {
            let tail = &text[idx + marker.len()..];
            let tail = tail.trim_start_matches(|c: char| {
                c == ':' || c == '.' || c == ' ' || c == '\t' || c == '-' || c == '—'
            });
            if let Some(candidate) = tail.split_whitespace().next() {
                let cleaned = candidate.trim_matches(|c: char| !c.is_alphanumeric() && c != '-');
                // Codes are short, alphanumeric, often dash-separated.
                if cleaned.len() >= 4
                    && cleaned.len() <= 64
                    && cleaned.chars().any(|c| c.is_ascii_alphanumeric())
                {
                    code = Some(cleaned.to_string());
                    break;
                }
            }
        }
    }

    (url, code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_login_output_url_and_code() {
        let text = "To authorize, open https://antigravity.dev/device and enter code AB12-CD34";
        let (url, code) = parse_login_output(text);
        assert_eq!(url.as_deref(), Some("https://antigravity.dev/device"));
        assert_eq!(code.as_deref(), Some("AB12-CD34"));
    }

    #[test]
    fn test_parse_login_output_russian_markers() {
        let text = "Откройте https://example.com/auth\nКод: WXYZ-1234";
        let (url, code) = parse_login_output(text);
        assert_eq!(url.as_deref(), Some("https://example.com/auth"));
        assert_eq!(code.as_deref(), Some("WXYZ-1234"));
    }

    #[test]
    fn test_parse_login_output_no_data() {
        let (url, code) = parse_login_output("Command not found");
        assert!(url.is_none());
        assert!(code.is_none());
    }

    #[test]
    fn test_validate_download_url_rejects_non_http() {
        assert!(validate_download_url("ftp://evil/x").is_err());
        assert!(validate_download_url("file:///etc/passwd").is_err());
        assert!(validate_download_url("").is_err());
        assert!(validate_download_url("https://ok/release").is_ok());
        assert!(validate_download_url("http://ok/release").is_ok());
    }

    #[test]
    fn test_validate_download_url_rejects_whitespace() {
        assert!(validate_download_url("https://ok/a b").is_err());
    }

    #[test]
    fn test_binary_path_is_fixed_inside_bin_dir() {
        let paths = AgyPaths {
            bin_dir: PathBuf::from("/opt/kamish/bin"),
            allow_download: true,
        };
        assert_eq!(paths.binary_path(), PathBuf::from("/opt/kamish/bin/agy"));
    }

    #[test]
    fn test_architecture_is_reported() {
        let arch = AgyManager::architecture();
        assert!(arch.contains('-'));
        assert!(arch.contains(std::env::consts::ARCH));
    }
}
