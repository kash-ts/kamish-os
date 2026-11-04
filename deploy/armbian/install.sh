#!/usr/bin/env bash
# ==============================================================================
# Kamish OS Unattended Automated Installation Script for Armbian Minimal
# Target Hardware: Orange Pi 4 Pro (Allwinner A733, 4GB LPDDR4, 3 TOPS NPU, ARM64)
# Peripherals: Seeed ReSpeaker XVF3800, GT-112 Audio Output
# ==============================================================================

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC} $*"; }
log_ok() { echo -e "${GREEN}[OK]${NC} $*"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*"; }

# 1. Root & Architecture Check
if [[ $EUID -ne 0 ]]; then
   log_error "This script must be run as root. Run with: sudo ./install.sh"
   exit 1
fi

ARCH=$(uname -m)
if [[ "${ARCH}" != "aarch64" ]]; then
    log_warn "Current architecture is '${ARCH}', target is 'aarch64' (ARM64 Orange Pi 4 Pro)."
    log_warn "Proceeding in development/cross-platform mode."
fi

export DEBIAN_FRONTEND=noninteractive

log_info "Step 1/7: Updating APT package lists and installing core dependencies..."
apt-get update -y
apt-get install -y --no-install-recommends \
    curl \
    wget \
    jq \
    sqlite3 \
    alsa-utils \
    libasound2 \
    mpv \
    python3 \
    python3-pip \
    python3-venv \
    bluez \
    bluez-tools \
    network-manager \
    avahi-daemon \
    avahi-utils \
    libnss-mdns \
    iptables \
    ca-certificates \
    udev

# Install yt-dlp if not installed
if ! command -v yt-dlp &> /dev/null; then
    log_info "Installing yt-dlp..."
    wget -q https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp -O /usr/local/bin/yt-dlp
    chmod a+rx /usr/local/bin/yt-dlp
fi

log_info "Step 2/7: Creating dedicated 'kamish' system user..."
if ! id -u kamish &>/dev/null; then
    useradd -r -s /usr/sbin/nologin -d /var/lib/kamish kamish
fi
usermod -aG audio,dialout,netdev,bluetooth kamish

log_info "Step 3/7: Preparing directories..."
mkdir -p /opt/kamish/bin
mkdir -p /opt/kamish/frontend
mkdir -p /var/lib/kamish
mkdir -p /var/log/kamish
mkdir -p /etc/kamish

chown -R kamish:kamish /var/lib/kamish /var/log/kamish /etc/kamish
chmod 750 /var/lib/kamish /var/log/kamish /etc/kamish

log_info "Step 4/7: Installing udev rules for Seeed ReSpeaker XVF3800..."
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

if [[ -f "${PROJECT_ROOT}/deploy/systemd/99-respeaker.rules" ]]; then
    cp "${PROJECT_ROOT}/deploy/systemd/99-respeaker.rules" /etc/udev/rules.d/99-respeaker.rules
    udevadm control --reload-rules && udevadm trigger
    log_ok "udev rules installed and reloaded."
fi

log_info "Step 5/7: Configuring default environment variables..."
ENV_FILE="/etc/kamish/kamish.env"
if [[ ! -f "${ENV_FILE}" ]]; then
    cat << 'EOF' > "${ENV_FILE}"
HOST=0.0.0.0
PORT=8080
RUST_LOG=info,kamish_core=debug
DATABASE_URL=sqlite:///var/lib/kamish/kamish_core.db
STATIC_DIR=/opt/kamish/frontend
AI_CLI_PATH=agy
AI_CLI_ENABLED=true
SAFE_VOLUME_CEILING=85
EOF
    chown root:kamish "${ENV_FILE}"
    chmod 640 "${ENV_FILE}"
    log_ok "Created default ${ENV_FILE}"
fi

log_info "Step 6/7: Deploying binaries and frontend assets..."
# Deploy backend binary if built
if [[ -f "${PROJECT_ROOT}/backend/target/release/kamish" ]]; then
    cp "${PROJECT_ROOT}/backend/target/release/kamish" /opt/kamish/bin/
    chmod 755 /opt/kamish/bin/kamish
    log_ok "Copied release backend binary."
elif [[ -f "${PROJECT_ROOT}/backend/target/debug/kamish" ]]; then
    cp "${PROJECT_ROOT}/backend/target/debug/kamish" /opt/kamish/bin/
    chmod 755 /opt/kamish/bin/kamish
    log_warn "Copied debug backend binary (build with --release for production)."
fi

# Deploy frontend assets if built
if [[ -d "${PROJECT_ROOT}/frontend/dist" ]]; then
    cp -r "${PROJECT_ROOT}/frontend/dist/"* /opt/kamish/frontend/
    chown -R kamish:kamish /opt/kamish/frontend
    log_ok "Copied frontend assets."
fi

# Install Antigravity CLI if not present
if ! command -v agy &>/dev/null; then
    if [[ -f "${PROJECT_ROOT}/deploy/scripts/install_antigravity.sh" ]]; then
        log_info "Attempting to install Antigravity CLI..."
        bash "${PROJECT_ROOT}/deploy/scripts/install_antigravity.sh" || log_warn "Auto-download of Antigravity CLI failed, continuing with fallback..."
    fi
fi

# Deploy fallback AI CLI emulator if 'agy' is still not installed
if ! command -v agy &>/dev/null && [[ -f "${PROJECT_ROOT}/scripts/agy-mock" ]]; then
    cp "${PROJECT_ROOT}/scripts/agy-mock" /usr/local/bin/agy
    chmod 755 /usr/local/bin/agy
    log_ok "Installed fallback Antigravity CLI emulator to /usr/local/bin/agy."
fi

log_info "Step 7/7: Installing and enabling systemd unit..."
if [[ -f "${PROJECT_ROOT}/deploy/systemd/kamish.service" ]]; then
    cp "${PROJECT_ROOT}/deploy/systemd/kamish.service" /etc/systemd/system/kamish.service
    systemctl daemon-reload
    systemctl enable kamish.service
    log_ok "kamish.service installed and enabled on boot."
fi

# Step 8: Configure Kamish hostname, Avahi mDNS (http://kamish.local) and port 80 redirect
log_info "Configuring mDNS and hostname 'kamish' for http://kamish.local..."
hostnamectl set-hostname kamish || true
if ! grep -q "kamish.local" /etc/hosts; then
    echo "127.0.0.1 kamish.local kamish" >> /etc/hosts
fi

# Setup Avahi HTTP service definition for mDNS discovery
mkdir -p /etc/avahi/services
cat << 'EOF' > /etc/avahi/services/kamish.service
<?xml version="1.0" standalone='no'?>
<!DOCTYPE service-group SYSTEM "avahi-service.dtd">
<service-group>
  <name replace-wildcards="yes">Kamish Core System</name>
  <service>
    <type>_http._tcp</type>
    <port>80</port>
    <txt-record>path=/</txt-record>
  </service>
  <service>
    <type>_http._tcp</type>
    <port>8080</port>
    <txt-record>path=/</txt-record>
  </service>
</service-group>
EOF

systemctl enable avahi-daemon || true
systemctl restart avahi-daemon || true

# Port 80 -> 8080 redirect so http://kamish.local works directly without typing :8080
if command -v iptables &> /dev/null; then
    iptables -t nat -C PREROUTING -p tcp --dport 80 -j REDIRECT --to-port 8080 2>/dev/null || \
    iptables -t nat -A PREROUTING -p tcp --dport 80 -j REDIRECT --to-port 8080 || true
fi

echo -e "\n${GREEN}================================================================${NC}"
echo -e "${GREEN} Kamish OS Installation Completed!${NC}"
echo -e "${GREEN}================================================================${NC}"
echo -e "To start the service:     systemctl start kamish.service"
echo -e "To check system logs:     journalctl -u kamish.service -f"
echo -e "To run hardware checks:   ${PROJECT_ROOT}/deploy/scripts/hardware_check.sh"
echo -e "Web Interface:            http://kamish.local  (или http://<IP>:8080)"
echo -e "${GREEN}================================================================${NC}\n"
