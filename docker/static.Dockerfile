# Tier 4 — Layer 1+3 static checks in a hermetic container.
# Runs cargo fmt --check + cargo clippy + tsc --noEmit + eslint
# in an environment where dependency drift can't affect results.
#
# Build:  docker build -f docker/static.Dockerfile -t overlay-static .
# Run:    docker run --rm overlay-static
#
# CI integration: this is the *replacement* for scripts/ci.ps1 layers
# 1+3 when running on Linux / non-Windows. Local devs keep using
# scripts/ci.ps1 (faster + matches their toolchain). CI uses this.

FROM rust:1-bookworm AS rust-base
RUN apt-get update && apt-get install -y \
    build-essential pkg-config libssl-dev libgtk-3-dev \
    libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev \
    && rm -rf /var/lib/apt/lists/*
RUN rustup component add rustfmt clippy

FROM rust-base AS node-base
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y nodejs

WORKDIR /app

# Cache Rust deps first.
COPY src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/
RUN mkdir -p src-tauri/src && echo "fn main() {}" > src-tauri/src/lib.rs \
    && cd src-tauri && cargo fetch && cd .. \
    && rm -rf src-tauri/src

# Cache Node deps.
COPY package.json package-lock.json ./
RUN npm ci --no-audit --no-fund

# Source.
COPY . .

# All four gates. Each `&&` step exits non-zero on any check failure.
RUN cd src-tauri \
    && cargo fmt --all -- --check \
    && cargo clippy --all-targets -- -D warnings \
    && cd .. \
    && npx tsc --noEmit \
    && npx eslint src --max-warnings 999  # TODO Tier 2.5: drop to 0

CMD ["echo", "static checks: PASS"]
