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

# Rust インストール
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --default-toolchain stable

WORKDIR /workspace

COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() { println!(\"dummy\") }" > src/main.rs \
    && cargo build --release \
    && rm -rf src

COPY . .

RUN cargo build --release

# === 実行ステージ ===
FROM ubuntu:22.04 AS runtime

ENV DEBIAN_FRONTEND=noninteractive \
    TZ=UTC \
    RUST_BACKTRACE=1 \
    PUBLIC_IP=127.0.0.1

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

COPY --from=build /workspace/target/release/virtual_voicebot /workspace/virtual_voicebot

EXPOSE 5060/udp
EXPOSE 10000-20000/udp

CMD ["/workspace/virtual_voicebot"]
