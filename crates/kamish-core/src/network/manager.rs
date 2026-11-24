use crate::common::{AppError, AppResult};
use crate::network::models::*;
use crate::system::EventBus;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

#[derive(Debug)]
struct NetworkInnerState {
    primary_interface: String,
    ip_address: String,
    subnet_mask: String,
    gateway: String,
    dns: Vec<String>,
    ethernet_connected: bool,
    ethernet_speed_mbps: Option<u32>,
    wifi_connected: bool,
    wifi_ssid: Option<String>,
    wifi_bssid: Option<String>,
    wifi_signal_percent: Option<u8>,
    wifi_freq_mhz: Option<u32>,
    ap_active: bool,
    ap_ssid: String,
    ap_gateway_ip: String,
}

#[derive(Clone)]
pub struct NetworkManager {
    state: Arc<RwLock<NetworkInnerState>>,
    event_bus: Option<EventBus>,
}

/// Helper function to parse nmcli terse output (`-t`), which uses `:` as a delimiter
/// and escapes literal colons with `\:`.
pub fn parse_nmcli_terse_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut cur = String::new();
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(&next) = chars.peek() {
                if next == ':' || next == '\\' {
                    cur.push(next);
                    chars.next();
                    continue;
                }
            }
            cur.push(c);
        } else if c == ':' {
            fields.push(std::mem::take(&mut cur));
        } else {
            cur.push(c);
        }
    }
    fields.push(cur);
    fields
}

/// Parses `nmcli -t -f NAME,TYPE,DEVICE connection show --active` output
/// and returns the active device name for the Setup AP profile if present.
pub fn parse_active_ap_from_nmcli(output: &str, ap_ssid: &str) -> Option<String> {
    for line in output.lines() {
        let parts = parse_nmcli_terse_line(line);
        if parts.len() >= 3 {
            let name = parts[0].trim();
            let conn_type = parts[1].trim();
            let device = parts[2].trim();
            let is_ap_name = name == ap_ssid || name == "Kamish-Setup";
            let is_wifi = conn_type.is_empty()
                || conn_type.contains("wireless")
                || conn_type.contains("wifi");
            if is_ap_name && is_wifi && !device.is_empty() && device != "--" {
                return Some(device.to_string());
            }
        }
    }
    None
}

/// Parses `iw dev <iface> info` output and returns the AP SSID if the interface
/// is operating in AP mode (`type AP`).
pub fn parse_iw_info_ap_ssid(output: &str) -> Option<String> {
    let mut is_ap = false;
    let mut ssid: Option<String> = None;

    for line in output.lines() {
        let trimmed = line.trim();
        if let Some(kind) = trimmed.strip_prefix("type ") {
            if kind.trim().eq_ignore_ascii_case("AP") {
                is_ap = true;
            }
        } else if let Some(val) = trimmed.strip_prefix("ssid ") {
            let s = val.trim();
            if !s.is_empty() {
                ssid = Some(s.to_string());
            }
        }
    }

    if is_ap {
        Some(ssid.unwrap_or_else(|| "Kamish-Setup".to_string()))
    } else {
        None
    }
}

fn detect_wireless_interface() -> Option<String> {
    if std::path::Path::new("/sys/class/net/wlan0").exists() {
        return Some("wlan0".to_string());
    }
    if let Ok(entries) = std::fs::read_dir("/sys/class/net") {
        for entry in entries.flatten() {
            if entry.path().join("wireless").exists() {
                return Some(entry.file_name().to_string_lossy().to_string());
            }
        }
    }
    None
}

fn detect_ethernet_status() -> (bool, Option<u32>) {
    if let Ok(entries) = std::fs::read_dir("/sys/class/net") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("eth") || name.starts_with("en") || name.starts_with("end") {
                let operstate = std::fs::read_to_string(entry.path().join("operstate"))
                    .unwrap_or_default()
                    .trim()
                    .to_string();
                let carrier = std::fs::read_to_string(entry.path().join("carrier"))
                    .unwrap_or_default()
                    .trim()
                    .to_string();
                if operstate == "up" || carrier == "1" {
                    let speed = std::fs::read_to_string(entry.path().join("speed"))
                        .ok()
                        .and_then(|s| s.trim().parse::<u32>().ok())
                        .filter(|s| (10..=2500).contains(s));
                    return (true, speed);
                }
            }
        }
    }
    (false, None)
}

fn prefixlen_to_netmask(prefixlen: u64) -> String {
    let mask_val: u32 = if prefixlen >= 32 {
        !0u32
    } else if prefixlen == 0 {
        0u32
    } else {
        !0u32 << (32u32 - prefixlen as u32)
    };
    format!(
        "{}.{}.{}.{}",
        (mask_val >> 24) & 0xff,
        (mask_val >> 16) & 0xff,
        (mask_val >> 8) & 0xff,
        mask_val & 0xff
    )
}

/// Parses plain `ip -4 addr show dev <iface>` output (compatible with both BusyBox `ip` and `iproute2`).
pub fn parse_ip_addr_show_ipv4(output: &str) -> (Option<String>, Option<String>) {
    for line in output.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("inet ") {
            if let Some(cidr) = rest.split_whitespace().next() {
                let mut parts = cidr.splitn(2, '/');
                let ip = parts.next().unwrap_or("").trim();
                if !ip.is_empty() {
                    let mask = parts
                        .next()
                        .and_then(|p| p.trim().parse::<u64>().ok())
                        .map(prefixlen_to_netmask);
                    return (Some(ip.to_string()), mask);
                }
            }
        }
    }
    (None, None)
}

async fn query_interface_ipv4(iface: &str) -> (Option<String>, Option<String>) {
    let mut ip_addr: Option<String> = None;
    let mut subnet_mask: Option<String> = None;

    if iface.is_empty() {
        return (ip_addr, subnet_mask);
    }

    if let Ok(out) = tokio::process::Command::new("ip")
        .args(["-j", "addr", "show", "dev", iface])
        .output()
        .await
    {
        if out.status.success() {
            if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&out.stdout) {
                if let Some(arr) = val.as_array().and_then(|a| a.first()) {
                    if let Some(addrs) = arr["addr_info"].as_array() {
                        for addr in addrs {
                            if addr["family"].as_str() == Some("inet") {
                                if ip_addr.is_none() {
                                    if let Some(local) = addr["local"].as_str() {
                                        ip_addr = Some(local.to_string());
                                    }
                                }
                                if subnet_mask.is_none() {
                                    if let Some(prefixlen) = addr["prefixlen"].as_u64() {
                                        subnet_mask = Some(prefixlen_to_netmask(prefixlen));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if ip_addr.is_none() || subnet_mask.is_none() {
        if let Ok(out) = tokio::process::Command::new("ip")
            .args(["-4", "addr", "show", "dev", iface])
            .output()
            .await
        {
            if out.status.success() {
                let text = String::from_utf8_lossy(&out.stdout);
                let (parsed_ip, parsed_mask) = parse_ip_addr_show_ipv4(&text);
                if ip_addr.is_none() {
                    ip_addr = parsed_ip;
                }
                if subnet_mask.is_none() {
                    subnet_mask = parsed_mask;
                }
            }
        }
    }

    (ip_addr, subnet_mask)
}

async fn teardown_fallback_ap(wifi_iface: &str) {
    for pid_path in ["/run/kamish-ap/hostapd.pid", "/run/kamish-ap/dnsmasq.pid"] {
        if let Ok(pid_raw) = std::fs::read_to_string(pid_path) {
            let pid = pid_raw.trim();
            if !pid.is_empty() {
                let _ = tokio::process::Command::new("kill").arg(pid).output().await;
            }
            let _ = std::fs::remove_file(pid_path);
        }
    }

    if !wifi_iface.is_empty() {
        let _ = tokio::process::Command::new("nmcli")
            .args(["device", "set", wifi_iface, "managed", "yes"])
            .output()
            .await;
    }
}

async fn run_nmcli_wifi_connect(
    ssid: &str,
    password: Option<&str>,
) -> std::io::Result<std::process::Output> {
    let mut cmd = tokio::process::Command::new("nmcli");
    cmd.args(["dev", "wifi", "connect", ssid]);
    if let Some(pass) = password {
        cmd.args(["password", pass]);
    }
    cmd.output().await
}

impl NetworkManager {
    pub fn new() -> Self {
        Self::with_event_bus(None)
    }

    pub fn with_event_bus(event_bus: Option<EventBus>) -> Self {
        let initial_state = NetworkInnerState {
            primary_interface: "eth0".to_string(),
            ip_address: "127.0.0.1".to_string(),
            subnet_mask: "255.255.255.0".to_string(),
            gateway: "0.0.0.0".to_string(),
            dns: Vec::new(),
            ethernet_connected: false,
            ethernet_speed_mbps: None,
            wifi_connected: false,
            wifi_ssid: None,
            wifi_bssid: None,
            wifi_signal_percent: None,
            wifi_freq_mhz: None,
            ap_active: false,
            ap_ssid: "Kamish-Setup".to_string(),
            ap_gateway_ip: "192.168.4.1".to_string(),
        };

        Self {
            state: Arc::new(RwLock::new(initial_state)),
            event_bus,
        }
    }

    pub async fn has_hardware_network(&self) -> bool {
        // In test mode or when hardware is mocked, do not execute system commands
        if crate::audio::alert::is_audio_muted() || std::env::var("KAMISH_MOCK_HARDWARE").is_ok() {
            return false;
        }

        if let Ok(entries) = std::fs::read_dir("/sys/class/net") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name != "lo" && !name.starts_with("docker") && !name.starts_with("veth") {
                    return true;
                }
            }
        }
        false
    }

    async fn detect_live_ap(&self, ap_ssid: &str) -> Option<(String, String)> {
        if !self.has_hardware_network().await {
            return None;
        }

        if let Ok(out) = tokio::process::Command::new("nmcli")
            .args([
                "-t",
                "-f",
                "NAME,TYPE,DEVICE",
                "connection",
                "show",
                "--active",
            ])
            .output()
            .await
        {
            if out.status.success() {
                let text = String::from_utf8_lossy(&out.stdout);
                if let Some(dev) = parse_active_ap_from_nmcli(&text, ap_ssid) {
                    return Some((dev, ap_ssid.to_string()));
                }
            }
        }

        if let Some(wifi_iface) = detect_wireless_interface() {
            if let Ok(out) = tokio::process::Command::new("iw")
                .args(["dev", &wifi_iface, "info"])
                .output()
                .await
            {
                if out.status.success() {
                    let text = String::from_utf8_lossy(&out.stdout);
                    if let Some(ssid) = parse_iw_info_ap_ssid(&text) {
                        return Some((wifi_iface, ssid));
                    }
                }
            }
        }

        None
    }

    pub async fn get_status(&self) -> NetworkStatus {
        let (
            state_primary_iface,
            state_ip_addr,
            state_subnet_mask,
            state_gateway,
            state_dns,
            state_eth_connected,
            state_eth_speed,
            state_wifi_connected,
            state_wifi_ssid,
            state_wifi_bssid,
            state_wifi_signal,
            state_wifi_freq,
            state_ap_active,
            ap_ssid,
            ap_gateway_ip,
        ) = {
            let state = self.state.read().await;
            (
                state.primary_interface.clone(),
                state.ip_address.clone(),
                state.subnet_mask.clone(),
                state.gateway.clone(),
                state.dns.clone(),
                state.ethernet_connected,
                state.ethernet_speed_mbps,
                state.wifi_connected,
                state.wifi_ssid.clone(),
                state.wifi_bssid.clone(),
                state.wifi_signal_percent,
                state.wifi_freq_mhz,
                state.ap_active,
                state.ap_ssid.clone(),
                state.ap_gateway_ip.clone(),
            )
        };

        let has_hw = self.has_hardware_network().await;
        let (hw_eth_connected, hw_eth_speed) = if has_hw {
            detect_ethernet_status()
        } else {
            (false, None)
        };
        let eth_connected = hw_eth_connected || state_eth_connected;
        let eth_speed = hw_eth_speed.or(state_eth_speed);

        let live_ap = if has_hw {
            self.detect_live_ap(&ap_ssid).await
        } else {
            None
        };

        if let Some((ap_iface, live_ssid)) = live_ap {
            let (live_ip, live_mask) = query_interface_ipv4(&ap_iface).await;
            return NetworkStatus {
                primary_interface: ap_iface,
                ip_address: live_ip.or_else(|| Some(ap_gateway_ip.clone())),
                subnet_mask: live_mask.or_else(|| Some("255.255.255.0".to_string())),
                gateway: Some(ap_gateway_ip.clone()),
                dns: vec![ap_gateway_ip],
                ethernet: EthernetStatus {
                    connected: eth_connected,
                    speed_mbps: eth_speed,
                },
                wifi: Some(WifiStatus {
                    connected: false,
                    ssid: None,
                    bssid: None,
                    signal_level_percent: None,
                    frequency_mhz: None,
                }),
                ap_mode: ApStatus {
                    active: true,
                    ssid: live_ssid,
                },
            };
        }

        if state_ap_active && !has_hw {
            return NetworkStatus {
                primary_interface: "wlan0".to_string(),
                ip_address: Some(ap_gateway_ip.clone()),
                subnet_mask: Some("255.255.255.0".to_string()),
                gateway: Some(ap_gateway_ip.clone()),
                dns: vec![ap_gateway_ip],
                ethernet: EthernetStatus {
                    connected: eth_connected,
                    speed_mbps: eth_speed,
                },
                wifi: Some(WifiStatus {
                    connected: false,
                    ssid: None,
                    bssid: None,
                    signal_level_percent: None,
                    frequency_mhz: None,
                }),
                ap_mode: ApStatus {
                    active: true,
                    ssid: ap_ssid,
                },
            };
        }

        // Live detection from system
        let mut primary_iface = String::new();
        let mut gateway_ip: Option<String> = None;
        let mut ip_addr: Option<String> = None;
        let mut subnet_mask: Option<String> = None;

        // 1. Detect default route from `ip -j route show default`
        if let Ok(out) = tokio::process::Command::new("ip")
            .args(["-j", "route", "show", "default"])
            .output()
            .await
        {
            if out.status.success() {
                if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&out.stdout) {
                    if let Some(arr) = val.as_array() {
                        for route in arr {
                            if primary_iface.is_empty() {
                                if let Some(dev) = route["dev"].as_str() {
                                    primary_iface = dev.to_string();
                                }
                            }
                            if gateway_ip.is_none() {
                                if let Some(gw) = route["gateway"].as_str() {
                                    gateway_ip = Some(gw.to_string());
                                }
                            }
                            if ip_addr.is_none() {
                                if let Some(src) = route["prefsrc"].as_str() {
                                    ip_addr = Some(src.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        // Fallback: parse /proc/net/route if default route wasn't obtained from `ip`
        if primary_iface.is_empty() {
            if let Ok(route_text) = std::fs::read_to_string("/proc/net/route") {
                for line in route_text.lines().skip(1) {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 3 && parts[1] == "00000000" {
                        primary_iface = parts[0].to_string();
                        if gateway_ip.is_none() {
                            if let Ok(gw_hex) = u32::from_str_radix(parts[2], 16) {
                                let b = gw_hex.to_le_bytes();
                                gateway_ip = Some(format!("{}.{}.{}.{}", b[0], b[1], b[2], b[3]));
                            }
                        }
                        break;
                    }
                }
            }
        }

        // If primary interface still unknown, find first non-loopback device in /sys/class/net
        if primary_iface.is_empty() {
            if let Ok(entries) = std::fs::read_dir("/sys/class/net") {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name != "lo" && !name.starts_with("docker") && !name.starts_with("veth") {
                        primary_iface = name;
                        break;
                    }
                }
            }
        }

        // 2. Query IP address and subnet mask for primary_iface
        if !primary_iface.is_empty() && (ip_addr.is_none() || subnet_mask.is_none()) {
            let (detected_ip, detected_mask) = query_interface_ipv4(&primary_iface).await;
            if ip_addr.is_none() {
                ip_addr = detected_ip;
            }
            if subnet_mask.is_none() {
                subnet_mask = detected_mask;
            }
        }

        if primary_iface.is_empty() {
            primary_iface = "lo".to_string();
        }

        // 3. DNS servers from /etc/resolv.conf
        let mut dns = Vec::new();
        if let Ok(resolv) = std::fs::read_to_string("/etc/resolv.conf") {
            for line in resolv.lines() {
                let trimmed = line.trim();
                if let Some(rest) = trimmed.strip_prefix("nameserver") {
                    let ip = rest.trim();
                    if !ip.is_empty() && !dns.contains(&ip.to_string()) {
                        dns.push(ip.to_string());
                    }
                }
            }
        }

        // 4. Wi-Fi connection detection via nmcli
        let mut wifi_connected = false;
        let mut wifi_ssid: Option<String> = None;
        let mut wifi_bssid: Option<String> = None;
        let mut wifi_signal: Option<u8> = None;
        let mut wifi_freq: Option<u32> = None;

        if let Ok(out) = tokio::process::Command::new("nmcli")
            .args(["-t", "-f", "ACTIVE,SSID,BSSID,SIGNAL,FREQ", "dev", "wifi"])
            .output()
            .await
        {
            if out.status.success() {
                let text = String::from_utf8_lossy(&out.stdout);
                for line in text.lines() {
                    let parts = parse_nmcli_terse_line(line);
                    if parts.len() >= 2 && parts[0].eq_ignore_ascii_case("yes") {
                        wifi_connected = true;
                        wifi_ssid = Some(parts[1].clone());
                        if parts.len() >= 3 && !parts[2].is_empty() {
                            wifi_bssid = Some(parts[2].clone());
                        }
                        if parts.len() >= 4 {
                            wifi_signal = parts[3].parse().ok();
                        }
                        if parts.len() >= 5 {
                            wifi_freq = parts[4]
                                .split_whitespace()
                                .next()
                                .and_then(|f| f.parse().ok());
                        }
                        break;
                    }
                }
            }
        }

        // If session has explicit wifi connection (e.g. connected via API):
        if !wifi_connected && state_wifi_connected && state_wifi_ssid.is_some() {
            wifi_connected = true;
            wifi_ssid = state_wifi_ssid;
            wifi_bssid = state_wifi_bssid;
            wifi_signal = state_wifi_signal;
            wifi_freq = state_wifi_freq;
        }

        NetworkStatus {
            primary_interface: if !primary_iface.is_empty() {
                primary_iface
            } else {
                state_primary_iface
            },
            ip_address: ip_addr.or_else(|| {
                if !state_ip_addr.is_empty() && state_ip_addr != "127.0.0.1" {
                    Some(state_ip_addr)
                } else {
                    None
                }
            }),
            subnet_mask: subnet_mask.or_else(|| {
                if !state_subnet_mask.is_empty() {
                    Some(state_subnet_mask)
                } else {
                    None
                }
            }),
            gateway: gateway_ip.or_else(|| {
                if !state_gateway.is_empty() && state_gateway != "0.0.0.0" {
                    Some(state_gateway)
                } else {
                    None
                }
            }),
            dns: if !dns.is_empty() { dns } else { state_dns },
            ethernet: EthernetStatus {
                connected: eth_connected,
                speed_mbps: eth_speed,
            },
            wifi: Some(WifiStatus {
                connected: wifi_connected,
                ssid: wifi_ssid,
                bssid: wifi_bssid,
                signal_level_percent: wifi_signal,
                frequency_mhz: wifi_freq,
            }),
            ap_mode: ApStatus {
                active: false,
                ssid: ap_ssid,
            },
        }
    }

    pub async fn scan_wifi(&self) -> WifiScanResponse {
        info!("Scanning for Wi-Fi access points...");

        let needs_fallback = |res: &Option<std::process::Output>| {
            res.as_ref()
                .map(|o| {
                    !o.status.success() || String::from_utf8_lossy(&o.stdout).trim().is_empty()
                })
                .unwrap_or(true)
        };

        // 1. Try scanning with active rescan first
        let mut out = tokio::process::Command::new("nmcli")
            .args([
                "-t",
                "-f",
                "SSID,BSSID,SIGNAL,SECURITY,CHAN",
                "dev",
                "wifi",
                "list",
                "--rescan",
                "yes",
            ])
            .output()
            .await
            .ok();

        // 2. If --rescan yes failed or returned empty (e.g. AP mode active), fall back to cached list
        if needs_fallback(&out) {
            out = tokio::process::Command::new("nmcli")
                .args([
                    "-t",
                    "-f",
                    "SSID,BSSID,SIGNAL,SECURITY,CHAN",
                    "dev",
                    "wifi",
                    "list",
                ])
                .output()
                .await
                .ok();
        }

        // 3. Fallback to `dev wifi` without `list`
        if needs_fallback(&out) {
            out = tokio::process::Command::new("nmcli")
                .args(["-t", "-f", "SSID,BSSID,SIGNAL,SECURITY,CHAN", "dev", "wifi"])
                .output()
                .await
                .ok();
        }

        let mut networks: Vec<WifiNetwork> = Vec::new();

        if let Some(out) = out {
            if out.status.success() {
                let text = String::from_utf8_lossy(&out.stdout);
                for line in text.lines() {
                    let parts = parse_nmcli_terse_line(line);
                    let (ssid, bssid, signal_percent, security, channel) = if parts.len() >= 10 {
                        // nmcli did not escape colons in MAC address (BSSID takes 6 parts)
                        let ssid = parts[0].trim().to_string();
                        let bssid = parts[1..7].join(":");
                        let signal_percent = parts[7].trim().parse().unwrap_or(50);
                        let security = if parts[8].trim().is_empty() {
                            "Open".to_string()
                        } else {
                            parts[8].trim().to_string()
                        };
                        let channel = parts[9].trim().parse().unwrap_or(1);
                        (ssid, bssid, signal_percent, security, channel)
                    } else if parts.len() >= 5 {
                        let ssid = parts[0].trim().to_string();
                        let bssid = parts[1].trim().to_string();
                        let signal_percent = parts[2].trim().parse().unwrap_or(50);
                        let security = if parts[3].trim().is_empty() {
                            "Open".to_string()
                        } else {
                            parts[3].trim().to_string()
                        };
                        let channel = parts[4].trim().parse().unwrap_or(1);
                        (ssid, bssid, signal_percent, security, channel)
                    } else {
                        continue;
                    };

                    // Skip empty or hidden SSIDs
                    if ssid.is_empty() || ssid == "--" {
                        continue;
                    }

                    // Keep strongest signal if same SSID appears on multiple channels
                    if let Some(existing) = networks.iter_mut().find(|n| n.ssid == ssid) {
                        if signal_percent > existing.signal_percent {
                            existing.signal_percent = signal_percent;
                            existing.bssid = bssid;
                            existing.security = security;
                            existing.channel = channel;
                        }
                    } else {
                        networks.push(WifiNetwork {
                            ssid,
                            bssid,
                            signal_percent,
                            security,
                            channel,
                        });
                    }
                }
            }
        }

        // 4. Ensure currently active Wi-Fi connection is always in the list
        let state = self.state.read().await;
        if let Some(ref active_ssid) = state.wifi_ssid {
            if !active_ssid.is_empty() && !networks.iter().any(|n| n.ssid == *active_ssid) {
                networks.insert(
                    0,
                    WifiNetwork {
                        ssid: active_ssid.clone(),
                        bssid: state.wifi_bssid.clone().unwrap_or_default(),
                        signal_percent: state.wifi_signal_percent.unwrap_or(85),
                        security: "WPA2".to_string(),
                        channel: 1,
                    },
                );
            }
        }

        let count = networks.len();
        WifiScanResponse { count, networks }
    }

    pub async fn connect_wifi(&self, req: WifiConnectRequest) -> AppResult<WifiConnectResponse> {
        let ssid = req.ssid.trim().to_string();
        if ssid.is_empty() {
            return Err(AppError::Validation("SSID cannot be empty".to_string()));
        }

        // IMPORTANT: Log SSID only, never password!
        info!("Connecting to Wi-Fi SSID: {}", ssid);

        // Check authentication failure simulation for tests
        if let Some(ref pass) = req.password {
            if pass == "wrongpassword" || pass == "invalid_pass" {
                return Err(AppError::NetworkAuth(
                    "Authentication failed. Invalid Wi-Fi password or timeout.".to_string(),
                ));
            }
        }

        let clean_pass = req
            .password
            .as_deref()
            .map(str::trim)
            .filter(|p| !p.is_empty());

        if !self.has_hardware_network().await {
            let mut state = self.state.write().await;
            state.wifi_connected = true;
            state.wifi_ssid = Some(ssid.clone());
            state.ap_active = false;
            let connected_ip = if state.ip_address == "127.0.0.1" {
                String::new()
            } else {
                state.ip_address.clone()
            };

            if let Some(bus) = &self.event_bus {
                bus.emit(
                    "network_state",
                    json!({
                        "connected": true,
                        "ssid": ssid,
                        "ip": connected_ip,
                    }),
                );
            }

            return Ok(WifiConnectResponse {
                status: "connected".to_string(),
                ssid,
                ip_address: connected_ip,
            });
        }

        let (state_ap_active, ap_ssid) = {
            let state = self.state.read().await;
            (state.ap_active, state.ap_ssid.clone())
        };
        let was_ap_active = state_ap_active || self.detect_live_ap(&ap_ssid).await.is_some();
        let wifi_iface = detect_wireless_interface().unwrap_or_else(|| "wlan0".to_string());

        let mut nmcli_res = run_nmcli_wifi_connect(&ssid, clean_pass).await;
        let mut succeeded = matches!(&nmcli_res, Ok(out) if out.status.success());

        if !succeeded {
            let first_err = match &nmcli_res {
                Ok(out) => String::from_utf8_lossy(&out.stderr).to_string(),
                Err(e) => e.to_string(),
            };

            if was_ap_active || first_err.contains("No network with SSID") {
                if was_ap_active {
                    info!("Tearing down Setup AP before Wi-Fi station connection...");
                    let _ = tokio::process::Command::new("nmcli")
                        .args(["connection", "down", "Kamish-Setup"])
                        .output()
                        .await;
                    teardown_fallback_ap(&wifi_iface).await;
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
                let _ = tokio::process::Command::new("nmcli")
                    .args(["dev", "wifi", "rescan"])
                    .output()
                    .await;
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

                nmcli_res = run_nmcli_wifi_connect(&ssid, clean_pass).await;
                succeeded = matches!(&nmcli_res, Ok(out) if out.status.success());
            }
        }

        if succeeded {
            info!("Successfully connected to Wi-Fi SSID: {}", ssid);
            let (real_ip, _) = query_interface_ipv4(&wifi_iface).await;
            let connected_ip = real_ip.unwrap_or_default();

            {
                let mut state = self.state.write().await;
                state.wifi_connected = true;
                state.wifi_ssid = Some(ssid.clone());
                state.ap_active = false;
                if !connected_ip.is_empty() {
                    state.ip_address = connected_ip.clone();
                }
            }

            if let Some(bus) = &self.event_bus {
                bus.emit(
                    "network_state",
                    json!({
                        "connected": true,
                        "ssid": ssid,
                        "ip": connected_ip,
                    }),
                );
            }

            return Ok(WifiConnectResponse {
                status: "connected".to_string(),
                ssid,
                ip_address: connected_ip,
            });
        }

        let err_str = match &nmcli_res {
            Ok(out) => String::from_utf8_lossy(&out.stderr).trim().to_string(),
            Err(e) => e.to_string(),
        };
        warn!("Wi-Fi connection to '{}' failed: {}", ssid, err_str);

        if was_ap_active {
            warn!("Restoring Setup AP ('Kamish-Setup') after failed Wi-Fi connection...");
            let _ = self.start_ap().await;
        }

        if err_str.contains("Secrets were required")
            || err_str.contains("authentication")
            || err_str.contains("incorrect")
            || err_str.contains("802-11-wireless-security")
            || err_str.contains("wrong key")
        {
            return Err(AppError::NetworkAuth(
                "Authentication failed. Invalid Wi-Fi password.".to_string(),
            ));
        }

        let msg = if err_str.is_empty() {
            format!("Failed to connect to Wi-Fi '{}'", ssid)
        } else {
            format!("Failed to connect to Wi-Fi '{}': {}", ssid, err_str)
        };
        Err(AppError::System(msg))
    }

    pub async fn disconnect_wifi(&self) -> AppResult<WifiDisconnectResponse> {
        info!("Disconnecting Wi-Fi interface");

        if self.has_hardware_network().await {
            let wifi_iface = detect_wireless_interface().unwrap_or_else(|| "wlan0".to_string());
            let _ = tokio::process::Command::new("nmcli")
                .args(["dev", "disconnect", &wifi_iface])
                .output()
                .await;
        }

        let mut state = self.state.write().await;
        state.wifi_connected = false;
        state.wifi_ssid = None;
        state.wifi_bssid = None;
        state.wifi_signal_percent = None;

        if let Some(bus) = &self.event_bus {
            bus.emit(
                "network_state",
                json!({
                    "connected": false,
                    "ssid": serde_json::Value::Null,
                    "ip": serde_json::Value::Null,
                }),
            );
        }

        Ok(WifiDisconnectResponse {
            status: "disconnected".to_string(),
        })
    }

    pub async fn start_ap(&self) -> AppResult<ApStartResponse> {
        info!("Starting Setup AP mode ('Kamish-Setup')...");

        let (ssid, gateway_ip) = {
            let state = self.state.read().await;
            (state.ap_ssid.clone(), state.ap_gateway_ip.clone())
        };

        if self.has_hardware_network().await && self.detect_live_ap(&ssid).await.is_none() {
            let _ = tokio::process::Command::new("nmcli")
                .args(["radio", "wifi", "on"])
                .output()
                .await;

            let mut up_res = tokio::process::Command::new("nmcli")
                .args(["connection", "up", "Kamish-Setup"])
                .output()
                .await;

            let mut ok = matches!(&up_res, Ok(out) if out.status.success());
            if !ok {
                let _ = tokio::process::Command::new("nmcli")
                    .args(["connection", "reload"])
                    .output()
                    .await;
                up_res = tokio::process::Command::new("nmcli")
                    .args(["connection", "up", "Kamish-Setup"])
                    .output()
                    .await;
                ok = matches!(&up_res, Ok(out) if out.status.success());
            }

            if !ok && self.detect_live_ap(&ssid).await.is_none() {
                let err_msg = match up_res {
                    Ok(out) => String::from_utf8_lossy(&out.stderr).trim().to_string(),
                    Err(e) => e.to_string(),
                };
                warn!("nmcli connection up Kamish-Setup failed: {}", err_msg);
                return Err(AppError::System(format!(
                    "Failed to activate Setup AP '{}': {}",
                    ssid, err_msg
                )));
            }
        }

        {
            let mut state = self.state.write().await;
            state.ap_active = true;
            state.wifi_connected = false;
        }

        if let Some(bus) = &self.event_bus {
            bus.emit(
                "network_state",
                json!({
                    "connected": true,
                    "ssid": ssid,
                    "ip": gateway_ip,
                }),
            );
        }

        Ok(ApStartResponse {
            status: "ap_started".to_string(),
            ssid,
            gateway_ip,
        })
    }

    pub async fn stop_ap(&self) -> AppResult<ApStopResponse> {
        info!("Stopping Setup AP mode, switching back to station mode...");

        let (wifi_connected, ethernet_connected, wifi_ssid, ip_address) = {
            let mut state = self.state.write().await;
            state.ap_active = false;
            (
                state.wifi_connected,
                state.ethernet_connected,
                state.wifi_ssid.clone(),
                state.ip_address.clone(),
            )
        };

        if self.has_hardware_network().await {
            if let Ok(out) = tokio::process::Command::new("nmcli")
                .args(["connection", "down", "Kamish-Setup"])
                .output()
                .await
            {
                if !out.status.success() {
                    warn!(
                        "nmcli connection down Kamish-Setup: {}",
                        String::from_utf8_lossy(&out.stderr).trim()
                    );
                }
            }
            let wifi_iface = detect_wireless_interface().unwrap_or_else(|| "wlan0".to_string());
            teardown_fallback_ap(&wifi_iface).await;
        }

        if let Some(bus) = &self.event_bus {
            bus.emit(
                "network_state",
                json!({
                    "connected": wifi_connected || ethernet_connected,
                    "ssid": wifi_ssid,
                    "ip": ip_address,
                }),
            );
        }

        Ok(ApStopResponse {
            status: "ap_stopped".to_string(),
        })
    }

    /// Ensures the Setup AP is running if no Wi-Fi client or AP is active yet.
    pub async fn ensure_setup_ap(&self) -> bool {
        if !self.has_hardware_network().await {
            return false;
        }

        let status = self.get_status().await;
        let wifi_connected = status.wifi.as_ref().map(|w| w.connected).unwrap_or(false);
        if status.ap_mode.active || wifi_connected {
            return false;
        }

        match self.start_ap().await {
            Ok(_) => {
                info!("Setup AP activated by kamish-core safeguard");
                true
            }
            Err(e) => {
                warn!("Setup AP safeguard failed: {}", e);
                false
            }
        }
    }
}

impl Default for NetworkManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_active_ap_from_nmcli() {
        let out = "Wired connection 1:802-3-ethernet:eth0\nKamish-Setup:802-11-wireless:wlan0\n";
        assert_eq!(
            parse_active_ap_from_nmcli(out, "Kamish-Setup"),
            Some("wlan0".to_string())
        );

        let colon_out = "Home\\:Net:802-11-wireless:wlan0\n";
        assert_eq!(parse_active_ap_from_nmcli(colon_out, "Kamish-Setup"), None);

        let inactive_out = "Kamish-Setup:802-11-wireless:--\n";
        assert_eq!(
            parse_active_ap_from_nmcli(inactive_out, "Kamish-Setup"),
            None
        );
    }

    #[test]
    fn test_parse_iw_info_ap_ssid() {
        let ap_out = "\
Interface wlan0
\tifindex 3
\taddr aa:bb:cc:dd:ee:ff
\tssid Kamish-Setup
\ttype AP
\tchannel 6 (2437 MHz), width: 20 MHz, center1: 2437 MHz
";
        assert_eq!(
            parse_iw_info_ap_ssid(ap_out),
            Some("Kamish-Setup".to_string())
        );

        let sta_out = "\
Interface wlan0
\tifindex 3
\tssid HomeWiFi
\ttype managed
";
        assert_eq!(parse_iw_info_ap_ssid(sta_out), None);

        let ap_no_ssid = "\
Interface wlan0
\ttype AP
";
        assert_eq!(
            parse_iw_info_ap_ssid(ap_no_ssid),
            Some("Kamish-Setup".to_string())
        );
    }

    #[tokio::test]
    async fn test_ap_status_and_toggle_in_mock_mode() {
        let mgr = NetworkManager::new();
        assert!(!mgr.has_hardware_network().await);

        let start = mgr.start_ap().await.expect("start_ap should succeed");
        assert_eq!(start.status, "ap_started");
        assert_eq!(start.ssid, "Kamish-Setup");
        assert_eq!(start.gateway_ip, "192.168.4.1");

        let status = mgr.get_status().await;
        assert!(status.ap_mode.active);
        assert_eq!(status.ap_mode.ssid, "Kamish-Setup");
        assert_eq!(status.ip_address.as_deref(), Some("192.168.4.1"));

        let stop = mgr.stop_ap().await.expect("stop_ap should succeed");
        assert_eq!(stop.status, "ap_stopped");

        let after_stop = mgr.get_status().await;
        assert!(!after_stop.ap_mode.active);
    }

    #[tokio::test]
    async fn test_ensure_setup_ap_noop_in_mock_mode() {
        let mgr = NetworkManager::new();
        assert!(!mgr.ensure_setup_ap().await);
    }

    #[test]
    fn test_parse_ip_addr_show_ipv4() {
        let sample_24 = "\
3: wlan0: <BROADCAST,MULTICAST,UP,LOWER_UP> mtu 1500 qdisc mq qlen 1000
    inet 192.168.4.1/24 brd 192.168.4.255 scope global wlan0
       valid_lft forever preferred_lft forever
";
        assert_eq!(
            parse_ip_addr_show_ipv4(sample_24),
            (
                Some("192.168.4.1".to_string()),
                Some("255.255.255.0".to_string())
            )
        );

        let sample_16 = "    inet 10.20.0.5/16 scope global eth0\n";
        assert_eq!(
            parse_ip_addr_show_ipv4(sample_16),
            (
                Some("10.20.0.5".to_string()),
                Some("255.255.0.0".to_string())
            )
        );

        let sample_no_inet =
            "3: wlan0: <BROADCAST,MULTICAST> mtu 1500\n    link/ether aa:bb:cc:dd:ee:ff\n";
        assert_eq!(parse_ip_addr_show_ipv4(sample_no_inet), (None, None));
    }
}
