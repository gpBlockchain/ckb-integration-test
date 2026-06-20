#!/usr/bin/env python3
"""
CKB Cell Query deployment script - replaces Ansible playbooks.

Usage:
    python deploy.py setup
    python deploy.py run <data_count>
    python deploy.py deploy_ckb <data_count>
    python deploy.py bench
    python deploy.py clean_job
    python deploy.py add_node
    python deploy.py restart_ckb <block_tip>
"""

import argparse
import logging
import os
import sys
import time
import concurrent.futures

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "../.."))

from ckb_deploy_tool import Inventory, SSHClient, CkbNode, CkbBenchmark, config_get
from ckb_deploy_tool.ckb_node import load_node_vars, add_node

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(levelname)s %(message)s")
logger = logging.getLogger(__name__)

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
PROJECT_DIR = os.path.dirname(SCRIPT_DIR)
JOB_ID = "benchmark-in-10h"


def get_paths():
    job_dir = os.path.join(PROJECT_DIR, "job", JOB_ID)
    return {
        "job_dir": job_dir,
        "ansible_dir": os.path.join(job_dir, "ansible"),
        "inventory": os.path.join(job_dir, "ansible", "inventory.yml"),
        "vars_dir": os.path.join(job_dir, "ansible", "vars"),
        "ssh_key": os.path.join(job_dir, "ssh", "id"),
        "spec_file": os.path.join(job_dir, "ansible", "files", "sync-spec.toml"),
        "logs_dir": os.path.join(job_dir, "ansible", "logs"),
    }


def ssh_gen_key(paths):
    ssh_dir = os.path.dirname(paths["ssh_key"])
    os.makedirs(ssh_dir, exist_ok=True)
    ssh_id = os.environ.get("SSH_ID", "")
    if ssh_id:
        with open(paths["ssh_key"], "w") as f:
            for line in ssh_id.split("@"):
                f.write(line + "\n")
        os.chmod(paths["ssh_key"], 0o600)


def setup(paths):
    os.makedirs(paths["job_dir"], exist_ok=True)
    import shutil
    src = os.path.join(PROJECT_DIR, "ansible")
    dst = paths["ansible_dir"]
    if os.path.exists(dst):
        shutil.rmtree(dst)
    shutil.copytree(src, dst)
    ssh_gen_key(paths)
    logger.info("Setup complete")


def get_ssh(host, paths):
    return SSHClient(
        host=host.ssh_host,
        user=host.ansible_user,
        private_key_path=paths["ssh_key"],
    )


def get_ckb_remote_url():
    return os.environ.get("CKB_REMOTE_URL", config_get("ckb.download_url"))


def deploy_node_with_data(node_name, data_url, paths):
    inv = Inventory(paths["inventory"])
    host = inv.get(node_name)
    vars_data = load_node_vars(paths["vars_dir"], node_name)

    with get_ssh(host, paths) as ssh:
        node = CkbNode(ssh, host, vars_data)
        node.install(get_ckb_remote_url(), download_tmp_dir="/tmp")
        node.data_install(data_url)
        spec = paths.get("spec_file")
        if spec and os.path.exists(spec):
            node.configure(spec_file=spec)
        else:
            node.configure()
        node.restart()


def miner_start(node_name, paths):
    inv = Inventory(paths["inventory"])
    host = inv.get(node_name)
    vars_data = load_node_vars(paths["vars_dir"], node_name)
    with get_ssh(host, paths) as ssh:
        node = CkbNode(ssh, host, vars_data)
        node.miner_start()


def link_p2p(n1_name, n2_name, paths):
    inv = Inventory(paths["inventory"])
    h1 = inv.get(n1_name)
    h2 = inv.get(n2_name)
    v1 = load_node_vars(paths["vars_dir"], n1_name)
    v2 = load_node_vars(paths["vars_dir"], n2_name)
    add_node(h1, h2, v1, v2)


def run_benchmark(paths):
    inv = Inventory(paths["inventory"])
    bastions_host = inv.get("bastions")
    bastions_vars = load_node_vars(paths["vars_dir"], "bastions")

    with get_ssh(bastions_host, paths) as ssh:
        bench = CkbBenchmark(ssh, bastions_host, bastions_vars)
        bench.install()

        rpc_urls = bastions_vars.get("ckb_urls", [])
        owner_key = bastions_vars.get("ckb_benchmark_owner_privkey", "")
        n_users = bastions_vars.get("ckb_benchmark_n_users", 10000)
        benchmark_url = bastions_vars.get("ckb_benchmark_url", rpc_urls[0] if rpc_urls else "")

        bench.miner_start(
            rpc_url="http://172.31.23.160:8020",
            mining_interval_ms=bastions_vars.get("ckb_mining_interval_ms", 8000),
        )

        bench.bench_with_tps(
            rpc_urls=rpc_urls,
            tps=2000,
            n_users=11,
            n_inout=1,
            bench_time_ms=1000000,
            concurrent_requests=bastions_vars.get("ckb_bench_concurrent_requests", 8),
            owner_privkey=owner_key,
        )

        bench.miner_start(
            rpc_url="http://172.31.23.160:8020",
            mining_interval_ms=10,
            min_tx_size=2,
            n_blocks=340,
        )

        bench.bench_with_tps(
            rpc_urls=rpc_urls,
            tps=1,
            n_users=11,
            n_inout=1,
            bench_time_ms=1000000,
            concurrent_requests=1,
            owner_privkey=owner_key,
        )

        os.makedirs(paths["logs_dir"], exist_ok=True)
        bench.collect_results(
            os.path.join(paths["logs_dir"], "demo.tar.gz"),
            log_file="data.tar.gz",
        )


def clean_node(node_name, paths):
    inv = Inventory(paths["inventory"])
    host = inv.get(node_name)
    vars_data = load_node_vars(paths["vars_dir"], node_name)
    with get_ssh(host, paths) as ssh:
        node = CkbNode(ssh, host, vars_data)
        node.clean()


def clean_bench(paths):
    inv = Inventory(paths["inventory"])
    host = inv.get("bastions")
    vars_data = load_node_vars(paths["vars_dir"], "bastions")
    with get_ssh(host, paths) as ssh:
        bench = CkbBenchmark(ssh, host, vars_data)
        bench.clean()


def restart_node(node_name, paths):
    inv = Inventory(paths["inventory"])
    host = inv.get(node_name)
    vars_data = load_node_vars(paths["vars_dir"], node_name)
    with get_ssh(host, paths) as ssh:
        node = CkbNode(ssh, host, vars_data)
        node.restart()


def do_link_all(paths):
    links = [
        ("node1", "node2"), ("node1", "node3"),
        ("node2", "node1"), ("node2", "node3"),
        ("node3", "node1"), ("node3", "node2"),
    ]
    for n1, n2 in links:
        link_p2p(n1, n2, paths)


def cmd_run(paths, data_count):
    data_url = f"http://172.31.45.113:8000/data.{data_count}.tar.gz"
    with concurrent.futures.ThreadPoolExecutor() as executor:
        futures = [
            executor.submit(deploy_node_with_data, n, data_url, paths)
            for n in ["node1", "node2", "node3"]
        ]
        concurrent.futures.wait(futures)
    logger.info("deploy successful")
    time.sleep(20)
    logger.info("link nodes")
    do_link_all(paths)
    logger.info("start bench")


def cmd_deploy_ckb(paths, data_count):
    data_url = f"http://172.31.45.113:8000/data.{data_count}.tar.gz"
    with concurrent.futures.ThreadPoolExecutor() as executor:
        futures = [
            executor.submit(deploy_node_with_data, n, data_url, paths)
            for n in ["node1", "node2", "node3"]
        ]
        concurrent.futures.wait(futures)
    logger.info("deploy successful")
    do_link_all(paths)
    logger.info("link successful")


def cmd_clean_job(paths):
    with concurrent.futures.ThreadPoolExecutor() as executor:
        futures = [executor.submit(clean_node, n, paths) for n in ["node1", "node2", "node3"]]
        futures.append(executor.submit(clean_bench, paths))
        concurrent.futures.wait(futures)
    logger.info("clean finished")


def cmd_restart_ckb(paths, block_tip):
    current_dir = os.getcwd()
    table_file = os.path.join(current_dir, "restart_cost_time.md")
    output_file = os.path.join(current_dir, "restart_cost_time.output")

    table_content = "\n\n| block tip number | wait_restart_rpc_cost_time |\n| ----| --- |"
    with open(table_file, "w") as f:
        f.write(table_content + "\n")

    restart_node("node2", paths)

    with open(output_file, "a") as f:
        f.write(f"| {block_tip} | N/A |\n")

    with open(output_file) as f:
        content = f.read()
    with open(table_file, "a") as f:
        f.write(content)
        f.write("\n\n<hr/>\n\n[Explanation of Terms](https://github.com/gpBlockchain/ckb-integration-test/tree/ckb-bench-with-data/ckb-bench-with-data#interpretation-of-test-results)\n")
    logger.info("finished")


def main():
    parser = argparse.ArgumentParser(description="CKB Cell Query deployment")
    parser.add_argument("command", choices=[
        "setup", "run", "deploy_ckb", "run_ckb", "clean_ckb_env",
        "clean_ckb_bench", "clean_job", "add_node", "bench", "get_log", "restart_ckb",
    ])
    parser.add_argument("extra", nargs="?", default=None)
    args = parser.parse_args()

    paths = get_paths()

    if args.command == "setup":
        setup(paths)
    elif args.command == "run":
        cmd_run(paths, args.extra or "1000w")
    elif args.command == "deploy_ckb":
        cmd_deploy_ckb(paths, args.extra or "1000w")
    elif args.command == "run_ckb":
        for n in ["node1", "node2", "node3"]:
            inv = Inventory(paths["inventory"])
            host = inv.get(n)
            vars_data = load_node_vars(paths["vars_dir"], n)
            with get_ssh(host, paths) as ssh:
                CkbNode(ssh, host, vars_data).start()
    elif args.command == "clean_ckb_env":
        with concurrent.futures.ThreadPoolExecutor() as executor:
            futures = [executor.submit(clean_node, n, paths) for n in ["node1", "node2", "node3"]]
            concurrent.futures.wait(futures)
        logger.info("clean ckb env")
    elif args.command == "clean_ckb_bench":
        clean_bench(paths)
    elif args.command == "clean_job":
        cmd_clean_job(paths)
    elif args.command == "add_node":
        do_link_all(paths)
    elif args.command == "bench":
        run_benchmark(paths)
    elif args.command == "get_log":
        inv = Inventory(paths["inventory"])
        host = inv.get("bastions")
        vars_data = load_node_vars(paths["vars_dir"], "bastions")
        with get_ssh(host, paths) as ssh:
            bench = CkbBenchmark(ssh, host, vars_data)
            os.makedirs(paths["logs_dir"], exist_ok=True)
            bench.collect_results(os.path.join(paths["logs_dir"], "demo.tar.gz"), log_file="data.tar.gz")
    elif args.command == "restart_ckb":
        cmd_restart_ckb(paths, args.extra or "0")


if __name__ == "__main__":
    main()
