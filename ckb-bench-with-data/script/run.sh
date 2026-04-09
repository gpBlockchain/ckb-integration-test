#!/usr/bin/env bash

set -euo pipefail
START_TIME=${START_TIME:-"$(date +%Y-%m-%d' '%H:%M:%S.%6N)"}

SCRIPT_PATH="$( cd -- "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P )"

pip install paramiko pyyaml tomlkit -q 2>/dev/null || true

main() {
  case $1 in
    "run")
      python3 "$SCRIPT_PATH/deploy.py" run ${2:-1000w}
      ;;
    "setup")
      python3 "$SCRIPT_PATH/deploy.py" setup
      ;;
    "deploy_ckb")
      python3 "$SCRIPT_PATH/deploy.py" deploy_ckb ${2:-1000w}
      ;;
    "run_ckb")
      python3 "$SCRIPT_PATH/deploy.py" run_ckb
      ;;
    "clean_ckb_env")
      python3 "$SCRIPT_PATH/deploy.py" clean_ckb_env
      ;;
    "clean_ckb_bench")
      python3 "$SCRIPT_PATH/deploy.py" clean_ckb_bench
      ;;
    "clean_job")
      python3 "$SCRIPT_PATH/deploy.py" clean_job
      ;;
    "add_node")
      python3 "$SCRIPT_PATH/deploy.py" add_node
      ;;
    "bench")
      python3 "$SCRIPT_PATH/deploy.py" bench
      ;;
    "get_log")
      python3 "$SCRIPT_PATH/deploy.py" get_log
      ;;
    "restart_ckb")
      python3 "$SCRIPT_PATH/deploy.py" restart_ckb ${2:-0}
      ;;
  esac
}

main $*
