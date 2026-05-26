# Tier 4 — Layer 2 unit tests in a hermetic container.
# Runs cargo test --lib + vitest run.
#
# Build:  docker build -f docker/unit.Dockerfile -t overlay-unit .
# Run:    docker run --rm overlay-unit

FROM rust:1-bookworm AS rust-base
RUN apt-get update && apt-get install -y build-essential libssl-dev \
    && rm -rf /var/lib/apt/lists/*

FROM rust-base AS node-base
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y nodejs

WORKDIR /app

COPY src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/
RUN mkdir -p src-tauri/src && echo "fn main() {}" > src-tauri/src/lib.rs \
    && cd src-tauri && cargo fetch && cd .. \
    && rm -rf src-tauri/src

COPY package.json package-lock.json ./
RUN npm ci --no-audit --no-fund

COPY . .

# Rust: 260 unit + integration tests + copy contract.
RUN cd src-tauri \
    && cargo test --lib --quiet \
    && cargo test --test copy_contract --quiet

# TS: vitest runs all *.test.{ts,tsx} files.
RUN npm test

CMD ["echo", "unit tests: PASS"]
