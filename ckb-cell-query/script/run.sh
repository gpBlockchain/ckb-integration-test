#!/usr/bin/env bash


set -euo pipefail
START_TIME=${START_TIME:-"$(date +%Y-%m-%d' '%H:%M:%S.%6N)"}
#  latest or v0.110.0 ...

JOB_ID="benchmark-in-10h"
SCRIPT_PATH="$( cd -- "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P )"
JOB_DIRECTORY="$(dirname "$SCRIPT_PATH")/job/$JOB_ID"
ANSIBLE_DIRECTORY=$JOB_DIRECTORY/ansible
ANSIBLE_INVENTORY=$JOB_DIRECTORY/ansible/inventory.yml
SSH_PRIVATE_KEY_PATH=$JOB_DIRECTORY/ssh/id
SSH_PUBLIC_KEY_PATH=$JOB_DIRECTORY/ssh/id.pub
WARMUP_BLOCKS=${WARMUP_BLOCKS:-10}
SYNC_TIMEOUT_SECONDS=${SYNC_TIMEOUT_SECONDS:-600}
SYNC_POLL_INTERVAL_SECONDS=${SYNC_POLL_INTERVAL_SECONDS:-2}
if [ -z "${CKB_REMOTE_URL:-}" ]; then
    CKB_REMOTE_URL="http://github-test-logs.ckbapp.dev/ckb/bin/ckb-develop-x86_64-unknown-linux-gnu-portable.tar.gz"
fi

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
#    echo $SSH_ID_PUB > $SSH_PUBLIC_KEY_PATH
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

  ckb_data_remote_url=$2
  ckb_download_tmp_dir="/tmp"
  cd $ANSIBLE_DIRECTORY
  ansible-playbook playbook.yml \
            -e "ckb_download_url=$CKB_REMOTE_URL node=$1" \
            -e "ckb_download_tmp_dir=$ckb_download_tmp_dir" \
            -t ckb_install
      ansible-playbook playbook.yml \
        -e "ckb_download_url=$CKB_REMOTE_URL node=$1" \
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

ansible_ckb_miner_stop() {
  ansible_config
  cd $ANSIBLE_DIRECTORY
  ansible-playbook playbook.yml \
    -e "node=$1" \
    -t ckb_miner_stop
}

validate_sync_settings() {
  local value

  for value in "$WARMUP_BLOCKS" "$SYNC_TIMEOUT_SECONDS" "$SYNC_POLL_INTERVAL_SECONDS"; do
    if ! [[ "$value" =~ ^[1-9][0-9]*$ ]]; then
      echo "WARMUP_BLOCKS, SYNC_TIMEOUT_SECONDS, and SYNC_POLL_INTERVAL_SECONDS must be positive integers" >&2
      return 1
    fi
  done
}

node_rpc_url() {
  local node=$1
  local host
  local port

  host=$(ansible-inventory --inventory "$ANSIBLE_INVENTORY" --host "$node" | jq -er '.ansible_host')
  port=$(awk -F: '/^[[:space:]]*ckb_rpc_listen_address:/ { gsub(/[[:space:]\"]/, "", $0); print $NF; exit }' "$ANSIBLE_DIRECTORY/vars/$node.yml")

  if ! [[ "$port" =~ ^[0-9]+$ ]]; then
    echo "Unable to determine the RPC port for $node" >&2
    return 1
  fi

  printf 'http://%s:%s\n' "$host" "$port"
}

ckb_tip_header() {
  local rpc_url=$1

  curl --fail --silent --show-error --connect-timeout 5 --max-time 10 \
    --header 'Content-Type: application/json' \
    --data '{"id": 0, "jsonrpc": "2.0", "method": "get_tip_header", "params": []}' \
    "$rpc_url" | jq -er '.result | select(.number != null and .hash != null) | [.number, .hash] | @tsv'
}

tip_number_to_decimal() {
  printf '%d\n' "$1"
}

wait_for_node1_warmup_blocks() {
  local node1_rpc_url=$1
  local initial_tip
  local current_tip
  local initial_number
  local current_number
  local target_number
  local deadline

  if ! initial_tip=$(ckb_tip_header "$node1_rpc_url"); then
    echo "Unable to read node1's initial tip from $node1_rpc_url" >&2
    return 1
  fi
  read -r initial_number _ <<< "$initial_tip"
  target_number=$(( $(tip_number_to_decimal "$initial_number") + WARMUP_BLOCKS ))
  deadline=$(( $(date +%s) + SYNC_TIMEOUT_SECONDS ))

  echo "Waiting for node1 to mine $WARMUP_BLOCKS warm-up blocks (target height: $target_number)"
  while (( $(date +%s) < deadline )); do
    if current_tip=$(ckb_tip_header "$node1_rpc_url"); then
      read -r current_number _ <<< "$current_tip"
      if (( $(tip_number_to_decimal "$current_number") >= target_number )); then
        echo "node1 reached warm-up height $current_number"
        return 0
      fi
    fi
    sleep "$SYNC_POLL_INTERVAL_SECONDS"
  done

  echo "Timed out waiting for node1 to mine $WARMUP_BLOCKS warm-up blocks" >&2
  return 1
}

wait_for_nodes_to_sync() {
  local node1_rpc_url=$1
  local node2_rpc_url=$2
  local node3_rpc_url=$3
  local node1_tip
  local node2_tip
  local node3_tip
  local deadline

  if ! node1_tip=$(ckb_tip_header "$node1_rpc_url"); then
    echo "Unable to read node1's tip before synchronization" >&2
    return 1
  fi
  deadline=$(( $(date +%s) + SYNC_TIMEOUT_SECONDS ))

  echo "Waiting for node2 and node3 to sync to node1 tip: $node1_tip"
  while (( $(date +%s) < deadline )); do
    if node2_tip=$(ckb_tip_header "$node2_rpc_url") \
      && node3_tip=$(ckb_tip_header "$node3_rpc_url") \
      && [[ "$node2_tip" == "$node1_tip" ]] \
      && [[ "$node3_tip" == "$node1_tip" ]]; then
      echo "node2 and node3 are synchronized with node1"
      return 0
    fi
    sleep "$SYNC_POLL_INTERVAL_SECONDS"
  done

  echo "Timed out waiting for node2 and node3 to synchronize with node1" >&2
  return 1
}

warm_up_and_sync_nodes() {
  local node1_rpc_url
  local node2_rpc_url
  local node3_rpc_url

  validate_sync_settings
  node1_rpc_url=$(node_rpc_url node1)
  node2_rpc_url=$(node_rpc_url node2)
  node3_rpc_url=$(node_rpc_url node3)

  ansible_ckb_miner_start node1
  if ! wait_for_node1_warmup_blocks "$node1_rpc_url"; then
    ansible_ckb_miner_stop node1
    return 1
  fi
  ansible_ckb_miner_stop node1

  wait_for_nodes_to_sync "$node1_rpc_url" "$node2_rpc_url" "$node3_rpc_url"
}

wait_for_existing_node_sync() {
  local node1_rpc_url
  local node2_rpc_url
  local node3_rpc_url

  validate_sync_settings
  node1_rpc_url=$(node_rpc_url node1)
  node2_rpc_url=$(node_rpc_url node2)
  node3_rpc_url=$(node_rpc_url node3)

  wait_for_nodes_to_sync "$node1_rpc_url" "$node2_rpc_url" "$node3_rpc_url"
}

link_all_nodes() {
  link_node_p2p node1 node2
  link_node_p2p node1 node3
  link_node_p2p node2 node1
  link_node_p2p node2 node3
  link_node_p2p node3 node1
  link_node_p2p node3 node2
}



ansible_ckb_restart() {
  ansible_config
  cd $ANSIBLE_DIRECTORY
  ansible-playbook playbook.yml \
    -e "node=$1" \
    -t ckb_restart
}



function ansible_wait_ckb_benchmark() {
    ansible_config
    cd $ANSIBLE_DIRECTORY
    ansible-playbook playbook.yml -e 'hostname=bastions' -e 'node=bastions' -t ckb_benchmark_install
    ansible-playbook playbook.yml -e 'hostname=bastions' -e 'node=bastions ckb_benchmark_url=http://172.31.23.160:8020' -t ckb_benchmark_miner_start
    ansible-playbook playbook.yml -e 'hostname=bastions' -e 'node=bastions ckb_bench_tps=2000 ckb_bench_time_ms=1000000 ckb_benchmark_n_users=11' -t ckb_benchmark_with_tps
    ansible-playbook playbook.yml -e 'hostname=bastions' -e 'node=bastions ckb_benchmark_url=http://172.31.23.160:8020 ckb_mining_interval_ms=10 ckb_bench_min_tx_size=2 ckb_n_blocks=340' -t ckb_bench_miner &
    ansible-playbook playbook.yml -e 'hostname=bastions' -e 'node=bastions ckb_bench_tps=1 ckb_bench_time_ms=1000000 ckb_benchmark_n_users=11 ckb_bench_concurrent_requests=1' -t ckb_benchmark_with_tps
    ansible-playbook playbook.yml -e 'hostname=bastions' -e 'node=bastions' -e "ckb_bench_log_file=demo.tar.gz" -t process_result
}

function ansible_process_result() {
      ansible_config
      cd $ANSIBLE_DIRECTORY
      ansible-playbook playbook.yml -e 'hostname=bastions' -e 'node=bastions' -e "ckb_bench_log_file=demo.tar.gz" -t process_result
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
      ansible_deploy_download_ckb node1 "http://172.31.45.113:8000/data.$2.tar.gz" &
      ansible_deploy_download_ckb node2 "http://172.31.45.113:8000/data.$2.tar.gz" &
      ansible_deploy_download_ckb node3 "http://172.31.45.113:8000/data.$2.tar.gz" &
      wait
      echo " deploy successful"
      sleep 20
#      wait node start
      echo "link nodes "
      link_all_nodes
      warm_up_and_sync_nodes
      echo "nodes are synchronized; ready to start bench"
      ;;
    "setup")
      job_setup
      ;;
    "deploy_ckb")
      ansible_deploy_download_ckb node1 "http://172.31.45.113:8000/data.$2.tar.gz" &
      ansible_deploy_download_ckb node2 "http://172.31.45.113:8000/data.$2.tar.gz" &
      ansible_deploy_download_ckb node3 "http://172.31.45.113:8000/data.$2.tar.gz" &
      wait
      echo "deploy successful"
      link_all_nodes
      warm_up_and_sync_nodes
      echo "link and synchronization successful"
      ;;
    "run_ckb")
      ansible_run_ckb node1
      ansible_run_ckb node2
      ansible_run_ckb node3
      ;;
   "clean_ckb_env")
      clean_ckb_env node1 &
      clean_ckb_env node2 &
      clean_ckb_env node3 &
      wait
      echo "clean ckb env"
      ;;
    "clean_ckb_bench")
      clean_ckb_bench_env
      ;;
   "clean_job")
      clean_ckb_env node1 &
      clean_ckb_env node2 &
      clean_ckb_env node3 &
      clean_ckb_bench_env &
      wait
      echo "clean finished"
      ;;
    "add_node")
      link_all_nodes
      warm_up_and_sync_nodes
      ;;
    "bench")
      wait_for_existing_node_sync
      ansible_wait_ckb_benchmark
      ;;
    "get_log")
      ansible_process_result
      ;;
    "restart_ckb")
          current_dir=`pwd`
          table_file="${current_dir}/restart_cost_time.md"
          output_file="${current_dir}/restart_cost_time.output"
          table_content="\n\n| block tip number | wait_restart_rpc_cost_time |\n| ----| --- |"

          # Ensure the files are created or cleared
          echo -e "$table_content" > "$table_file"
          elapsed_time=`ansible_ckb_restart node2 | grep -o 'Wait For CKB RPC Service Launched Time: [0-9]\+ seconds' | awk '{print $(NF-1)}'`
          echo -e "| $2 | ${elapsed_time}s|" >> "$output_file"

          cat "$output_file" >> "$table_file"
          echo -e "\n\n<hr/>\n\n[Explanation of Terms](https://github.com/gpBlockchain/ckb-integration-test/tree/ckb-bench-with-data/ckb-bench-with-data#interpretation-of-test-results)" >> "$table_file"
          echo "finished"
      ;;
    esac
}

main $*
