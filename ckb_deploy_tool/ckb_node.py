"""CKB node operations via SSH - replaces ansible-ckb role."""

import json
import logging
import os
import re
import yaml
from typing import Optional
from .ssh_client import SSHClient
from .ckb_rpc import CkbRpcClient
from .inventory import Host

logger = logging.getLogger(__name__)


class CkbNode:
    """Manages a CKB node on a remote host, replacing the ansible-ckb role."""

    def __init__(self, ssh: SSHClient, host: Host, vars_data: dict):
        self.ssh = ssh
        self.host = host
        self.vars = vars_data
        self.workspace = vars_data.get("ckb_workspace", "/var/lib/ckb")
        self.service = vars_data.get("ckb_service", "ckb")
        self.data_dir = vars_data.get("ckb_data_dir", f"{self.workspace}/data")
        self.rpc_address = vars_data.get("ckb_rpc_listen_address", "0.0.0.0:8114")
        self.network_listen = vars_data.get(
            "ckb_network_listen_addresses", ["/ip4/0.0.0.0/tcp/8114"]
        )
        self.download_tmp_dir = vars_data.get("ckb_download_tmp_dir", "/tmp")

    @property
    def rpc_port(self) -> str:
        return self.rpc_address.split(":")[-1]

    @property
    def p2p_port(self) -> str:
        addr = self.network_listen[0] if self.network_listen else "/ip4/0.0.0.0/tcp/8114"
        m = re.search(r"(\d+)$", addr)
        return m.group(1) if m else "8114"

    @property
    def rpc_url(self) -> str:
        return f"http://{self.host.p2p_ip}:{self.rpc_port}"

    @property
    def rpc_client(self) -> CkbRpcClient:
        return CkbRpcClient(self.rpc_url)

    def install(self, download_url: str, download_tmp_dir: Optional[str] = None):
        """Download and install CKB binary (replaces ckb_install tag)."""
        tmp_dir = download_tmp_dir or self.download_tmp_dir
        logger.info(f"Installing CKB on {self.host.name} from {download_url}")
        self.ssh.run(f"mkdir -p {self.workspace}", sudo=True)
        self.ssh.run(
            f"cd {tmp_dir} && "
            f"curl -L -o ckb.tar.gz '{download_url}' && "
            f"tar xzf ckb.tar.gz && "
            f"cp -f ckb_*/ckb {self.workspace}/ckb && "
            f"chmod +x {self.workspace}/ckb && "
            f"rm -rf ckb.tar.gz ckb_*",
            sudo=True,
        )
        self.ssh.run(f"{self.workspace}/ckb --version", sudo=True)

    def data_install(self, data_url: str):
        """Download and install CKB data (replaces ckb_data_install tag)."""
        logger.info(f"Installing CKB data on {self.host.name} from {data_url}")
        self.ssh.run(f"mkdir -p {self.data_dir}", sudo=True)
        self.ssh.run(
            f"cd {self.download_tmp_dir} && "
            f"curl -L -o data.tar.gz '{data_url}' && "
            f"tar xzf data.tar.gz -C {self.data_dir} && "
            f"rm -f data.tar.gz",
            sudo=True,
        )

    def configure(self, spec_file: Optional[str] = None, extra_vars: Optional[dict] = None):
        """Configure CKB node (replaces ckb_configure tag).

        Strategy: let ``ckb init`` generate a complete default ckb.toml
        (with all required fields like max_peers, etc.), then read it back
        and patch only the fields we need to change.  This avoids the
        ``missing field`` errors that occur when generating ckb.toml from
        scratch.
        """
        logger.info(f"Configuring CKB on {self.host.name}")
        merged = {**self.vars}
        if extra_vars:
            merged.update(extra_vars)

        chain_spec = merged.get("ckb_chain_spec_file", "")
        chain_spec_bundled = merged.get("ckb_chain_spec_bundled", "")
        bootnodes = merged.get("ckb_network_bootnodes", [])
        block_assembler = merged.get("ckb_block_assembler", {})
        prometheus = merged.get("ckb_prometheus", {})

        self.ssh.run(f"mkdir -p {self.workspace}", sudo=True)
        self.ssh.run(f"mkdir -p {self.data_dir}/logs", sudo=True)

        self.ssh.run(
            f"cd {self.workspace} && {self.workspace}/ckb init --chain dev --force",
            sudo=True,
        )

        if spec_file:
            self.ssh.run(f"mkdir -p {self.workspace}/specs", sudo=True)
            self.ssh.upload(spec_file, "/tmp/_ckb_spec.toml")
            self.ssh.run(f"cp /tmp/_ckb_spec.toml {self.workspace}/specs/{chain_spec}", sudo=True)

        default_toml = self.ssh.run(f"cat {self.workspace}/ckb.toml", sudo=True)
        patched = _patch_ckb_toml(default_toml, merged)
        self.ssh.write_file(f"{self.workspace}/ckb.toml", patched, sudo=True)

        default_miner = self.ssh.run(f"cat {self.workspace}/ckb-miner.toml", sudo=True)
        patched_miner = _patch_miner_toml(default_miner, self.rpc_port, merged)
        self.ssh.write_file(f"{self.workspace}/ckb-miner.toml", patched_miner, sudo=True)

        systemd_unit = (
            "[Unit]\n"
            f"Description=CKB Node ({self.service})\n"
            "After=network.target\n"
            "\n"
            "[Service]\n"
            "Type=simple\n"
            f"ExecStart={self.workspace}/ckb -C {self.workspace} run\n"
            "Restart=on-failure\n"
            "User=root\n"
            f"WorkingDirectory={self.workspace}\n"
            "\n"
            "[Install]\n"
            "WantedBy=multi-user.target\n"
        )
        self.ssh.write_file(f"/etc/systemd/system/{self.service}.service", systemd_unit, sudo=True)

        miner_service_name = f"{self.service}_miner"
        miner_unit = (
            "[Unit]\n"
            f"Description=CKB Miner ({miner_service_name})\n"
            f"After={self.service}.service\n"
            "\n"
            "[Service]\n"
            "Type=simple\n"
            f"ExecStart={self.workspace}/ckb -C {self.workspace} miner\n"
            "Restart=on-failure\n"
            "User=root\n"
            f"WorkingDirectory={self.workspace}\n"
            "\n"
            "[Install]\n"
            "WantedBy=multi-user.target\n"
        )
        self.ssh.write_file(f"/etc/systemd/system/{miner_service_name}.service", miner_unit, sudo=True)

        self.ssh.run("systemctl daemon-reload", sudo=True)

    def start(self):
        logger.info(f"Starting CKB on {self.host.name}")
        self.ssh.service_action(self.service, "start")

    def stop(self):
        logger.info(f"Stopping CKB on {self.host.name}")
        self.ssh.run(f"systemctl stop {self.service} || true", sudo=True)

    def restart(self):
        logger.info(f"Restarting CKB on {self.host.name}")
        self.ssh.service_action(self.service, "restart")

    def miner_start(self):
        miner_service = f"{self.service}_miner"
        logger.info(f"Starting CKB miner on {self.host.name}")
        self.ssh.service_action(miner_service, "start")

    def miner_stop(self):
        miner_service = f"{self.service}_miner"
        logger.info(f"Stopping CKB miner on {self.host.name}")
        self.ssh.run(f"systemctl stop {miner_service} || true", sudo=True)

    def clean(self):
        logger.info(f"Cleaning CKB on {self.host.name}")
        self.stop()
        self.miner_stop()
        self.ssh.run(f"rm -rf {self.data_dir}", sudo=True)
        self.ssh.run(f"rm -rf {self.workspace}", sudo=True)

    def status(self):
        out = self.ssh.run(f"systemctl status {self.service} || true", sudo=True)
        return out


def _patch_ckb_toml(default_toml: str, merged: dict) -> str:
    """Patch the default ckb.toml generated by ``ckb init`` with our overrides.

    Works by reading the existing TOML line-by-line and replacing specific
    key = value lines.  This preserves all default fields (max_peers, etc.)
    that CKB requires but we don't explicitly set.
    """
    import re as _re

    data_dir = merged.get("ckb_data_dir", "/var/lib/ckb/data")
    rpc_listen = merged.get("ckb_rpc_listen_address", "0.0.0.0:8114")
    network_listen = merged.get("ckb_network_listen_addresses", ["/ip4/0.0.0.0/tcp/8114"])
    bootnodes = merged.get("ckb_network_bootnodes", [])
    chain_spec = merged.get("ckb_chain_spec_file", "")
    chain_spec_bundled = merged.get("ckb_chain_spec_bundled", "")
    block_assembler = merged.get("ckb_block_assembler", {})
    prometheus = merged.get("ckb_prometheus", {})

    lines = default_toml.splitlines()
    result = []
    current_section = ""

    for line in lines:
        stripped = line.strip()

        section_match = _re.match(r'^\[([^\]]+)\]', stripped)
        if section_match:
            current_section = section_match.group(1)

        if stripped.startswith("data_dir"):
            result.append(f'data_dir = "{data_dir}"')
            continue

        if current_section == "chain" and stripped.startswith("spec"):
            if chain_spec:
                result.append(f'spec = {{ file = "specs/{chain_spec}" }}')
            elif chain_spec_bundled:
                result.append(f'spec = {{ bundled = "{chain_spec_bundled}" }}')
            else:
                result.append(line)
            continue

        if current_section == "network" and stripped.startswith("listen_addresses"):
            result.append(f"listen_addresses = {json.dumps(network_listen)}")
            continue

        if current_section == "network" and stripped.startswith("bootnodes") and bootnodes:
            result.append(f"bootnodes = {json.dumps(bootnodes)}")
            continue

        if current_section == "rpc" and stripped.startswith("listen_address"):
            result.append(f'listen_address = "{rpc_listen}"')
            continue

        if current_section == "rpc" and stripped.startswith("modules"):
            result.append('modules = ["Net", "Pool", "Miner", "Chain", "Stats", "Subscription", "Experiment", "Debug", "Indexer"]')
            continue

        if current_section == "logger" and stripped.startswith("log_to_file"):
            result.append("log_to_file = true")
            continue
        if current_section == "logger" and stripped.startswith("log_to_stdout"):
            result.append("log_to_stdout = false")
            continue
        if current_section == "logger" and stripped.startswith("log_dir"):
            result.append(f'log_dir = "{data_dir}/logs"')
            continue

        result.append(line)

    content = "\n".join(result) + "\n"

    if block_assembler and block_assembler.get("code_hash"):
        content = _remove_toml_section(content, "block_assembler")
        content += (
            "\n[block_assembler]\n"
            f'code_hash = "{block_assembler["code_hash"]}"\n'
            f'args = "{block_assembler.get("args", "")}"\n'
            f'hash_type = "{block_assembler.get("hash_type", "type")}"\n'
            f'message = "{block_assembler.get("message", "0x")}"\n'
        )

    if prometheus and prometheus.get("listen_address"):
        content = _remove_toml_section(content, "metrics.exporter.prometheus")
        content += (
            "\n[metrics.exporter.prometheus]\n"
            f'listen_address = "{prometheus["listen_address"]}"\n'
        )

    return content


def _remove_toml_section(content: str, section_name: str) -> str:
    """Remove a TOML section and all its content (including commented-out versions).

    Removes everything from the line containing ``[section_name]`` (commented
    or not) up to (but not including) the next ``[other_section]`` header.
    Also removes stray bare keys (code_hash, args, etc.) that might have been
    placed after a commented-out section header.
    """
    import re as _re
    escaped = _re.escape(section_name)
    pattern = (
        r'(?m)'
        r'^[# \t]*\[' + escaped + r'\][^\n]*\n'
        r'(?:(?!\[)[^\n]*\n)*'
    )
    return _re.sub(pattern, '', content)


def _patch_miner_toml(default_miner: str, rpc_port: str, merged: dict) -> str:
    """Patch the default ckb-miner.toml with our overrides."""
    lines = default_miner.splitlines()
    result = []
    for line in lines:
        stripped = line.strip()
        if stripped.startswith("rpc_url"):
            result.append(f'rpc_url = "http://127.0.0.1:{rpc_port}"')
            continue
        poll = merged.get("ckb_miner_poll_interval")
        if poll and stripped.startswith("poll_interval"):
            result.append(f"poll_interval = {poll}")
            continue
        result.append(line)
    return "\n".join(result) + "\n"


def load_node_vars(vars_dir: str, node_name: str) -> dict:
    """Load variables from a YAML vars file, resolving Ansible-style template references."""
    import os
    path = os.path.join(vars_dir, f"{node_name}.yml")
    if not os.path.exists(path):
        path = os.path.join(vars_dir, "all.yml")
    if not os.path.exists(path):
        return {}
    with open(path) as f:
        data = yaml.safe_load(f) or {}
    workspace = data.get("ckb_workspace", "/var/lib/ckb")
    for k, v in data.items():
        if isinstance(v, str) and "{{ ckb_workspace }}" in v:
            data[k] = v.replace("{{ ckb_workspace }}", workspace)
    return data


def add_node(
    node1_host: Host,
    node2_host: Host,
    node1_vars: dict,
    node2_vars: dict,
):
    """Connect node2 to node1 via add_node RPC (replaces ckb_add_node.yml).

    Automatically waits for both nodes' RPC services to be ready before
    issuing any RPC calls, avoiding ConnectionRefusedError when nodes
    have just been (re)started.
    """
    n1_rpc_port = node1_vars.get("ckb_rpc_listen_address", "0.0.0.0:8114").split(":")[-1]
    n2_rpc_port = node2_vars.get("ckb_rpc_listen_address", "0.0.0.0:8114").split(":")[-1]
    n1_listen = node1_vars.get("ckb_network_listen_addresses", ["/ip4/0.0.0.0/tcp/8114"])
    n1_p2p_port = re.search(r"(\d+)$", n1_listen[0]).group(1) if n1_listen else "8114"

    node1_ip = node1_host.p2p_ip
    node2_ip = node2_host.p2p_ip

    rpc1 = CkbRpcClient(f"http://{node1_ip}:{n1_rpc_port}")
    rpc2 = CkbRpcClient(f"http://{node2_ip}:{n2_rpc_port}")

    logger.info(f"Waiting for RPC on {node1_host.name} ({rpc1.url}) ...")
    rpc1.wait_for_rpc_ready()
    logger.info(f"Waiting for RPC on {node2_host.name} ({rpc2.url}) ...")
    rpc2.wait_for_rpc_ready()

    info = rpc1.local_node_info()
    node1_id = info["node_id"]
    logger.info(f"Node1 ({node1_host.name}) node_id: {node1_id}")

    address = f"/ip4/{node1_ip}/tcp/{n1_p2p_port}"
    logger.info(f"Adding node1 to node2: peer_id={node1_id}, address={address}")
    rpc2.add_node(node1_id, address)


def set_network_active(host: Host, node_vars: dict, active: bool):
    """Set network active state via RPC (replaces ckb_set_network_active.yml)."""
    rpc_port = node_vars.get("ckb_rpc_listen_address", "0.0.0.0:8114").split(":")[-1]
    rpc = CkbRpcClient(f"http://{host.ansible_host}:{rpc_port}")
    rpc.wait_for_rpc_ready()
    rpc.set_network_active(active)


def wait_pending_load(host: Host, node_vars: dict, pending_target: int):
    """Wait until pending tx count >= target (replaces ckb_wait_pending_load.yml)."""
    rpc_port = node_vars.get("ckb_rpc_listen_address", "0.0.0.0:8114").split(":")[-1]
    rpc = CkbRpcClient(f"http://{host.ansible_host}:{rpc_port}")
    rpc.wait_for_rpc_ready()
    rpc.wait_pending_ge(pending_target)


def wait_pending_commit(host: Host, node_vars: dict, pending_target: int):
    """Wait until pending tx count == target (replaces ckb_wait_pengding_tx_commit.yml)."""
    rpc_port = node_vars.get("ckb_rpc_listen_address", "0.0.0.0:8114").split(":")[-1]
    rpc = CkbRpcClient(f"http://{host.ansible_host}:{rpc_port}")
    rpc.wait_for_rpc_ready()
    rpc.wait_pending_eq(pending_target)
