#!/usr/bin/env python3
"""
CKB Sync Pending TX deployment script - replaces Ansible playbooks.

Usage:
    python deploy.py setup
    python deploy.py run
    python deploy.py deploy_ckb
    python deploy.py add_node
    python deploy.py miner
    python deploy.py ckb_pending
    python deploy.py clean_ckb_env
"""

import argparse
import logging
import os
import sys
import json
import urllib.request

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "../.."))

from ckb_deploy_tool import Inventory, SSHClient, CkbNode, CkbBenchmark
from ckb_deploy_tool.ckb_node import load_node_vars, add_node, wait_pending_load
from ckb_deploy_tool.ckb_rpc import CkbRpcClient

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(levelname)s %(message)s")
logger = logging.getLogger(__name__)

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
PROJECT_DIR = os.path.dirname(SCRIPT_DIR)
JOB_ID = os.environ.get("JOB_ID", f"benchmark-{__import__('datetime').date.today()}-in-10h")


def get_paths():
    job_dir = os.path.join(PROJECT_DIR, "job", JOB_ID)
    return {
        "job_dir": job_dir,
        "ansible_dir": os.path.join(job_dir, "ansible"),
        "inventory": os.path.join(job_dir, "ansible", "inventory.yml"),
        "vars_dir": os.path.join(job_dir, "ansible", "vars"),
        "ssh_key": os.path.join(job_dir, "ssh", "id"),
        "spec_file": os.path.join(job_dir, "ansible", "files", "sync-spec.toml"),
    }


def ssh_gen_key(paths):
    ssh_dir = os.path.dirname(paths["ssh_key"])
    os.makedirs(ssh_dir, exist_ok=True)
    if os.path.exists(paths["ssh_key"]):
        logger.info(f"{paths['ssh_key']} already exists, reusing")
        return
    os.makedirs(os.path.dirname(paths["ssh_key"]), exist_ok=True)
    os.system(f"ssh-keygen -t rsa -N '' -f {paths['ssh_key']}")


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


def get_latest_ckb_url():
    resp = urllib.request.urlopen("https://api.github.com/repos/nervosnetwork/ckb/releases/latest")
    data = json.loads(resp.read())
    for asset in data["assets"]:
        url = asset["browser_download_url"]
        if "unknown-linux-gnu-portable" in url and not url.endswith(".asc"):
            return url
    raise RuntimeError("Cannot find CKB release URL")


def deploy_node(node_name, paths):
    inv = Inventory(paths["inventory"])
    host = inv.get(node_name)
    vars_data = load_node_vars(paths["vars_dir"], node_name)

    ckb_url = get_latest_ckb_url()
    with get_ssh(host, paths) as ssh:
        node = CkbNode(ssh, host, vars_data)
        node.install(ckb_url)
        spec = paths.get("spec_file")
        if spec and os.path.exists(spec):
            node.configure(spec_file=spec)
        else:
            node.configure()
        node.start()


def link_p2p(n1_name, n2_name, paths):
    inv = Inventory(paths["inventory"])
    h1 = inv.get(n1_name)
    h2 = inv.get(n2_name)
    v1 = load_node_vars(paths["vars_dir"], n1_name)
    v2 = load_node_vars(paths["vars_dir"], n2_name)
    add_node(h1, h2, v1, v2)


def start_miner(node_name, paths):
    inv = Inventory(paths["inventory"])
    host = inv.get(node_name)
    vars_data = load_node_vars(paths["vars_dir"], node_name)
    with get_ssh(host, paths) as ssh:
        node = CkbNode(ssh, host, vars_data)
        node.miner_start()


def ckb_wait_pending(node_name, pending_count, paths):
    inv = Inventory(paths["inventory"])
    host = inv.get(node_name)
    vars_data = load_node_vars(paths["vars_dir"], node_name)
    wait_pending_load(host, vars_data, int(pending_count))


def clean_node(node_name, paths):
    inv = Inventory(paths["inventory"])
    host = inv.get(node_name)
    vars_data = load_node_vars(paths["vars_dir"], node_name)
    with get_ssh(host, paths) as ssh:
        node = CkbNode(ssh, host, vars_data)
        node.clean()


def cmd_run(paths):
    for n in ["node1", "node2", "node3", "node4"]:
        deploy_node(n, paths)
    ckb_wait_pending("node4", 8000, paths)
    link_p2p("node1", "node2", paths)
    link_p2p("node1", "node3", paths)
    link_p2p("node2", "node4", paths)
    link_p2p("node3", "node4", paths)
    start_miner("node1", paths)


def cmd_deploy_ckb(paths):
    for n in ["node1", "node2", "node3", "node4"]:
        deploy_node(n, paths)


def cmd_add_node(paths):
    link_p2p("node1", "node2", paths)
    link_p2p("node1", "node3", paths)
    link_p2p("node2", "node4", paths)
    link_p2p("node3", "node4", paths)


def cmd_clean_ckb_env(paths):
    for n in ["node1", "node2", "node3", "node4"]:
        clean_node(n, paths)


def main():
    parser = argparse.ArgumentParser(description="CKB Sync Pending TX deployment")
    parser.add_argument("command", choices=[
        "setup", "run", "deploy_ckb", "run_ckb", "miner",
        "clean_ckb_env", "add_node", "ckb_pending",
    ])
    parser.add_argument("extra", nargs="?", default=None)
    args = parser.parse_args()

    paths = get_paths()

    if args.command == "setup":
        setup(paths)
    elif args.command == "run":
        cmd_run(paths)
    elif args.command == "deploy_ckb":
        cmd_deploy_ckb(paths)
    elif args.command == "run_ckb":
        for n in ["node1", "node2", "node3", "node4"]:
            inv = Inventory(paths["inventory"])
            host = inv.get(n)
            vars_data = load_node_vars(paths["vars_dir"], n)
            with get_ssh(host, paths) as ssh:
                CkbNode(ssh, host, vars_data).start()
    elif args.command == "miner":
        start_miner("node1", paths)
    elif args.command == "clean_ckb_env":
        cmd_clean_ckb_env(paths)
    elif args.command == "add_node":
        cmd_add_node(paths)
    elif args.command == "ckb_pending":
        ckb_wait_pending("node4", args.extra or "100", paths)


if __name__ == "__main__":
    main()
