#!/usr/bin/env bash
# Buildroot-сборка через Docker с retry (сеть рвёт TLS к registry/auth.docker.io)
set -u
cd /mnt/m/Project/Kamish-os/radiant-bardeen/firmware

BUILD_OK=0
for i in 1 2 3 4 5 6; do
    echo "=== docker compose build: attempt $i"
    if docker compose build builder; then
        BUILD_OK=1
        break
    fi
    echo "=== build failed, retry in 10s"
    sleep 10
done

if [ "$BUILD_OK" != "1" ]; then
    echo "BUILD FAILED after retries"
    exit 1
fi

echo "=== docker compose run (Buildroot make)"
for i in 1 2 3 4 5 6 7 8; do
    # Гонка недопустима: убиваем любые брошенные контейнеры прошлых попыток
    # (pkill compose-клиента не убивает контейнер — make продолжает жить и
    # портит общий output/)
    docker ps -q --filter name=firmware-builder | xargs -r docker kill 2>/dev/null
    sleep 1
    echo "=== make: attempt $i"
    if docker compose run --rm builder; then
        echo "MAKE OK"
        exit 0
    fi
    echo "=== make failed (вероятно сеть на скачивании), retry in 10s"
    sleep 10
done

echo "MAKE FAILED after retries"
exit 1
