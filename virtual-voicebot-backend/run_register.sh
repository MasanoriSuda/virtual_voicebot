#!/usr/bin/env bash
set -euo pipefail

# REGISTER test runner. Optionally load envs from REGISTER_ENV_FILE or .env.register.

if [ -n "${REGISTER_ENV_FILE:-}" ]; then
  if [ -f "$REGISTER_ENV_FILE" ]; then
    set -a
    # shellcheck disable=SC1090
    source "$REGISTER_ENV_FILE"
    set +a
  else
    echo "[run_register] REGISTER_ENV_FILE not found: $REGISTER_ENV_FILE" >&2
    exit 2
  fi
elif [ -f ".env.register" ]; then
  set -a
  # shellcheck disable=SC1091
  source ".env.register"
  set +a
fi

: "${SIP_BIND_IP:=0.0.0.0}"
: "${SIP_PORT:=5060}"
: "${RTP_PORT:=10000}"
: "${LOCAL_IP:=0.0.0.0}"
: "${RECORDING_HTTP_ADDR:=0.0.0.0:18080}"
: "${RUST_LOG:=info}"

if [ -z "${ADVERTISED_IP:-}" ]; then
  ADVERTISED_IP="$LOCAL_IP"
fi

: "${REGISTRAR_PORT:=5060}"
: "${REGISTRAR_TRANSPORT:=udp}"
: "${REGISTER_EXPIRES:=3600}"

missing=0
if [ -z "${REGISTRAR_HOST:-}" ]; then
  echo "[run_register] REGISTRAR_HOST is required" >&2
  missing=1
fi
if [ -z "${REGISTER_USER:-}" ]; then
  echo "[run_register] REGISTER_USER is required" >&2
  missing=1
fi
if [ "$missing" -ne 0 ]; then
  exit 2
fi

if [ -z "${REGISTER_DOMAIN:-}" ]; then
  REGISTER_DOMAIN="$REGISTRAR_HOST"
fi
if [ -z "${REGISTER_AUTH_USER:-}" ]; then
  REGISTER_AUTH_USER="$REGISTER_USER"
fi

password_status="(not set)"
if [ -n "${REGISTER_AUTH_PASSWORD:-}" ]; then
  password_status="(set)"
fi

if [ "$REGISTRAR_TRANSPORT" != "udp" ]; then
  echo "[run_register] WARNING: outbound $REGISTRAR_TRANSPORT is not supported; use udp" >&2
fi

export SIP_BIND_IP SIP_PORT RTP_PORT LOCAL_IP ADVERTISED_IP RECORDING_HTTP_ADDR RUST_LOG
export REGISTRAR_HOST REGISTRAR_PORT REGISTRAR_TRANSPORT
export REGISTER_USER REGISTER_DOMAIN REGISTER_EXPIRES REGISTER_AUTH_USER REGISTER_AUTH_PASSWORD

echo "[run_register] SIP UDP/TCP  ${SIP_BIND_IP}:${SIP_PORT} (advertised ${ADVERTISED_IP})"
echo "[run_register] RTP          ${RTP_PORT}"
echo "[run_register] Registrar    ${REGISTRAR_HOST}:${REGISTRAR_PORT} (${REGISTRAR_TRANSPORT})"
echo "[run_register] Register     ${REGISTER_USER}@${REGISTER_DOMAIN} expires=${REGISTER_EXPIRES}"
echo "[run_register] Auth user    ${REGISTER_AUTH_USER} password ${password_status}"
echo "[run_register] RUST_LOG     ${RUST_LOG}"

cargo run --bin virtual-voicebot-backend
