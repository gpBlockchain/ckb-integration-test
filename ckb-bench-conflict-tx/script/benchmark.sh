#!/usr/bin/env bash

set -euo pipefail

START_TIME=${START_TIME:-"$(date +%Y-%m-%d' '%H:%M:%S.%6N)"}

SCRIPT_PATH="$( cd -- "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P )"

pip install paramiko pyyaml -q 2>/dev/null || true

function main() {
    case $1 in
        "setup")
            python3 "$SCRIPT_PATH/deploy.py" setup
            ;;
        "run")
            python3 "$SCRIPT_PATH/deploy.py" run
            ;;
        "deploy")
            python3 "$SCRIPT_PATH/deploy.py" deploy
            ;;
        "clean")
            python3 "$SCRIPT_PATH/deploy.py" clean
            ;;
    esac
}

main $*
