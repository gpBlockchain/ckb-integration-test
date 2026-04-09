#!/usr/bin/env bash

set -x
set -euo pipefail

SCRIPT_PATH="$( cd -- "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P )"

pip install paramiko pyyaml -q 2>/dev/null || true

main() {
  case $1 in
    "setup")
      python3 "$SCRIPT_PATH/deploy.py" setup
      ;;
    "run")
      python3 "$SCRIPT_PATH/deploy.py" run
      ;;
    "deploy_ckb")
      python3 "$SCRIPT_PATH/deploy.py" deploy_ckb
      ;;
    "run_ckb")
      python3 "$SCRIPT_PATH/deploy.py" run_ckb
      ;;
    "miner")
      python3 "$SCRIPT_PATH/deploy.py" miner
      ;;
    "clean_ckb_env")
      python3 "$SCRIPT_PATH/deploy.py" clean_ckb_env
      ;;
    "add_node")
      python3 "$SCRIPT_PATH/deploy.py" add_node
      ;;
    "ckb_pending")
      python3 "$SCRIPT_PATH/deploy.py" ckb_pending ${2:-100}
      ;;
    "get-prepare-data")
      cd "$(dirname "$SCRIPT_PATH")/ansible/files"
      git clone https://github.com/gpBlockchain/ckb-prepare-data.git && \
      cd ckb-prepare-data && \
      git checkout 10b6d09b5f0cf898f46d4fa6f018007262461918 && \
      git rm -r * && \
      git checkout HEAD -- big-tx
      ;;
  esac
}

main $*
