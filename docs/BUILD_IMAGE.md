# Сборка и запись установочного образа KAMISH OS

Инструкция для Windows-хоста. Сборка идёт в WSL2 (Ubuntu), через Docker внутри WSL.

---

## 1. Что уже готово

Всё перечисленное уже сделано и лежит в репозитории (`M:\Project\Kamish-os\radiant-bardeen`):

- BSP-источники ядра и U-Boot прописаны в `firmware/buildroot-external-kamish/configs/kamish_orangepi_4_pro_defconfig`;
- BSP-инструменты упаковки загрузчика перенесены в `firmware/buildroot-external-kamish/board/kamish/orangepi-4-pro/pack/`;
- `post-image.sh` собирает boot0 + boot_package и сшивает их в merged-образ;
- Docker установлен в WSL2 (engine 29.1.3, работает);
- Rust 1.100.0 + target aarch64 установлен в WSL.

Единственное, что не успело дойти до конца: dev-хедеры libc для arm64
(`libc6-dev-arm64-cross`) и сами бинарники `kamish-core`/`kamish-web` под aarch64.
Без них образ соберётся, но будет **без** Rust-сервисов Kamish (шаг 4 делает
это правильно; если пропустить — образ будет с чистым systemd без Kamish).

---

## 2. Установить недостающее (один раз)

В PowerShell:

```powershell
wsl -u root -e bash -c "DEBIAN_FRONTEND=noninteractive apt-get install -y libc6-dev-arm64-cross"
```

Проверка: в WSL (`wsl`) команда
`ls /usr/aarch64-linux-gnu/include/bits/libc-header-start.h` должна показать файл.

---

## 3. Кросс-собрать бинарники Kamish под aarch64

```powershell
wsl -e bash /mnt/m/Project/Kamish-os/radiant-bardeen/scripts/cross-build-kamish.sh
```

Скрипт:
1. rsync'ает `crates/`, `database/`, `Cargo.toml`, `Cargo.lock` в `~/kamish-src` (быстрая ФС WSL);
2. `cargo build --release --workspace` для `aarch64-unknown-linux-gnu`;
3. копирует `kamish-core` и `kamish-web` в репозиторий `target/aarch64-unknown-linux-gnu/release/`.

Критерий успеха: в конце вывод `CROSS BUILD OK` и строки `file` вида
`ELF 64-bit LSB executable, ARM aarch64`.
Время: ~10–20 минут (первый раз; зависимости уже скачаны).

Повторный запуск безопасен: rsync синхронизирует изменения исходников,
cargo пересобирает только изменённое.

> Если сеть обрывает `Updating crates.io index` — просто запустите скрипт
> повторно (сеть до crates.io работает, обрывы разовые).

---

## 4. Собрать образ через Buildroot (Docker в WSL)

```powershell
cd M:\Project\Kamish-os\radiant-bardeen\firmware
wsl -e bash -lc "cd /mnt/m/Project/Kamish-os/radiant-bardeen/firmware && docker compose build builder && docker compose run --rm builder"
```

Либо одной командой из репозитория:

```powershell
wsl -e bash -lc "cd /mnt/m/Project/Kamish-os/radiant-bardeen/firmware && bash scripts/build-firmware.sh"
```

Что происходит:
- контейнер Debian bookworm собирает Buildroot 2024.02.1 с BSP-ядром
  `orange-pi-6.6-sun60iw2` (6.6.98) и BSP U-Boot `v2018.05-sun60iw2`;
- `post-image.sh` пакует загрузчик (boot0_sdcard_a733.fex + boot_package.fex
  через dragonsecboot → `u-boot-orangepi4pro-merged.bin`);
- `genimage.sh` собирает итоговый `kamish-sdcard.img`;
- `kamish-core`/`kamish-web` и SPA попадают в rootfs через пакет `kamish`.

Критерий успеха: файл

```
firmware/output/images/kamish-sdcard.img
```

(на самом деле output лежит на ФС WSL: `/home/ipoma/kamish-buildroot/output/images/kamish-sdcard.img`,
но в контейнере это тот же каталог, смонтированный из compose).

Скопировать в репозиторий для удобства:

```powershell
wsl -e bash -c "cp /home/ipoma/kamish-buildroot/output/images/kamish-sdcard.img /mnt/m/Project/Kamish-os/radiant-bardeen/firmware/output/ && ls -lh /mnt/m/Project/Kamish-os/radiant-bardeen/firmware/output/"
```

Время: 1–3 часа (первая сборка качает ~2 ГБ исходников Buildroot и собирает
тулчейн aarch64, ядро, systemd, mpv и т.д.). Повторные сборки — минуты.

> Памятка по обрывам сети: если Buildroot упал на скачивании — просто
> запустите `docker compose run --rm builder` ещё раз; он продолжит с места
> остановки (кэш в `~/kamish-buildroot/dl`).

---

## 5. Записать образ на носитель

**На Windows (card reader вставлен в ПК):**

Найти номер диска (ВАЖНО: по размеру карты, а не по букве):
```powershell
Get-Disk
```

Записать (замените N на номер диска и путь на фактический):
```powershell
# альтернативно — Rufus/balenaEtcher GUI, им образ тоже подходит (raw .img)
wsl -e bash -c "dd if=/mnt/m/Project/Kamish-os/radiant-bardeen/firmware/output/kamish-sdcard.img of=/dev/sdX bs=4M status=progress conv=fsync"
```

В WSL ваш card reader виден как `/dev/sdX` (X = буква, смотрите `lsblk` в WSL —
по размеру карты!). Если WSL не видит карту — пишите dd прямо из Linux-машины
или через Rufus/Etcher в Windows.

**Либо прямо из Linux/WSL, если карта там видна:**
```bash
lsblk                      # найти /dev/sdX по размеру (по размеру!)
sudo dd if=kamish-sdcard.img of=/dev/sdX bs=4M status=progress conv=fsync
sudo eject /dev/sdX
```

⚠️ `of=/dev/sdX` — устройство ЦЕЛИКОМ, не раздел. Всё, что было на карте, будет стёрто.

---

## 6. Вставить в Orange Pi 4 Pro и включить

Плата (Orange Pi 4 Pro, Allwinner A733):
1. Вставьте microSD-карту в слот платы.
2. Подключите UART-консоль или HDMI-монитор+клавиатуру (опционально, для отладки).
3. Включите питание.

Что должно произойти:
- BROM → boot0 (offset 8K) → boot_package → U-Boot (BSP, v2018.05-sun60iw2);
- U-Boot выполняет `boot.scr` (A/B RAUC-логика, слот A = `mmcblk0p2`);
- ядро 6.6.98 (BSP `sun60i-a733-orangepi-4-pro`), rootfs `system_a` (ext4, p2);
- systemd поднимает `kamish-core`, `kamish-web`, `kamish-mounts`, `kamish-healthcheck`.

Проверка после загрузки (через UART-консоль ttyS0, 115200, или dropbear по SSH):
```bash
uname -a                 # должно быть Linux kamish 6.6.98
systemctl status kamish-core kamish-web
ls /opt/kamish/bin/      # kamish-core kamish-web
ls /opt/kamish/dist/     # SPA (index.html, assets)
```
Веб-панель — HTTP на порту 8080 (по `kamish-web.service`).

---

## 7. Автоматическая сборка образа через GitHub Actions (CI)

Сборка полного установочного `.img` образа перенесена в GitHub Actions:
Workflow: [`.github/workflows/build-image.yml`](../.github/workflows/build-image.yml).

### Запуск:
- **Вручную**: вкладка **Actions → Build Firmware SD Card Image → Run workflow** (можно выбрать сжатие: `zstd` (по умолчанию), `xz` или без сжатия).
- **По тегу релиза**: при публикации тега `v*` (например, `v0.1.0`) workflow автоматически прикрепляет сжатый образ, SHA256 контрольные суммы и базовый `rootfs.tar.gz` к GitHub Releases.

### Оптимизации:
- Используется готовый тулчейн **Bootlin AArch64 glibc** (`BR2_TOOLCHAIN_EXTERNAL_BOOTLIN`), что исключает долгую компиляцию GCC с нуля и экономит 40–50 минут.
- Кэшируются каталоги загрузок (`dl/`) и кэш компилятора (`ccache`).
- Очищается диск раннера (~30 ГБ свободного места).
- Готовый 4 ГБ образ автоматически сжимается с помощью Zstandard (`kamish-sdcard.img.zst`) до ~300 МБ для быстрой загрузки.

---

## Краткая шпаргалка (все команды по порядку)

```powershell
# 1. (один раз) хедеры libc arm64
wsl -u root -e bash -c "apt-get install -y libc6-dev-arm64-cross"

# 2. Кросс-сборка Kamish (~15 мин)
wsl -e bash /mnt/m/Project/Kamish-os/radiant-bardeen/scripts/cross-build-kamish.sh

# 3. Сборка образа (1–3 ч, первый раз)
wsl -e bash -lc "cd /mnt/m/Project/Kamish-os/radiant-bardeen/firmware && docker compose build builder && docker compose run --rm builder"

# 4. Копировать img в репозиторий
wsl -e bash -c "cp /home/ipoma/kamish-buildroot/output/images/kamish-sdcard.img /mnt/m/Project/Kamish-os/radiant-bardeen/firmware/output/"

# 5. Записать на карту (X — буква из lsblk, по размеру карты!)
wsl -e bash -c "lsblk"
wsl -e bash -c "dd if=/mnt/m/Project/Kamish-os/radiant-bardeen/firmware/output/kamish-sdcard.img of=/dev/sdX bs=4M status=progress conv=fsync"
```

---

## Известные ограничения (честно)

1. **Загрузку образа я не проверял** — у меня нет платы. Сборка и упаковка
   загрузчика воспроизводят официальную цепочку OrangePi (orangepi-build),
   но первый реальный запуск подтвердит только железо.
2. `saveenv` U-Boot пишет в env по offset 0x800000 — офсет выбран в зазоре
   между boot0 и boot_package; проверьте, что на вашей SD-карте это не
   конфликтует (не должно — разметка начинается с 32M).
3. Data-раздел (p4) не монтируется автоматически — `kamish-mounts.service`
   создаёт каталоги прямо в rootfs. Раздел 2 ГБ остаётся свободным.
4. Если Buildroot соберётся, но `kamish-core`/`kamish-web` не попадут в
   образ (не был запущен шаг 3) — образ загрузится, но веб-панели и
   сервисов Kamish не будет. Это видно по `ls /opt/kamish/bin/`.
