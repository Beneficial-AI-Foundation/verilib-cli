
FROM --platform=linux/amd64 rust:slim-bookworm AS builder

RUN apt-get update && apt-get install -y \
    curl \
    git \
    python3 \
    python3-pip \
    python3-requests \
    build-essential \
    cmake \
    pkg-config \
    libssl-dev \
    z3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

RUN git clone https://github.com/Beneficial-AI-Foundation/installers_for_various_tools.git installers


WORKDIR /build/installers
RUN echo "n" | python3 verus_installer_from_release.py

RUN echo "n" | python3 verus_analyzer_installer.py
RUN echo "n" | python3 scip_installer.py

WORKDIR /build
RUN git clone https://github.com/Beneficial-AI-Foundation/probe-verus.git

WORKDIR /build/probe-verus
RUN cargo install --path .

FROM --platform=linux/amd64 rust:slim-bookworm AS runtime

RUN apt-get update && apt-get install -y \
    libssl3 \
    z3 \
    python3 \
    python3-pip \
    python3-requests \
    && rm -rf /var/lib/apt/lists/*

# Install specific Rust toolchain for Verus
RUN rustup toolchain install 1.93.0

# Ensure permissions for toolchain and tools
RUN chmod -R a+rx /usr/local/rustup /usr/local/cargo

RUN useradd -m -u 1000 tooluser


COPY --from=builder /usr/local/cargo/bin/probe-verus /usr/local/bin/probe-verus

# Copy verus
COPY --from=builder /root/verus /usr/local/verus
ENV PATH="/usr/local/verus:${PATH}"

# Copy verus-analyzer
COPY --from=builder /root/verus-analyzer /usr/local/verus-analyzer
ENV PATH="/usr/local/verus-analyzer:${PATH}"

# Copy scip
COPY --from=builder /root/scip /usr/local/scip
ENV PATH="/usr/local/scip:${PATH}"

# Ensure permissions for copied tools
RUN chmod -R a+rx /usr/local/verus /usr/local/verus-analyzer /usr/local/scip

# Setup workspace
WORKDIR /workspace
USER tooluser

# Entrypoint
ENTRYPOINT ["probe-verus"]
