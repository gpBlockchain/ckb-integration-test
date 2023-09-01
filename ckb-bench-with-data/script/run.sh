#!/usr/bin/env bash


set -x
set -euo pipefail

START_TIME=${START_TIME:-"$(date +%Y-%m-%d' '%H:%M:%S.%6N)"}
#  latest or v0.110.0 ...
download_ckb_version="v0.111.0-rc10"

JOB_ID=${JOB_ID:-"benchmark-$(date +'%Y-%m-%d')-in-10h"}
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

ansible_config() {
  export ANSIBLE_PRIVATE_KEY_FILE=$SSH_PRIVATE_KEY_PATH
  export ANSIBLE_INVENTORY=$ANSIBLE_INVENTORY
}

function ssh_gen_key() {
    echo $SSH_ID > $SSH_PRIVATE_KEY_PATH
    echo $SSH_ID_PUB > $SSH_PUBLIC_KEY_PATH
}

# Setup Ansible running environment.
function ansible_setup() {
    cd $ANSIBLE_DIRECTORY
    ansible-galaxy install -r requirements.yml --force
}


# Deploy CKB onto target AWS EC2 instances.
### $1 : node1 ,node2 node3...
ansible_deploy_download_ckb() {
  ansible_config

  if [ ${download_ckb_version} == "latest" ]; then
    ckb_remote_url=`curl --silent "https://api.github.com/repos/nervosnetwork/ckb/releases/latest" | jq -r ".assets[].browser_download_url" | grep unknown-linux-gnu-portable | grep -v asc`
    ckb_remote_url="http://18.162.180.86:8000/ckb_v0.110.1_aarch64-unknown-linux-gnu.tar.gz"
    ckb_data_remote_url=$2
    cd $ANSIBLE_DIRECTORY
    ansible-playbook playbook.yml \
      -e "ckb_download_url=$ckb_remote_url node=$1" \
      -t ckb_install,ckb_data_install,ckb_configure
    return
  fi
  ckb_remote_url="https://github.com/nervosnetwork/ckb/releases/download/${download_ckb_version}/ckb_${download_ckb_version}_x86_64-unknown-centos-gnu.tar.gz"
  ckb_remote_url="http://18.162.180.86:8000/ckb_v0.110.1_aarch64-unknown-linux-gnu.tar.gz"
  ckb_data_remote_url=$2
  cd $ANSIBLE_DIRECTORY
  ansible-playbook playbook.yml \
    -e "ckb_download_url=$ckb_remote_url node=$1" \
    -e "ckb_data_download_url=$ckb_data_remote_url" \
    -t ckb_install,ckb_data_install,ckb_configure
}

### $1 : node1 ,node2 node3...
ansible_run_ckb() {
  ansible_config
  cd $ANSIBLE_DIRECTORY
  ansible-playbook playbook.yml \
    -e "node=$1" \
    -t ckb_start
}


function link_node_p2p() {
  ansible_config
  cd $ANSIBLE_DIRECTORY
  ansible-playbook playbook.yml \
    -e "node=localhost" \
    -e "n1=$1" \
    -e "n2=$2" \
    -t ckb_add_node
}

# $1: node1 ,node2 ...
clean_ckb_env(){
  ansible_config
  cd $ANSIBLE_DIRECTORY
  ansible-playbook playbook.yml \
    -e "node=$1" \
    -t ckb_stop,ckb_clean
}

ckb_miner(){
  ansible_config
    cd $ANSIBLE_DIRECTORY
    ansible-playbook playbook.yml \
      -e "node=$1" \
      -t ckb_miner_start
}

#// node false
function ckb_set_network_active() {
    ansible_config
    cd $ANSIBLE_DIRECTORY
    ansible-playbook playbook.yml \
      -e "node=localhost" \
      -e "n1=$1" \
      -e "network_status=$2" \
      -t ckb_set_network_active
}


function ansible_wait_ckb_benchmark() {
    ansible_config
    cd $ANSIBLE_DIRECTORY
    ckb_bench_log_file="logs/data-${1}.tar.gz"
    ansible-playbook playbook.yml -e 'hostname=bastions' -e 'node=bastions' -t ckb_benchmark_install
    ansible-playbook playbook.yml -e 'hostname=bastions' -e 'node=bastions' -t ckb_benchmark_miner_start
    ansible-playbook playbook.yml -e 'hostname=bastions' -e 'node=bastions' -t ckb_benchmark_with_tps
    ansible-playbook playbook.yml -e 'hostname=bastions' -e 'node=bastions' -e "ckb_bench_log_file=$ckb_bench_log_file" -t process_result
}

function ansible_process_result() {
      ansible_config
      cd $ANSIBLE_DIRECTORY
      ansible-playbook playbook.yml -e 'hostname=bastions' -e 'node=bastions' -t process_result
}

function clean_ckb_bench_env(){
  ansible_config
  cd $ANSIBLE_DIRECTORY
  ansible-playbook playbook.yml \
    -e 'hostname=bastions' -e 'node=bastions' \
    -t ckb_bench_stop,ckb_bench_clean
}




main() {
  case $1 in
    "run")
      job_setup
      ansible_deploy_download_ckb node1 "http://172.31.45.113:8000/data.1001w.tar.gz" &
      ansible_deploy_download_ckb node2 "http://172.31.45.113:8000/data.1001w.tar.gz" &
      ansible_deploy_download_ckb node3 "http://172.31.45.113:8000/data.1001w.tar.gz" &
      wait

      link_node_p2p node1 node2
      link_node_p2p node1 node3
      link_node_p2p node2 node3
      wait

      ansible_wait_ckb_benchmark 1000

      clean_ckb_env node1 &
      clean_ckb_env node2 &
      clean_ckb_env node3 &
      clean_ckb_bench_env &
      wait

      ansible_deploy_download_ckb node1 "http://172.31.45.113:8000/data.3000w.tar.gz" &
      ansible_deploy_download_ckb node2 "http://172.31.45.113:8000/data.3000w.tar.gz" &
      ansible_deploy_download_ckb node3 "http://172.31.45.113:8000/data.3000w.tar.gz" &
      wait
      sleep 30
      link_node_p2p node1 node2
      link_node_p2p node1 node3
      link_node_p2p node2 node3
      ansible_wait_ckb_benchmark 3000

#      clean_ckb_env node1
#      clean_ckb_env node2
#      clean_ckb_env node3
#      clean_ckb_bench_env
      ;;
    "setup")
      job_setup
      ;;
    "deploy_ckb")
      ansible_deploy_download_ckb node1
      ansible_deploy_download_ckb node2
      ansible_deploy_download_ckb node3
      ;;
    "run_ckb")
      ansible_run_ckb node1
      ansible_run_ckb node2
      ansible_run_ckb node3
      ;;
   "clean_ckb_env")
      clean_ckb_env node1
      clean_ckb_env node2
      clean_ckb_env node3
      ;;
    "clean_ckb_bench")
      clean_ckb_bench_env
      ;;
   "clean_job")
      clean_ckb_env node1
      clean_ckb_env node2
      clean_ckb_env node3
      clean_ckb_bench_env
      ;;
    "add_node")
      link_node_p2p node1 node2
      link_node_p2p node1 node3
      link_node_p2p node2 node3
      ;;
    "bench")
      ansible_wait_ckb_benchmark 300
      ;;
    "get_log")
      ansible_process_result
      ;;
  esac
}

main $*


