#!/usr/bin/env bash

# ENVIRONMENT VARIABLES:
#
#   * AWS_ACCESS_KEY, required, the AWS access key
#   * AWS_SECRET_KEY, required, the AWS secret key
#   * AWS_EC2_TYPE, optional, default is c5.xlarge, the AWS EC2 type
#   * GITHUB_TOKEN, required, GitHub API authentication token
set -euo pipefail

AWS_ACCESS_KEY=${AWS_ACCESS_KEY}
AWS_SECRET_KEY=${AWS_SECRET_KEY}
AWS_EC2_TYPE=${AWS_EC2_TYPE:-"c5.xlarge"}
GITHUB_TOKEN=${GITHUB_TOKEN}

download_ckb_version="latest"
JOB_ID=${JOB_ID:-"sync-mainnet-$(date +'%Y-%m-%d')-in-10h"}
TAR_FILENAME="ckb.sync-mainnet-$(date +'%Y-%m-%d')-in-10h.tar.gz"
SCRIPT_PATH="$(
  cd -- "$(dirname "$0")" >/dev/null 2>&1
  pwd -P
)"
JOB_DIRECTORY="$(dirname "$SCRIPT_PATH")/job/$JOB_ID"
ANSIBLE_DIRECTORY=$JOB_DIRECTORY/ansible
ANSIBLE_INVENTORY=$JOB_DIRECTORY/ansible/inventory.yml
TERRAFORM_DIRECTORY="$JOB_DIRECTORY/terraform"
SSH_PRIVATE_KEY_PATH=$JOB_DIRECTORY/ssh/id
SSH_PUBLIC_KEY_PATH=$JOB_DIRECTORY/ssh/id.pub
START_TIME=${START_TIME:-"$(date +%Y-%m-%d' '%H:%M:%S.%6N)"}
GITHUB_REF_NAME=${GITHUB_REF_NAME:-"develop"}
GITHUB_REPOSITORY=${GITHUB_REPOSITORY:-"nervosnetwork/ckb"}
GITHUB_BRANCH=${GITHUB_BRANCH:-"$GITHUB_REF_NAME"}

pip install paramiko pyyaml tomlkit -q 2>/dev/null || true

terraform_config() {
  export TF_VAR_access_key=$AWS_ACCESS_KEY
  export TF_VAR_secret_key=$AWS_SECRET_KEY
  export TF_VAR_prefix=$JOB_ID
  export TF_VAR_private_key_path=$SSH_PRIVATE_KEY_PATH
  export TF_VAR_public_key_path=$SSH_PUBLIC_KEY_PATH
}

terraform_apply() {
  terraform_config

  cd $TERRAFORM_DIRECTORY
  terraform init
  terraform plan
  terraform apply -auto-approve
  terraform output | grep -v EOT | tee $ANSIBLE_INVENTORY
}

terraform_destroy() {
  terraform_config

  cd $TERRAFORM_DIRECTORY
  terraform destroy -auto-approve
}

build_ckb() {
  git -C $JOB_DIRECTORY clone \
    --branch $GITHUB_BRANCH \
    --depth 1 \
    https://github.com/$GITHUB_REPOSITORY.git

  cd $JOB_DIRECTORY/ckb
  make build

  cd target/release
  tar czf "$TAR_FILENAME" ckb
}

github_add_comment() {
  export GITHUB_TOKEN=${GITHUB_TOKEN}
  report="$1"
  $SCRIPT_PATH/ok.sh add_comment nervosnetwork/ckb 2372 "$report"

  CKB_HEAD_REF=$(cd $JOB_DIRECTORY/ckb && git log --pretty=format:'%h' -n 1)
  $SCRIPT_PATH/ok.sh add_commit_comment nervosnetwork/ckb $CKB_HEAD_REF "$report"
}

insert_report_to_postgres() {
  export PGHOST=${PGHOST}
  export PGPORT=${PGPORT}
  export PGUSER=${PGUSER}
  export PGPASSWORD=${PGPASSWORD}
  export PGDATABASE=${PGDATABASE}
  export GITHUB_RUN_ID=${GITHUB_RUN_ID}
  export CKB_COMMIT_ID=${CKB_COMMIT_ID}
  export CKB_COMMIT_TIME=${CKB_COMMIT_TIME}
  export GITHUB_RUN_STATE=${GITHUB_RUN_STATE:-0}
  export GITHUB_EVENT_NAME=${GITHUB_EVENT_NAME}
  END_TIME=$(date +%Y-%m-%d' '%H:%M:%S.%6N)
  GITHUB_RUN_LINK="https://github.com/${GITHUB_REPOSITORY}/actions/runs/$GITHUB_RUN_ID"
  psql -c "INSERT INTO sync_mainnet (github_run_id,github_run_state,start_time,end_time,github_branch,github_trigger_event,github_run_link)  \
             VALUES ('$GITHUB_RUN_ID','$GITHUB_RUN_STATE','$START_TIME','$END_TIME','$GITHUB_BRANCH','$GITHUB_EVENT_NAME','$GITHUB_RUN_LINK');"

  time=$START_TIME
  if [ -n "'ls $ANSIBLE_DIRECTORY/*.brief.md'" ]; then
    cat $ANSIBLE_DIRECTORY/*.brief.md >$ANSIBLE_DIRECTORY/sync-mainnet.brief.md
  fi
  if [ -f "$ANSIBLE_DIRECTORY/sync-mainnet.brief.md" ]; then
    while read -r LINE; do
      LINE=$(echo "$LINE" | sed -e 's/\r//g')
      ckb_version=$(echo $LINE | awk -F '|' '{print $2}')
      time_s=$(echo $LINE | awk -F '|' '{print $3}')
      speed=$(echo $LINE | awk -F '|' '{print $4}')
      tip=$(echo $LINE | awk -F '|' '{print $5}')
      hostname=$(echo $LINE | awk -F '|' '{print $6}')
      replay_tps=$(echo $LINE | awk -F '|' '{print $8}')
      psql -c "INSERT INTO sync_mainnet_report (github_run_id,time,ckb_version,ckb_commit_id,ckb_commit_time,time_s,speed,tip,hostname,replay_tps)  \
             VALUES ('$GITHUB_RUN_ID','$time','$ckb_version','$CKB_COMMIT_ID','$CKB_COMMIT_TIME','$time_s','$speed','$tip','$hostname','$replay_tps');"
    done <"$ANSIBLE_DIRECTORY/sync-mainnet.brief.md"
  fi
}

main() {
  case $1 in
    "run")
      python3 "$SCRIPT_PATH/deploy.py" setup
      terraform_apply
      build_ckb
      python3 "$SCRIPT_PATH/deploy.py" ansible
      github_add_comment "$(python3 "$SCRIPT_PATH/deploy.py" report)"
      ;;
    "setup")
      python3 "$SCRIPT_PATH/deploy.py" setup
      ;;
    "build")
      build_ckb
      ;;
    "terraform")
      terraform_apply
      ;;
    "ansible")
      python3 "$SCRIPT_PATH/deploy.py" ansible
      ;;
    "report")
      python3 "$SCRIPT_PATH/deploy.py" report
      ;;
    "clean")
      terraform_destroy
      python3 "$SCRIPT_PATH/deploy.py" clean
      ;;
   "clean_ckb_env")
      python3 "$SCRIPT_PATH/deploy.py" clean_ckb_env
      ;;
   "clean_job")
      python3 "$SCRIPT_PATH/deploy.py" clean_job
      ;;
   "insert_report_to_postgres")
      insert_report_to_postgres
      ;;
  esac
}

main $*
