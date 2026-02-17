# === ビルドステージ ===
FROM ubuntu:22.04 AS build

ENV DEBIAN_FRONTEND=noninteractive \
    TZ=UTC \
    RUST_BACKTRACE=1 \
    PATH="/root/.cargo/bin:${PATH}"

RUN apt-get update && apt-get install -y \
    curl \
    ca-certificates \
    git \
    build-essential \
    pkg-config \
    libssl-dev \
    python3 \
    python3-pip \
    python3-venv \
    sip-tester \
    && rm -rf /var/lib/apt/lists/*

# Whisper 用 Python ライブラリ（開発用）
RUN pip3 install --no-cache-dir \
    faster-whisper \
    && rm -rf /root/.cache/pip

# Rust インストール
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --default-toolchain stable

# cargo watch（開発時ホットリロード）
RUN cargo install cargo-watch

WORKDIR /workspace

COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src/bin \
    && echo "fn main() { println!(\"dummy\") }" > src/main.rs \
    && echo "fn main() { println!(\"dummy\") }" > src/bin/serversync.rs \
    && cargo build --release \
    && rm -rf src

COPY . .

RUN cargo build --release

# === 実行ステージ ===
FROM ubuntu:22.04 AS runtime

ENV DEBIAN_FRONTEND=noninteractive \
    TZ=UTC \
    RUST_BACKTRACE=1 \
    LOCAL_IP=127.0.0.1

RUN apt-get update && \
    apt-get install -y software-properties-common && \
    add-apt-repository universe && \
    apt-get update && \
    apt-get install -y \
        ca-certificates \
        git \
        sipp \
        python3 \
        python3-pip \
        python3-venv \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /workspace

COPY --from=build /workspace/target/release/virtual-voicebot-backend /workspace/virtual-voicebot-backend

EXPOSE 5060/udp
EXPOSE 10000-10100/udp
EXPOSE 18080

CMD ["/workspace/virtual-voicebot-backend"]
