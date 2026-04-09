#!/usr/bin/env python3
"""
CKB Bench Server deployment script - replaces Ansible playbooks.

Usage:
    python deploy.py setup
    python deploy.py run
    python deploy.py deploy
    python deploy.py restart
    python deploy.py clean
"""

import argparse
import logging
import os
import sys
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
        "spec_file": os.path.join(job_dir, "ansible", "files", "benchmark-spec.toml"),
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
    """Setup job directory and SSH keys."""
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


def deploy_node(node_name, paths, extra_vars=None):
    """Deploy and start a CKB node."""
    inv = Inventory(paths["inventory"])
    host = inv.get(node_name)
    vars_data = load_node_vars(paths["vars_dir"], node_name)
    if extra_vars:
        vars_data.update(extra_vars)

    with get_ssh(host, paths) as ssh:
        node = CkbNode(ssh, host, vars_data)
        node.install(get_ckb_remote_url(), download_tmp_dir="/tmp")
        spec = paths.get("spec_file")
        if spec and os.path.exists(spec):
            node.configure(spec_file=spec, extra_vars=extra_vars)
        else:
            node.configure(extra_vars=extra_vars)
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

        rpc_urls = bastions_vars.get("ckb_urls", ["http://172.31.28.209:8021"])
        owner_key = bastions_vars.get("ckb_benchmark_owner_privkey", "")
        n_users = bastions_vars.get("ckb_benchmark_n_users", 100)

        bench.prepare(rpc_urls, n_users=n_users, owner_privkey=owner_key)
        bench.stop()

        for n_inout in [1, 2, 5, 10]:
            bench.bench_with_tps(
                rpc_urls=rpc_urls,
                tps=bastions_vars.get("ckb_bench_tps", 2000),
                n_users=n_users,
                n_inout=n_inout,
                bench_time_ms=bastions_vars.get("ckb_bench_time_ms", 1800000),
                concurrent_requests=bastions_vars.get("ckb_bench_concurrent_requests", 8),
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


def restart_node(node_name, paths):
    inv = Inventory(paths["inventory"])
    host = inv.get(node_name)
    vars_data = load_node_vars(paths["vars_dir"], node_name)
    with get_ssh(host, paths) as ssh:
        node = CkbNode(ssh, host, vars_data)
        node.restart()


def clean_bench(paths):
    inv = Inventory(paths["inventory"])
    host = inv.get("bastions")
    vars_data = load_node_vars(paths["vars_dir"], "bastions")
    with get_ssh(host, paths) as ssh:
        bench = CkbBenchmark(ssh, host, vars_data)
        bench.clean()


def cmd_run(paths):
    deploy_node("node2", paths)
    deploy_node("node1", paths)
    deploy_node("node3", paths)
    miner_start("node2", paths)

    for n1, n2 in [
        ("node2", "node1"), ("node2", "node3"),
        ("node1", "node2"), ("node1", "node3"),
        ("node3", "node1"), ("node3", "node2"),
    ]:
        link_p2p(n1, n2, paths)

    run_benchmark(paths)


def cmd_deploy(paths):
    deploy_node("node2", paths)
    deploy_node("node1", paths)
    deploy_node("node3", paths)
    miner_start("node2", paths)

    for n1, n2 in [
        ("node2", "node1"), ("node2", "node3"),
        ("node1", "node2"), ("node1", "node3"),
        ("node3", "node1"), ("node3", "node2"),
    ]:
        link_p2p(n1, n2, paths)


def cmd_restart(paths):
    with concurrent.futures.ThreadPoolExecutor() as executor:
        futures = [
            executor.submit(restart_node, n, paths)
            for n in ["node1", "node2", "node3"]
        ]
        concurrent.futures.wait(futures)
    logger.info("restart succ")


def cmd_clean(paths):
    with concurrent.futures.ThreadPoolExecutor() as executor:
        futures = [
            executor.submit(clean_node, n, paths)
            for n in ["node1", "node2", "node3"]
        ]
        futures.append(executor.submit(clean_bench, paths))
        concurrent.futures.wait(futures)
    logger.info("clean finished")


def main():
    parser = argparse.ArgumentParser(description="CKB Bench Server deployment")
    parser.add_argument("command", choices=["setup", "run", "deploy", "restart", "clean"])
    args = parser.parse_args()

    paths = get_paths()

    if args.command == "setup":
        setup(paths)
    elif args.command == "run":
        cmd_run(paths)
    elif args.command == "deploy":
        cmd_deploy(paths)
    elif args.command == "restart":
        cmd_restart(paths)
    elif args.command == "clean":
        cmd_clean(paths)


if __name__ == "__main__":
    main()
