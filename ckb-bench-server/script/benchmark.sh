#!/usr/bin/env bash

set -euo pipefail

set -euo pipefail
START_TIME=${START_TIME:-"$(date +%Y-%m-%d' '%H:%M:%S.%6N)"}
#  latest or v0.110.0 ...
download_ckb_version="v0.111.0-rc10"

JOB_ID="benchmark-in-10h"
SCRIPT_PATH="$( cd -- "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P )"
JOB_DIRECTORY="$(dirname "$SCRIPT_PATH")/job/$JOB_ID"
ANSIBLE_DIRECTORY=$JOB_DIRECTORY/ansible
ANSIBLE_INVENTORY=$JOB_DIRECTORY/ansible/inventory.yml
SSH_PRIVATE_KEY_PATH=$JOB_DIRECTORY/ssh/id
SSH_PUBLIC_KEY_PATH=$JOB_DIRECTORY/ssh/id.pub


function job_setup() {
    mkdir -p $JOB_DIRECTORY
    cp -r "$(dirname "$SCRIPT_PATH")/ansible"   $JOB_DIRECTORY/ansible

    ssh_gen_key
    ansible_setup
}

function job_clean() {
    rm -rf $JOB_DIRECTORY
}

clean_ckb_env(){
  ansible_config
  cd $ANSIBLE_DIRECTORY
  ansible-playbook playbook.yml \
    -e "node=$1" \
    -t ckb_stop,ckb_clean
}


function clean_ckb_bench_env(){
  ansible_config
  cd $ANSIBLE_DIRECTORY
  ansible-playbook playbook.yml \
    -e 'hostname=bastions' -e 'node=bastions' \
    -t ckb_bench_stop,ckb_bench_clean
}




function ssh_gen_key() {
    mkdir  -p $JOB_DIRECTORY/ssh
    IFS='@' read -ra elements <<< "$SSH_ID"
    for element in "${elements[@]}"; do
      echo "$element" >> $SSH_PRIVATE_KEY_PATH
    done
    echo $SSH_ID_PUB > $SSH_PUBLIC_KEY_PATH
    chmod 600 $SSH_PRIVATE_KEY_PATH
}


function ansible_config() {
    export ANSIBLE_PRIVATE_KEY_FILE=$SSH_PRIVATE_KEY_PATH
    export ANSIBLE_INVENTORY=$ANSIBLE_INVENTORY
}

# Setup Ansible running environment.
function ansible_setup() {
    cd $ANSIBLE_DIRECTORY
    ansible-galaxy install -r requirements.yml --force
}


ansible_deploy_download_ckb() {
  ansible_config
  if [ ${download_ckb_version} == "latest" ]; then
    ckb_remote_url="http://github-test-logs.ckbapp.dev/ckb/bin/ckb-develop-x86_64-unknown-linux-gnu-portable.tar.gz"
    cd $ANSIBLE_DIRECTORY
    ansible-playbook playbook.yml \
      -e "ckb_download_url=$ckb_remote_url node=$1" \
      -t ckb_install,ckb_data_install,ckb_configure
    return
  fi
  ckb_remote_url="http://github-test-logs.ckbapp.dev/ckb/bin/ckb-develop-x86_64-unknown-linux-gnu-portable.tar.gz"
  cd $ANSIBLE_DIRECTORY
  ansible-playbook playbook.yml \
    -e "ckb_download_url=$ckb_remote_url node=$1 ckb_download_tmp_dir=/tmp" \
    -t ckb_install,ckb_configure,ckb_restart,ckb_miner_start
}



# Wait for CKB synchronization completion.
function ansible_wait_ckb_benchmark() {
    ansible_config
    cd $ANSIBLE_DIRECTORY
    ansible-playbook playbook.yml -e 'hostname=bastions node=bastions'  -t ckb_benchmark_install
    ansible-playbook playbook.yml -e 'hostname=bastions node=bastions'  -t ckb_benchmark_prepare
    ansible-playbook playbook.yml -e 'hostname=bastions node=bastions'  -t ckb_bench_stop
    ansible-playbook playbook.yml -e 'hostname=bastions node=bastions ckb_bench_n_inout=1'  -t ckb_benchmark_with_tps
    ansible-playbook playbook.yml -e 'hostname=bastions node=bastions ckb_bench_n_inout=2'  -t ckb_benchmark_with_tps
    ansible-playbook playbook.yml -e 'hostname=bastions node=bastions ckb_bench_n_inout=5'  -t ckb_benchmark_with_tps
    ansible-playbook playbook.yml -e 'hostname=bastions node=bastions ckb_bench_n_inout=10'  -t ckb_benchmark_with_tps
    ansible-playbook playbook.yml -e 'hostname=bastions' -e 'node=bastions' -e "ckb_bench_log_file=demo.tar.gz" -t process_result
}

function main() {
    case $1 in
        "run")
            ansible_deploy_download_ckb node2
            ansible_wait_ckb_benchmark
            ;;
        "setup")
            job_setup
            ;;
        "clean")
            clean_ckb_env node2
            clean_ckb_bench_env
            ;;
        esac
}

main $*
