"""SSH client wrapper using paramiko for remote command execution and file transfer."""

import io
import os
import logging
import paramiko
import tempfile
import time
from typing import Optional

logger = logging.getLogger(__name__)


class SSHClient:
    def __init__(
        self,
        host: str,
        user: str,
        private_key_path: str,
        port: int = 22,
        timeout: int = 60,
    ):
        self.host = host
        self.user = user
        self.private_key_path = private_key_path
        self.port = port
        self.timeout = timeout
        self._client: Optional[paramiko.SSHClient] = None

    def connect(self):
        if self._client is not None:
            return
        self._client = paramiko.SSHClient()
        self._client.set_missing_host_key_policy(paramiko.AutoAddPolicy())
        key = paramiko.RSAKey.from_private_key_file(self.private_key_path)
        self._client.connect(
            hostname=self.host,
            port=self.port,
            username=self.user,
            pkey=key,
            timeout=self.timeout,
        )
        logger.info(f"Connected to {self.user}@{self.host}")

    def close(self):
        if self._client:
            self._client.close()
            self._client = None

    def run(self, cmd: str, sudo: bool = False, check: bool = True, env: Optional[dict] = None) -> str:
        self.connect()
        if sudo:
            escaped = cmd.replace("\\", "\\\\").replace('"', '\\"').replace('$', '\\$').replace('`', '\\`')
            cmd = f'sudo bash -c "{escaped}"'
        logger.info(f"[{self.host}] Running: {cmd}")
        stdin, stdout, stderr = self._client.exec_command(cmd, timeout=self.timeout, environment=env)
        exit_code = stdout.channel.recv_exit_status()
        out = stdout.read().decode("utf-8", errors="replace")
        err = stderr.read().decode("utf-8", errors="replace")
        if out:
            logger.info(f"[{self.host}] stdout: {out.strip()}")
        if err:
            logger.warning(f"[{self.host}] stderr: {err.strip()}")
        if check and exit_code != 0:
            raise RuntimeError(
                f"Command failed on {self.host} (exit={exit_code}): {cmd}\nstderr: {err}"
            )
        return out

    def run_long(self, cmd: str, sudo: bool = False, timeout: int = 72000) -> str:
        self.connect()
        if sudo:
            escaped = cmd.replace("\\", "\\\\").replace('"', '\\"').replace('$', '\\$').replace('`', '\\`')
            cmd = f'sudo bash -c "{escaped}"'
        logger.info(f"[{self.host}] Running (long): {cmd}")
        stdin, stdout, stderr = self._client.exec_command(cmd, timeout=timeout)
        exit_code = stdout.channel.recv_exit_status()
        out = stdout.read().decode("utf-8", errors="replace")
        err = stderr.read().decode("utf-8", errors="replace")
        if out:
            logger.info(f"[{self.host}] stdout (truncated): {out.strip()[:500]}")
        if err:
            logger.warning(f"[{self.host}] stderr: {err.strip()[:500]}")
        return out

    def upload(self, local_path: str, remote_path: str):
        self.connect()
        sftp = self._client.open_sftp()
        logger.info(f"[{self.host}] Uploading {local_path} -> {remote_path}")
        sftp.put(local_path, remote_path)
        sftp.close()

    def download(self, remote_path: str, local_path: str, sudo: bool = False):
        self.connect()
        if sudo:
            tmp_path = f"/tmp/_deploy_download_{os.path.basename(remote_path)}"
            self.run(f"cp {remote_path} {tmp_path} && chmod 644 {tmp_path}", sudo=True)
            remote_path = tmp_path
        os.makedirs(os.path.dirname(local_path) or ".", exist_ok=True)
        sftp = self._client.open_sftp()
        logger.info(f"[{self.host}] Downloading {remote_path} -> {local_path}")
        sftp.get(remote_path, local_path)
        sftp.close()

    def write_file(self, remote_path: str, content: str, sudo: bool = False):
        """Write string content to a remote file safely via SFTP.

        Avoids shell quoting issues that arise when piping multi-line
        content through ``sudo bash -c '...'`` (heredocs, single quotes, etc.).
        """
        self.connect()
        logger.info(f"[{self.host}] Writing file {remote_path} ({len(content)} bytes)")
        sftp = self._client.open_sftp()
        if sudo:
            tmp_path = f"/tmp/_deploy_write_{os.path.basename(remote_path)}"
            with sftp.open(tmp_path, "w") as f:
                f.write(content)
            sftp.close()
            self.run(f"mkdir -p $(dirname {remote_path})", sudo=True)
            self.run(f"mv {tmp_path} {remote_path}", sudo=True)
        else:
            with sftp.open(remote_path, "w") as f:
                f.write(content)
            sftp.close()

    def __enter__(self):
        self.connect()
        return self

    def __exit__(self, *args):
        self.close()

    def service_action(self, service_name: str, action: str):
        self.run(f"systemctl {action} {service_name}", sudo=True)

    def wait_for_file_pattern(self, filepath: str, pattern: str, timeout: int = 72000, interval: int = 30):
        """Wait until a pattern appears in a file on the remote host."""
        import re
        start = time.time()
        while time.time() - start < timeout:
            out = self.run(f"grep -E '{pattern}' {filepath} || true", check=False)
            if out.strip():
                logger.info(f"[{self.host}] Pattern found: {out.strip()[:200]}")
                return out.strip()
            time.sleep(interval)
        raise TimeoutError(f"Pattern '{pattern}' not found in {filepath} within {timeout}s")
