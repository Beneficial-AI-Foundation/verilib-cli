
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
ARG CACHEBUST=1
RUN git clone https://github.com/Beneficial-AI-Foundation/probe-verus.git

WORKDIR /build/probe-verus
RUN cargo install --path .

# Download scripts
WORKDIR /build
RUN git clone https://github.com/Beneficial-AI-Foundation/dalek-lite.git /build/dalek-lite && \
    cd /build/dalek-lite && \
    git checkout c36b395aa5526af7940d1db0f66ea60db4e3a157

FROM --platform=linux/amd64 rust:slim-bookworm AS runtime

RUN apt-get update && apt-get install -y \
    libssl3 \
    z3 \
    python3 \
    python3-pip \
    python3-requests \
    && rm -rf /var/lib/apt/lists/*

# Install uv python package manager
RUN pip3 install uv --break-system-packages

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

# Copy scripts from builder
COPY --from=builder /build/dalek-lite/scripts /usr/local/bin/scripts
ENV PATH="/usr/local/bin/scripts:${PATH}"

# Add local analysis script override
COPY scripts/analyze_verus_specs_proofs.py /usr/local/bin/scripts/analyze_verus_specs_proofs.py

# Patch script to allow REPO_ROOT env var and generic crate name
RUN sed -i 's|repo_root = script_dir.parent|import os; repo_root = Path(os.environ.get("REPO_ROOT", script_dir.parent))|' /usr/local/bin/scripts/analyze_verus_specs_proofs.py && \
    sed -i 's|CRATE_NAME = "curve25519_dalek"|import os; CRATE_NAME = os.environ.get("CRATE_NAME", "curve25519_dalek")|' /usr/local/bin/scripts/analyze_verus_specs_proofs.py && \
    sed -i 's|CRATE_DIR = "curve25519-dalek"|CRATE_DIR = os.environ.get("CRATE_DIR", "curve25519-dalek")|' /usr/local/bin/scripts/analyze_verus_specs_proofs.py && \
    sed -i 's|skip_parts = {"curve25519-dalek", "src"}|skip_parts = {CRATE_DIR, "src"}|' /usr/local/bin/scripts/analyze_verus_specs_proofs.py && \
    sed -i 's|module_stripped = module.replace(f"{CRATE_NAME}::", "")|module_stripped = module.replace(f"{CRATE_NAME}::", "")|' /usr/local/bin/scripts/analyze_verus_specs_proofs.py

# Ensure permissions for copied tools
RUN chmod -R a+rx /usr/local/verus /usr/local/verus-analyzer /usr/local/scip /usr/local/bin/scripts

# Setup workspace
WORKDIR /workspace
USER tooluser

# Entrypoint
ENTRYPOINT ["probe-verus"]
