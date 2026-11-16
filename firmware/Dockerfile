FROM debian:bookworm-slim

ENV DEBIAN_FRONTEND=noninteractive
ENV LANG=C.UTF-8

# Install Buildroot host compilation dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    bash \
    bc \
    binutils \
    build-essential \
    bzip2 \
    ca-certificates \
    cpio \
    curl \
    file \
    git \
    graphviz \
    gzip \
    libncurses-dev \
    libssl-dev \
    locales \
    make \
    mercurial \
    patch \
    perl \
    python3 \
    python3-dev \
    python3-pip \
    rsync \
    sed \
    subversion \
    tar \
    unzip \
    wget \
    which \
    flex \
    bison \
    zstd \
    rauc \
    openssl \
    dosfstools \
    mtools \
    libelf-dev \
    device-tree-compiler \
    && rm -rf /var/lib/apt/lists/*

# Add unprivileged build user (Buildroot refuses to build as root)
RUN useradd -m -s /bin/bash builder && \
    mkdir -p /build && chown -R builder:builder /build

USER builder
WORKDIR /build

# Buildroot version
ENV BUILDROOT_VERSION=2024.02.1

# Тарбол копируется из контекста (wget в контейнере ненадёжен при обрывах сети;
# тарбол скачивается заранее скриптом scripts/fetch-buildroot.sh).
# Дерево хранится как /build/buildroot-orig: docker-compose монтирует
# кэш-каталог хоста на /build/buildroot, а pristine-дерево копируется туда
# при первом запуске. Это нужно, чтобы дерево Buildroot и output/ были на
# одной файловой системе — иначе syncconfig падает на rename() (EXDEV).
COPY buildroot-${BUILDROOT_VERSION}.tar.gz /build/
RUN tar -xzf /build/buildroot-${BUILDROOT_VERSION}.tar.gz && \
    mv /build/buildroot-${BUILDROOT_VERSION} /build/buildroot-orig && \
    rm /build/buildroot-${BUILDROOT_VERSION}.tar.gz && \
    mkdir -p /build/buildroot

WORKDIR /build/buildroot

CMD ["/bin/bash"]
