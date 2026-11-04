#!/usr/bin/env bash
# ==============================================================================
# Lavalink v4 Runner for Kamish OS (YouTube, Yandex Music, SoundCloud)
# ==============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "${SCRIPT_DIR}"

LAVALINK_VERSION="4.0.8"
LAVALINK_JAR="Lavalink.jar"
DOWNLOAD_URL="https://github.com/lavalink-devs/Lavalink/releases/download/${LAVALINK_VERSION}/Lavalink.jar"

# 1. Check for Java 17+
if ! command -v java >/dev/null 2>&1; then
    echo "[ERROR] Java is not installed. Please install OpenJDK 17+ (e.g. apt install openjdk-17-jre-headless)."
    exit 1
fi

JAVA_VER=$(java -version 2>&1 | head -n 1 | awk -F '"' '{print $2}' | cut -d'.' -f1)
if [[ "${JAVA_VER}" -lt 17 ]]; then
    echo "[ERROR] Java version must be >= 17. Detected: ${JAVA_VER}"
    exit 1
fi

# 2. Download Lavalink.jar if not present
if [[ ! -f "${LAVALINK_JAR}" ]]; then
    echo "[INFO] Downloading Lavalink v${LAVALINK_VERSION}..."
    curl -fSL --retry 3 "${DOWNLOAD_URL}" -o "${LAVALINK_JAR}"
    echo "[OK] Downloaded ${LAVALINK_JAR}"
fi

# 3. Ensure plugins and logs directories exist
mkdir -p plugins logs

echo "[INFO] Starting Lavalink v4 audio server..."
echo "       Sources: YouTube (plugin), Yandex Music (LavaSrc), SoundCloud (native)"
echo "       Config: ${SCRIPT_DIR}/application.yml"

# JVM parameters optimized for ARM64 and low-footprint embedded environments
exec java \
    -Xmx256m \
    -Xms64m \
    -XX:+UseSerialGC \
    -XX:+ShrinkHeapInSteps \
    -Dstdout.encoding=UTF-8 \
    -Dstderr.encoding=UTF-8 \
    -jar "${LAVALINK_JAR}" \
    --spring.config.location="file:${SCRIPT_DIR}/application.yml"
