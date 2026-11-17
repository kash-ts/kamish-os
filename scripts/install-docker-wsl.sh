#!/usr/bin/env bash
# Установка Docker Engine внутрь WSL2 (Ubuntu) — запуск от root (wsl -u root)
set -eu

echo "=== apt install docker.io + docker-compose-v2 ==="
apt-get update -qq
DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
    docker.io docker-compose-v2

echo "=== enable & start dockerd ==="
if systemctl is-system-running >/dev/null 2>&1; then
    systemctl enable --now docker || true
else
    # без systemd — запуск сервиса вручную
    service docker start || true
fi
sleep 3
docker version --format 'docker engine: {{.Server.Version}}' && echo "DOCKER OK"

echo "=== add user ipoma to docker group ==="
usermod -aG docker ipoma && echo "added"
