#!/usr/bin/env bash
set -euo pipefail

# Zoiper 動作確認用の起動スクリプト（ログは stdout）。
# 必要なら env で上書き:
#   SIP_BIND_IP=0.0.0.0 SIP_PORT=5060 RTP_PORT=10000 \
#   LOCAL_IP=192.168.1.10 ADVERTISED_IP=192.168.1.10 \
#   RECORDING_HTTP_ADDR=0.0.0.0:18080 RUST_LOG=info ./run_uas.sh

: "${SIP_BIND_IP:=0.0.0.0}"
: "${SIP_PORT:=5060}"
: "${RTP_PORT:=10000}"
: "${LOCAL_IP:=0.0.0.0}"
: "${RECORDING_HTTP_ADDR:=0.0.0.0:18080}"
: "${RUST_LOG:=info}"

if [ -z "${ADVERTISED_IP:-}" ]; then
  ADVERTISED_IP="$LOCAL_IP"
fi

export SIP_BIND_IP SIP_PORT RTP_PORT LOCAL_IP ADVERTISED_IP RECORDING_HTTP_ADDR RUST_LOG

echo "[run_uas] SIP UDP/TCP  ${SIP_BIND_IP}:${SIP_PORT} (advertised ${ADVERTISED_IP})"
echo "[run_uas] RTP          ${RTP_PORT}"
echo "[run_uas] Recording    ${RECORDING_HTTP_ADDR}"
echo "[run_uas] RUST_LOG     ${RUST_LOG}"

cargo run --bin virtual-voicebot-backend
