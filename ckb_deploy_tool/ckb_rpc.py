"""CKB JSON-RPC client."""

import json
import logging
import time
import urllib.request
import urllib.error

from .config import get as _cfg

logger = logging.getLogger(__name__)


def _rpc_ready_retries():
    return _cfg("rpc.ready_retries", 60)


def _rpc_ready_delay():
    return _cfg("rpc.ready_delay_sec", 5)


class CkbRpcClient:
    def __init__(self, url: str):
        self.url = url

    def _call(self, method: str, params: list = None) -> dict:
        params = params or []
        payload = {
            "id": 0,
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        }
        data = json.dumps(payload).encode("utf-8")
        req = urllib.request.Request(
            self.url,
            data=data,
            headers={"Content-Type": "application/json"},
        )
        with urllib.request.urlopen(req, timeout=30) as resp:
            return json.loads(resp.read().decode("utf-8"))

    def _call_with_retry(self, method: str, params: list = None,
                         retries: int = None,
                         delay: int = None) -> dict:
        """Call an RPC method, retrying on connection errors (e.g. node still booting)."""
        if retries is None:
            retries = _rpc_ready_retries()
        if delay is None:
            delay = _rpc_ready_delay()
        last_err = None
        for attempt in range(1, retries + 1):
            try:
                return self._call(method, params)
            except (urllib.error.URLError, ConnectionError, OSError) as e:
                last_err = e
                logger.warning(
                    f"RPC {method} to {self.url} failed (attempt {attempt}/{retries}): {e}"
                )
                if attempt < retries:
                    time.sleep(delay)
        raise ConnectionError(
            f"RPC {method} to {self.url} failed after {retries} attempts: {last_err}"
        ) from last_err

    def wait_for_rpc_ready(self, retries: int = None,
                           delay: int = None):
        """Block until the RPC endpoint responds successfully."""
        self._call_with_retry("local_node_info", retries=retries, delay=delay)
        logger.info(f"RPC {self.url} is ready")

    def local_node_info(self) -> dict:
        resp = self._call_with_retry("local_node_info")
        return resp["result"]

    def add_node(self, peer_id: str, address: str):
        resp = self._call_with_retry("add_node", [peer_id, address])
        logger.info(f"add_node response: {resp}")
        return resp

    def set_network_active(self, state: bool):
        resp = self._call_with_retry("set_network_active", [state])
        logger.info(f"set_network_active({state}) response: {resp}")
        return resp

    def tx_pool_info(self) -> dict:
        resp = self._call("tx_pool_info")
        return resp["result"]

    def get_tip_block_number(self) -> int:
        resp = self._call("get_tip_block_number")
        return int(resp["result"], 16)

    def get_consensus(self) -> dict:
        resp = self._call("get_consensus")
        return resp["result"]

    def wait_pending_ge(self, target: int, retries: int = 60000, delay: int = 10):
        """Wait until tx_pool_info.pending >= target."""
        for i in range(retries):
            try:
                info = self.tx_pool_info()
                pending = int(info["pending"], 16)
                logger.info(f"tx_pool pending={pending}, target>={target}")
                if pending >= target:
                    return pending
            except Exception as e:
                logger.warning(f"tx_pool_info failed: {e}")
            time.sleep(delay)
        raise TimeoutError(f"Pending never reached >={target}")

    def wait_pending_eq(self, target: int, retries: int = 60000, delay: int = 10):
        """Wait until tx_pool_info.pending == target."""
        for i in range(retries):
            try:
                info = self.tx_pool_info()
                pending = int(info["pending"], 16)
                logger.info(f"tx_pool pending={pending}, target=={target}")
                if pending == target:
                    return pending
            except Exception as e:
                logger.warning(f"tx_pool_info failed: {e}")
            time.sleep(delay)
        raise TimeoutError(f"Pending never reached =={target}")
