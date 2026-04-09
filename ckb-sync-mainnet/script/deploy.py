#!/usr/bin/env python3
"""
CKB Sync Mainnet deployment script - replaces Ansible playbooks.

Usage:
    python deploy.py setup
    python deploy.py run
    python deploy.py ansible     (deploy + sync + report)
    python deploy.py report
    python deploy.py clean
    python deploy.py clean_ckb_env
    python deploy.py insert_report_to_postgres
"""

import argparse
import logging
import os
import sys
import json
import re
import urllib.request
import subprocess
from datetime import datetime

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "../.."))

from ckb_deploy_tool import Inventory, SSHClient, CkbNode
from ckb_deploy_tool.ckb_node import load_node_vars
from ckb_deploy_tool.ckb_rpc import CkbRpcClient

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(levelname)s %(message)s")
logger = logging.getLogger(__name__)

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
PROJECT_DIR = os.path.dirname(SCRIPT_DIR)
JOB_ID = os.environ.get("JOB_ID", f"sync-mainnet-{datetime.now().strftime('%Y-%m-%d')}-in-10h")
DOWNLOAD_CKB_VERSION = "latest"


def get_paths():
    job_dir = os.path.join(PROJECT_DIR, "job", JOB_ID)
    return {
        "job_dir": job_dir,
        "ansible_dir": os.path.join(job_dir, "ansible"),
        "inventory": os.path.join(job_dir, "ansible", "inventory.yml"),
        "vars_dir": os.path.join(job_dir, "ansible", "vars"),
        "ssh_key": os.path.join(job_dir, "ssh", "id"),
        "terraform_dir": os.path.join(job_dir, "terraform"),
    }


def ssh_gen_key(paths):
    if os.path.exists(paths["ssh_key"]):
        logger.info(f"{paths['ssh_key']} already exists, reusing")
        return
    os.makedirs(os.path.dirname(paths["ssh_key"]), exist_ok=True)
    os.system(f"ssh-keygen -t rsa -N '' -f {paths['ssh_key']}")


def setup(paths):
    os.makedirs(paths["job_dir"], exist_ok=True)
    import shutil
    for subdir in ["ansible", "terraform"]:
        src = os.path.join(PROJECT_DIR, subdir)
        dst = os.path.join(paths["job_dir"], subdir)
        if os.path.exists(src):
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


def get_target_tip_number():
    payload = json.dumps({
        "jsonrpc": "2.0", "id": 42,
        "method": "get_tip_block_number", "params": [],
    }).encode()
    req = urllib.request.Request(
        "http://mainnet.ckb.dev:80",
        data=payload,
        headers={"Content-Type": "application/json"},
    )
    resp = urllib.request.urlopen(req)
    data = json.loads(resp.read())
    return int(data["result"], 16)


def deploy_all_nodes(paths, ckb_url=None, local_source=None):
    inv = Inventory(paths["inventory"])
    vars_data = load_node_vars(paths["vars_dir"], "all")

    for host in inv.all_hosts():
        with get_ssh(host, paths) as ssh:
            node = CkbNode(ssh, host, vars_data)
            if local_source:
                ssh.upload(local_source, f"/tmp/ckb_local.tar.gz")
                ssh.run("cd /tmp && tar xzf ckb_local.tar.gz", sudo=True)
                ssh.run(f"cp /tmp/ckb {node.workspace}/ckb && chmod +x {node.workspace}/ckb", sudo=True)
            else:
                url = ckb_url or get_latest_ckb_url()
                node.install(url)
            node.configure()


def wait_sync(paths, target_number):
    inv = Inventory(paths["inventory"])
    vars_data = load_node_vars(paths["vars_dir"], "all")

    for host in inv.all_hosts():
        with get_ssh(host, paths) as ssh:
            node = CkbNode(ssh, host, vars_data)
            node.restart()

            pattern = f"ChainService INFO ckb_chain::chain  block: {target_number},"
            ssh.wait_for_file_pattern(
                f"{node.data_dir}/logs/run.log",
                pattern,
                timeout=72000,
            )
            logger.info(f"Node {host.name} reached block {target_number}")


def ckb_replay(paths):
    inv = Inventory(paths["inventory"])
    vars_data = load_node_vars(paths["vars_dir"], "all")

    for host in inv.all_hosts():
        with get_ssh(host, paths) as ssh:
            node = CkbNode(ssh, host, vars_data)
            node.stop()
            ssh.run(
                f"{node.workspace}/ckb -C {node.workspace} replay --tmp-target='/tmp' --profile >> {node.data_dir}/logs/run.log",
                sudo=True,
            )
            node.start()
            tps_line = ssh.run(
                f"grep -m 1 'End profiling, duration:.*txs.*tps:[0-9]\\+' {node.data_dir}/logs/run.log | awk -F: '{{print $4}}'",
                check=False,
            )
            logger.info(f"Replay TPS for {host.name}: {tps_line.strip()}")


def markdown_report(paths):
    inv = Inventory(paths["inventory"])
    vars_data = load_node_vars(paths["vars_dir"], "all")
    target = get_target_tip_number()

    lines = []
    lines.append("**Sync-Mainnet Report**:")
    lines.append("| Version | Time(s) | Speed | Tip | Hostname | Network |")
    lines.append("| :--- | :--- | :--- | :--- | :--- | :--- |")

    for host in inv.all_hosts():
        rpc_port = vars_data.get("ckb_rpc_listen_address", "0.0.0.0:8114").split(":")[-1]
        rpc = CkbRpcClient(f"http://{host.ansible_host}:{rpc_port}")

        try:
            node_info = rpc.local_node_info()
            consensus = rpc.get_consensus()
            version = node_info.get("version", "unknown")
            network = consensus.get("id", "unknown")
        except Exception as e:
            logger.warning(f"RPC failed for {host.name}: {e}")
            version = "unknown"
            network = "unknown"

        with get_ssh(host, paths) as ssh:
            data_dir = vars_data.get("ckb_data_dir", "/var/lib/ckb/data")
            if "{{ ckb_workspace }}" in data_dir:
                workspace = vars_data.get("ckb_workspace", "/var/lib/ckb")
                data_dir = data_dir.replace("{{ ckb_workspace }}", workspace)

            start_line = ssh.run(f"head -n 1 {data_dir}/logs/run.log", check=False).strip()
            target_line = ssh.run(
                f"grep -m 1 'block: {target},' {data_dir}/logs/run.log || true",
                check=False,
            ).strip()

            entry = f"| {version} | N/A | N/A | {target} | {host.name} | {network} | 0 |"
            lines.append(entry)

            with open(f"{host.name}.brief.md", "w") as f:
                f.write(entry + "\n")

    report = "\n".join(lines)
    logger.info(report)
    return report


def clean_ckb_env(paths):
    inv = Inventory(paths["inventory"])
    vars_data = load_node_vars(paths["vars_dir"], "all")
    for host in inv.all_hosts():
        with get_ssh(host, paths) as ssh:
            node = CkbNode(ssh, host, vars_data)
            node.clean()


def main():
    parser = argparse.ArgumentParser(description="CKB Sync Mainnet deployment")
    parser.add_argument("command", choices=[
        "setup", "run", "ansible", "report", "clean", "clean_ckb_env",
        "clean_job", "insert_report_to_postgres", "build",
    ])
    args = parser.parse_args()

    paths = get_paths()

    if args.command == "setup":
        setup(paths)
    elif args.command == "run":
        setup(paths)
        deploy_all_nodes(paths)
        target = get_target_tip_number()
        wait_sync(paths, target)
        report = markdown_report(paths)
    elif args.command == "ansible":
        deploy_all_nodes(paths)
        target = get_target_tip_number()
        wait_sync(paths, target)
        markdown_report(paths)
    elif args.command == "report":
        markdown_report(paths)
    elif args.command == "clean":
        import shutil
        if os.path.exists(paths["job_dir"]):
            shutil.rmtree(paths["job_dir"])
    elif args.command == "clean_ckb_env":
        clean_ckb_env(paths)
    elif args.command == "clean_job":
        import shutil
        if os.path.exists(paths["job_dir"]):
            shutil.rmtree(paths["job_dir"])
    elif args.command == "insert_report_to_postgres":
        logger.info("insert_report_to_postgres - use the existing shell script for postgres operations")


if __name__ == "__main__":
    main()
