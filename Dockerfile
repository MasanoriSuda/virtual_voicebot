##############################################
# 1) Build Stage
##############################################
FROM rust:1.76-bullseye AS builder

# 依存キャッシュを効かせる（高速ビルドのコツ）
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main(){}" > src/main.rs
RUN cargo build --release

# 本物のソースを入れて再ビルド
COPY . .
RUN cargo build --release


##############################################
# 2) Runtime Stage
##############################################
FROM debian:bullseye-slim

# 実行に必要最低限だけ入れる（証明書）
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# 実行バイナリをコピー
COPY --from=builder /app/target/release/sip_pcmu_bot /usr/local/bin/sip_pcmu_bot

# デフォルト環境変数
ENV RUST_LOG=info
ENV SIP_BIND_IP=0.0.0.0
ENV SIP_PORT=5060
ENV RTP_PORT=40000
ENV LOCAL_IP=127.0.0.1

# UDP 5060(SIP), 40000(RTP)
EXPOSE 5060/udp
EXPOSE 40000/udp

# 実行コマンド
CMD ["sip_pcmu_bot"]
