#!/usr/bin/env bash
set -euo pipefail

# SIP/SDP広告
export LOCAL_IP=0.0.0.0
export ADVERTISED_IP=192.168.10.110   # 公開IPに差し替え
export ADVERTISED_RTP_PORT=40000 # NATで開けているRTPポートに合わせる
# 実際にバインドするRTPポートも広告と合わせる（Zoiper側が送る先と一致させる）
export RTP_PORT=${RTP_PORT:-40000}

# LLM
export GEMINI_API_KEY="...your key..."
export GEMINI_MODEL="gemini-2.5-flash-lite"

# ASR: ローカルWhisperを使う場合はOFF
export USE_AWS_TRANSCRIBE=1
# USE_AWS_TRANSCRIBE=1 の場合は下記も設定
# export AWS_TRANSCRIBE_BUCKET="your-bucket"
# export AWS_TRANSCRIBE_PREFIX="voicebot"
# AWSの認証情報/リージョンは usual AWS env か IAM ロールで

RUST_LOG=info cargo run
