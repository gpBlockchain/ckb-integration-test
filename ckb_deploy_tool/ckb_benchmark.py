"""CKB benchmark operations via SSH - replaces ansible-ckb-benchmark role."""

import logging
import os
from typing import Optional
from .ssh_client import SSHClient
from .inventory import Host

logger = logging.getLogger(__name__)

CKB_BENCHMARK_WORKSPACE = "/var/lib/ckb-benchmark"
CKB_BENCHMARK_BIN = f"{CKB_BENCHMARK_WORKSPACE}/ckb-bench"


class CkbBenchmark:
    """Manages CKB benchmark on a remote host, replacing the ansible-ckb-benchmark role."""

    def __init__(self, ssh: SSHClient, host: Host, vars_data: dict):
        self.ssh = ssh
        self.host = host
        self.vars = vars_data
        self.workspace = CKB_BENCHMARK_WORKSPACE

    def install(self, bench_url: Optional[str] = None):
        """Install ckb-bench binary (replaces ckb_benchmark_install tag)."""
        logger.info(f"Installing ckb-bench on {self.host.name}")
        self.ssh.run(f"mkdir -p {self.workspace}/data", sudo=True)
        url = bench_url or "https://github.com/nervosnetwork/ckb-integration-test/releases/latest/download/ckb-bench-linux-x86_64.tar.gz"
        self.ssh.run(
            f"cd /tmp && "
            f"curl -L -o ckb-bench.tar.gz '{url}' && "
            f"tar xzf ckb-bench.tar.gz && "
            f"cp -f ckb-bench {CKB_BENCHMARK_BIN} && "
            f"chmod +x {CKB_BENCHMARK_BIN} && "
            f"rm -f ckb-bench.tar.gz ckb-bench",
            sudo=True,
        )

    def prepare(self, rpc_urls: list[str], n_users: int = 100, owner_privkey: str = ""):
        """Prepare benchmark cells (replaces ckb_benchmark_prepare tag)."""
        urls_str = ",".join(rpc_urls)
        cmd = (
            f"{CKB_BENCHMARK_BIN} prepare "
            f"--rpc-urls {urls_str} "
            f"--n-users {n_users}"
        )
        if owner_privkey:
            cmd += f" --owner-privkey {owner_privkey}"
        logger.info(f"Preparing benchmark on {self.host.name}: {cmd}")
        self.ssh.run(cmd, sudo=True)

    def miner_start(
        self,
        rpc_url: str,
        mining_interval_ms: int = 500,
        min_tx_size: Optional[int] = None,
        n_blocks: Optional[int] = None,
    ):
        """Start miner via ckb-bench (replaces ckb_benchmark_miner_start tag)."""
        cmd = (
            f"nohup {CKB_BENCHMARK_BIN} miner "
            f"--rpc-url {rpc_url} "
            f"--mining-interval-ms {mining_interval_ms}"
        )
        if min_tx_size is not None:
            cmd += f" --min-tx-size {min_tx_size}"
        if n_blocks is not None:
            cmd += f" --n-blocks {n_blocks}"
        cmd += f" > {self.workspace}/data/ckb-bench-miner.log 2>&1 &"
        logger.info(f"Starting benchmark miner on {self.host.name}")
        self.ssh.run(cmd, sudo=True)

    def bench_with_tps(
        self,
        rpc_urls: list[str],
        tps: int = 2000,
        n_users: int = 100,
        n_inout: int = 1,
        bench_time_ms: int = 1800000,
        concurrent_requests: int = 8,
        owner_privkey: str = "",
        logfile: Optional[str] = None,
        min_fee: Optional[int] = None,
        max_fee: Optional[int] = None,
    ):
        """Run benchmark with TPS (replaces ckb_benchmark_with_tps / bench_with_tps_and_fee tags)."""
        urls_str = ",".join(rpc_urls)
        log_path = logfile or f"{self.workspace}/data/ckb-bench.log"
        cmd = (
            f"{CKB_BENCHMARK_BIN} bench "
            f"--rpc-urls {urls_str} "
            f"--n-users {n_users} "
            f"--n-inout {n_inout} "
            f"--bench-time-ms {bench_time_ms} "
            f"--concurrent-requests {concurrent_requests} "
            f"--tps {tps}"
        )
        if owner_privkey:
            cmd += f" --owner-privkey {owner_privkey}"
        if min_fee is not None:
            cmd += f" --min-fee {min_fee}"
        if max_fee is not None:
            cmd += f" --max-fee {max_fee}"
        cmd += f" >> {log_path} 2>&1"
        logger.info(f"Running benchmark on {self.host.name}: tps={tps}, n_inout={n_inout}")
        self.ssh.run(cmd, sudo=True)

    def bench_with_tps_background(self, **kwargs):
        """Run benchmark with TPS in background."""
        rpc_urls = kwargs.pop("rpc_urls")
        urls_str = ",".join(rpc_urls)
        logfile = kwargs.pop("logfile", f"{self.workspace}/data/ckb-bench.log")
        tps = kwargs.get("tps", 2000)
        n_users = kwargs.get("n_users", 100)
        n_inout = kwargs.get("n_inout", 1)
        bench_time_ms = kwargs.get("bench_time_ms", 1800000)
        concurrent_requests = kwargs.get("concurrent_requests", 8)
        owner_privkey = kwargs.get("owner_privkey", "")
        min_fee = kwargs.get("min_fee")
        max_fee = kwargs.get("max_fee")

        cmd = (
            f"nohup {CKB_BENCHMARK_BIN} bench "
            f"--rpc-urls {urls_str} "
            f"--n-users {n_users} "
            f"--n-inout {n_inout} "
            f"--bench-time-ms {bench_time_ms} "
            f"--concurrent-requests {concurrent_requests} "
            f"--tps {tps}"
        )
        if owner_privkey:
            cmd += f" --owner-privkey {owner_privkey}"
        if min_fee is not None:
            cmd += f" --min-fee {min_fee}"
        if max_fee is not None:
            cmd += f" --max-fee {max_fee}"
        cmd += f" >> {logfile} 2>&1 &"
        logger.info(f"Running benchmark in background on {self.host.name}")
        self.ssh.run(cmd, sudo=True)

    def stop(self):
        """Stop ckb-bench processes (replaces ckb_bench_stop tag)."""
        logger.info(f"Stopping ckb-bench on {self.host.name}")
        self.ssh.run("pkill -f ckb-bench || true", sudo=True)

    def clean(self):
        """Clean benchmark data (replaces ckb_bench_clean tag)."""
        logger.info(f"Cleaning benchmark on {self.host.name}")
        self.stop()
        self.ssh.run(f"rm -rf {self.workspace}", sudo=True)

    def collect_results(self, local_dest: str, log_file: str = "data.tar.gz"):
        """Fetch benchmark results (replaces process_result / fetch tag)."""
        remote_path = f"{self.workspace}/{log_file}"
        logger.info(f"Collecting results from {self.host.name}: {remote_path} -> {local_dest}")
        self.ssh.download(remote_path, local_dest, sudo=True)

    def collect_json_log(self, local_dir: str):
        """Fetch ckb-bench.json, ckb-bench.log, ckb-bench.brief.md."""
        os.makedirs(local_dir, exist_ok=True)
        for fname in ["ckb-bench.json", "ckb-bench.log", "ckb-bench.brief.md"]:
            try:
                self.ssh.download(
                    f"{self.workspace}/data/{fname}",
                    os.path.join(local_dir, fname),
                    sudo=True,
                )
            except Exception as e:
                logger.warning(f"Failed to download {fname}: {e}")
