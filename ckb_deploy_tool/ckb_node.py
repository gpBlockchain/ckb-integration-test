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

        Generates ckb.toml and sets up systemd service.
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

        self.ssh.run(f"mkdir -p {self.data_dir}/logs", sudo=True)

        init_cmd = f"cd {self.workspace} && {self.workspace}/ckb init --chain dev --force 2>/dev/null || true"
        self.ssh.run(init_cmd, sudo=True)

        if spec_file:
            self.ssh.upload(spec_file, f"/tmp/_ckb_spec.toml")
            self.ssh.run(f"cp /tmp/_ckb_spec.toml {self.workspace}/specs/{chain_spec}", sudo=True)

        rpc_listen = merged.get("ckb_rpc_listen_address", "0.0.0.0:8114")
        network_listen = merged.get("ckb_network_listen_addresses", ["/ip4/0.0.0.0/tcp/8114"])

        ckb_toml_parts = [f'data_dir = "{self.data_dir}"']

        if chain_spec:
            ckb_toml_parts.append(f'\n[chain]\nspec = {{ file = "specs/{chain_spec}" }}')
        elif chain_spec_bundled:
            ckb_toml_parts.append(f'\n[chain]\nspec = {{ bundled = "{chain_spec_bundled}" }}')

        ckb_toml_parts.append(f'\n[network]\nlisten_addresses = {json.dumps(network_listen)}')
        if bootnodes:
            ckb_toml_parts.append(f'bootnodes = {json.dumps(bootnodes)}')

        ckb_toml_parts.append(f'\n[rpc]\nlisten_address = "{rpc_listen}"')
        ckb_toml_parts.append('modules = ["Net", "Pool", "Miner", "Chain", "Stats", "Subscription", "Experiment", "Debug", "Indexer"]')

        ckb_toml_parts.append(f'\n[logger]\nfilter = "info"\nlog_to_file = true\nlog_to_stdout = false')
        ckb_toml_parts.append(f'log_dir = "{self.data_dir}/logs"')

        if block_assembler:
            ckb_toml_parts.append(f'\n[block_assembler]')
            ckb_toml_parts.append(f'code_hash = "{block_assembler.get("code_hash", "")}"')
            ckb_toml_parts.append(f'args = "{block_assembler.get("args", "")}"')
            ckb_toml_parts.append(f'hash_type = "{block_assembler.get("hash_type", "type")}"')
            ckb_toml_parts.append(f'message = "{block_assembler.get("message", "0x")}"')

        miner_dummy = merged.get("ckb_miner_dummy_value")
        if miner_dummy:
            ckb_toml_parts.append(f'\n[miner.workers]\n[miner.workers.Dummy]\ndelay_type = "Constant"\nvalue = {miner_dummy}')

        if prometheus:
            ckb_toml_parts.append(f'\n[metrics.exporter.prometheus]')
            ckb_toml_parts.append(f'listen_address = "{prometheus.get("listen_address", "0.0.0.0:8100")}"')

        ckb_toml_content = "\n".join(ckb_toml_parts)
        self.ssh.run(f"cat > {self.workspace}/ckb.toml << 'CKBEOF'\n{ckb_toml_content}\nCKBEOF", sudo=True)

        miner_poll = merged.get("ckb_miner_poll_interval", 1000)
        miner_toml = f"""[miner]
client = {{ rpc_url = "http://127.0.0.1:{self.rpc_port}" }}
poll_interval = {miner_poll}
"""
        if block_assembler and block_assembler.get("key"):
            pass

        self.ssh.run(f"cat > {self.workspace}/ckb-miner.toml << 'MINEREOF'\n{miner_toml}\nMINEREOF", sudo=True)

        systemd_unit = f"""[Unit]
Description=CKB Node ({self.service})
After=network.target

[Service]
Type=simple
ExecStart={self.workspace}/ckb -C {self.workspace} run
Restart=on-failure
User=root
WorkingDirectory={self.workspace}

[Install]
WantedBy=multi-user.target
"""
        self.ssh.run(
            f"cat > /etc/systemd/system/{self.service}.service << 'SVCEOF'\n{systemd_unit}\nSVCEOF",
            sudo=True,
        )

        miner_service_name = f"{self.service}_miner"
        miner_unit = f"""[Unit]
Description=CKB Miner ({miner_service_name})
After={self.service}.service

[Service]
Type=simple
ExecStart={self.workspace}/ckb -C {self.workspace} miner
Restart=on-failure
User=root
WorkingDirectory={self.workspace}

[Install]
WantedBy=multi-user.target
"""
        self.ssh.run(
            f"cat > /etc/systemd/system/{miner_service_name}.service << 'MSVEOF'\n{miner_unit}\nMSVEOF",
            sudo=True,
        )
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
