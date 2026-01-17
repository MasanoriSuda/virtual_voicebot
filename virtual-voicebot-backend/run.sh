#!/usr/bin/env bash
set -euo pipefail

MODE="${1:-always}"

case "$MODE" in
  always|all|custom) ;;
  *)
    echo "usage: ./run.sh [always|all|custom]" >&2
    exit 2
    ;;
esac

cargo run -q --bin sipp_test -- "$MODE"
