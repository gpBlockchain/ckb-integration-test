#!/usr/bin/env bash

set -Eeuo pipefail

SCRIPT_DIRECTORY="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd -P)"
VENV_DIRECTORY=${VENV_DIRECTORY:-"$SCRIPT_DIRECTORY/.venv"}
PYTHON_BIN=${PYTHON_BIN:-python3}

cleanup() {
  if ! bash script/run.sh clean_job; then
    echo "Warning: benchmark cleanup failed" >&2
  fi
}

cd "$SCRIPT_DIRECTORY"
trap cleanup EXIT

bash script/run.sh setup
bash script/run.sh run 98400f6a67af07025f5959af35ed653d649f745b8f54bf3f07bef9bd605ee946.1w.1024w.cell
bash script/run.sh bench

if [[ ! -x "$VENV_DIRECTORY/bin/python" ]]; then
  "$PYTHON_BIN" -m venv "$VENV_DIRECTORY"
fi
"$VENV_DIRECTORY/bin/python" -m pip install --requirement requirements.txt
"$VENV_DIRECTORY/bin/python" script/gen_report.py
report=$(<demo.md)
export GITHUB_TOKEN
bash script/ok.sh add_comment nervosnetwork/acceptance-internal 1222 "$report"

"$VENV_DIRECTORY/bin/python" wkr.py
report=$(<wkr.md)
bash script/ok.sh add_comment nervosnetwork/acceptance-internal 1222 "$report"
