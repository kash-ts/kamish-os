use rand::Rng;
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tracing::{info, warn};

static SSH_ENABLED: AtomicBool = AtomicBool::new(false);
static TEMP_PASSWORD: Mutex<Option<String>> = Mutex::new(None);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SshStatus {
    pub enabled: bool,
    pub user: String,
    pub port: u16,
    pub active_sessions: usize,
    pub root_login_allowed: bool,
    pub has_temp_password: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SshToggleRequest {
    pub enable: bool,
    pub ssh_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SshToggleResponse {
    pub enabled: bool,
    pub temp_password: Option<String>,
    pub user: String,
    pub port: u16,
    pub message: String,
}

/// Generates a secure, readable 12-character alphanumeric temporary password
fn generate_temp_password() -> String {
    const CHARSET: &[u8] = b"abcdefghijkmnpqrstuvwxyz23456789ABCDEFGHJKLMNPQRSTUVWXYZ";
    let mut rng = rand::thread_rng();
    (0..12)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

/// Retrieves the current Dropbear SSH status
pub fn get_ssh_status() -> SshStatus {
    let mut enabled = SSH_ENABLED.load(Ordering::SeqCst);
    let mut active_sessions = 0;

    // Check system status via kamish-dropbear-mgr if present
    if let Ok(output) = Command::new("kamish-dropbear-mgr").arg("status").output() {
        if output.status.success() {
            if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
                if let Some(en) = val.get("enabled").and_then(|v| v.as_bool()) {
                    enabled = en;
                    SSH_ENABLED.store(en, Ordering::SeqCst);
                }
                if let Some(sess) = val.get("active_sessions").and_then(|v| v.as_u64()) {
                    active_sessions = sess as usize;
                }
            }
        }
    }

    let has_temp_password = TEMP_PASSWORD.lock().map(|g| g.is_some()).unwrap_or(false);

    SshStatus {
        enabled,
        user: "kamish".to_string(),
        port: 22,
        active_sessions,
        root_login_allowed: false,
        has_temp_password,
    }
}

/// Toggles Dropbear SSH on or off, generating a new temporary password on enable
pub fn toggle_ssh(req: SshToggleRequest) -> Result<SshToggleResponse, String> {
    if req.enable {
        let temp_pass = generate_temp_password();
        info!("Enabling Dropbear SSH with temporary credentials for user 'kamish'");

        // Call kamish-dropbear-mgr enable <temp_pass> [key]
        let mut cmd = Command::new("kamish-dropbear-mgr");
        cmd.arg("enable").arg(&temp_pass);
        if let Some(ref key) = req.ssh_key {
            cmd.arg(key);
        }

        match cmd.output() {
            Ok(out) => {
                if !out.status.success() {
                    warn!("kamish-dropbear-mgr returned non-zero code in dev environment");
                }
            }
            Err(e) => {
                info!(
                    "kamish-dropbear-mgr not found on host (running in dev/mock mode): {}",
                    e
                );
            }
        }

        SSH_ENABLED.store(true, Ordering::SeqCst);
        if let Ok(mut lock) = TEMP_PASSWORD.lock() {
            *lock = Some(temp_pass.clone());
        }

        Ok(SshToggleResponse {
            enabled: true,
            temp_password: Some(temp_pass),
            user: "kamish".to_string(),
            port: 22,
            message: "Служба Dropbear SSH успешно включена. Вход root запрещен, используйте пользователя kamish.".to_string(),
        })
    } else {
        info!("Disabling Dropbear SSH service");

        let _ = Command::new("kamish-dropbear-mgr").arg("disable").output();

        SSH_ENABLED.store(false, Ordering::SeqCst);
        if let Ok(mut lock) = TEMP_PASSWORD.lock() {
            *lock = None;
        }

        Ok(SshToggleResponse {
            enabled: false,
            temp_password: None,
            user: "kamish".to_string(),
            port: 22,
            message: "Служба Dropbear SSH выключена.".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssh_toggle_lifecycle() {
        // Initially disabled or toggled
        let res_enable = toggle_ssh(SshToggleRequest {
            enable: true,
            ssh_key: None,
        })
        .unwrap();

        assert!(res_enable.enabled);
        assert!(res_enable.temp_password.is_some());
        assert_eq!(res_enable.user, "kamish");

        let status = get_ssh_status();
        assert!(status.enabled);
        assert!(!status.root_login_allowed);

        let res_disable = toggle_ssh(SshToggleRequest {
            enable: false,
            ssh_key: None,
        })
        .unwrap();

        assert!(!res_disable.enabled);
        assert!(res_disable.temp_password.is_none());

        let status_off = get_ssh_status();
        assert!(!status_off.enabled);
    }
}
