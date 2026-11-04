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

RUN wget -q https://buildroot.org/downloads/buildroot-${BUILDROOT_VERSION}.tar.gz && \
    tar -xzf buildroot-${BUILDROOT_VERSION}.tar.gz && \
    mv buildroot-${BUILDROOT_VERSION} buildroot && \
    rm buildroot-${BUILDROOT_VERSION}.tar.gz

WORKDIR /build/buildroot

CMD ["/bin/bash"]
