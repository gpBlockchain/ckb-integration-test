#!/usr/bin/env bash


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

ansible_config() {
  export ANSIBLE_PRIVATE_KEY_FILE=$SSH_PRIVATE_KEY_PATH
  export ANSIBLE_INVENTORY=$ANSIBLE_INVENTORY
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

    # quake
#    ckb_remote_url="http://18.162.180.86:8000/ckb_v0.110.1_aarch64-unknown-linux-gnu.tar.gz"
# develop
    ckb_remote_url="http://github-test-logs.ckbapp.dev/ckb/bin/ckb-develop-x86_64-unknown-linux-gnu-portable.tar.gz"
    ckb_data_remote_url=$2
    cd $ANSIBLE_DIRECTORY
    ckb_download_tmp_dir="/tmp"
    ansible-playbook playbook.yml \
          -e "ckb_download_url=$ckb_remote_url node=$1" \
          -e "ckb_download_tmp_dir=$ckb_download_tmp_dir" \
          -t ckb_install
    ansible-playbook playbook.yml \
      -e "ckb_download_url=$ckb_remote_url node=$1" \
      -e "ckb_data_download_url=$ckb_data_remote_url" \
      -t ckb_data_install,ckb_configure,ckb_restart
    return
  fi
#  ckb_remote_url="https://github.com/nervosnetwork/ckb/releases/download/${download_ckb_version}/ckb_${download_ckb_version}_x86_64-unknown-centos-gnu.tar.gz"
  #quake
  ckb_remote_url="http://github-test-logs.ckbapp.dev/ckb/bin/ckb-develop-x86_64-unknown-linux-gnu-portable.tar.gz"
# develop
#    ckb_remote_url="http://18.162.180.86:8000/ckb_v0.111.0-rc10_x86_64-unknown-linux-gnu-portable.tar.gz"

  ckb_data_remote_url=$2
  ckb_download_tmp_dir="/tmp"
  cd $ANSIBLE_DIRECTORY
  ansible-playbook playbook.yml \
            -e "ckb_download_url=$ckb_remote_url node=$1" \
            -e "ckb_download_tmp_dir=$ckb_download_tmp_dir" \
            -t ckb_install
      ansible-playbook playbook.yml \
        -e "ckb_download_url=$ckb_remote_url node=$1" \
        -e "ckb_data_download_url=$ckb_data_remote_url" \
        -t ckb_data_install,ckb_configure,ckb_restart
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
    -e "node=$1" \
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

ansible_ckb_miner_start() {
  ansible_config
  cd $ANSIBLE_DIRECTORY
  ansible-playbook playbook.yml \
    -e "node=$1" \
    -t ckb_miner_start
}



function ansible_wait_ckb_benchmark() {
    ansible_config
    cd $ANSIBLE_DIRECTORY
    ansible-playbook playbook.yml -e 'hostname=bastions' -e 'node=bastions' -t ckb_benchmark_install
    ansible-playbook playbook.yml -e 'hostname=bastions' -e 'node=bastions' -t ckb_benchmark_miner_start
    ansible-playbook playbook.yml -e 'hostname=bastions' -e 'node=bastions ckb_bench_tps=10' -t ckb_benchmark_with_tps
    ansible-playbook playbook.yml -e 'hostname=bastions' -e 'node=bastions ckb_bench_tps=150' -t ckb_benchmark_with_tps
    ansible_ckb_miner_start node2
    ansible-playbook playbook.yml -e 'hostname=bastions' -e 'node=bastions ckb_bench_tps=2000' -t ckb_benchmark_with_tps
    ansible-playbook playbook.yml -e 'hostname=bastions' -e 'node=bastions' -e "ckb_bench_log_file=demo.tar.gz" -t process_result
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
      ansible_deploy_download_ckb node1 "http://172.31.45.113:8000/data.1000w.tar.gz" &
      ansible_deploy_download_ckb node2 "http://172.31.45.113:8000/data.1000w.tar.gz" &
      ansible_deploy_download_ckb node3 "http://172.31.45.113:8000/data.1000w.tar.gz" &
      wait
      echo " deploy successful"
      sleep 20
      wait node start
      echo "link nodes "
      link_node_p2p node1 node2
      link_node_p2p node1 node3
      link_node_p2p node2 node1
      link_node_p2p node2 node3
      link_node_p2p node3 node1
      link_node_p2p node3 node2
      echo "start bench "
      ansible_wait_ckb_benchmark

#      clean_ckb_env node1 &
#      clean_ckb_env node2 &
#      clean_ckb_env node3 &
#      wait

#      clean_ckb_bench_env &
#      wait

#      ansible_deploy_download_ckb node1 "http://172.31.45.113:8000/data.3000w.tar.gz" &
#      ansible_deploy_download_ckb node2 "http://172.31.45.113:8000/data.3000w.tar.gz" &
#      ansible_deploy_download_ckb node3 "http://172.31.45.113:8000/data.3000w.tar.gz" &
#      wait
#      sleep 30
#      link_node_p2p node1 node2
#      link_node_p2p node1 node3
#      link_node_p2p node2 node3
#      ansible_wait_ckb_benchmark 3000

#      clean_ckb_env node1
#      clean_ckb_env node2
#      clean_ckb_env node3
#      clean_ckb_bench_env
      ;;
    "setup")
      job_setup
      ;;
    "deploy_ckb")
      ansible_deploy_download_ckb node1 "http://172.31.45.113:8000/data.1000w.tar.gz" &
      ansible_deploy_download_ckb node2 "http://172.31.45.113:8000/data.1000w.tar.gz" &
      ansible_deploy_download_ckb node3 "http://172.31.45.113:8000/data.1000w.tar.gz" &
      wait
      echo "deploy successful"
      link_node_p2p node1 node2
      link_node_p2p node1 node3
      link_node_p2p node2 node1
      link_node_p2p node2 node3
      link_node_p2p node3 node1
      link_node_p2p node3 node2
      echo "link successful"
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
      link_node_p2p node2 node1
      link_node_p2p node2 node3
      link_node_p2p node3 node1
      link_node_p2p node3 node2
      ;;
    "bench")
      ansible_wait_ckb_benchmark
      ;;
    "get_log")
      ansible_process_result
      ;;
  esac
}

main $*


