"""Parse Ansible-style inventory.yml into Python data structures."""

import yaml
from dataclasses import dataclass, field
from typing import Optional


@dataclass
class Host:
    name: str
    ansible_user: str
    ansible_host: str
    internal_ip: Optional[str] = None
    extra: dict = field(default_factory=dict)

    @property
    def p2p_ip(self) -> str:
        return self.internal_ip if self.internal_ip else self.ansible_host

    @property
    def ssh_host(self) -> str:
        return self.ansible_host


class Inventory:
    def __init__(self, path: str):
        with open(path) as f:
            data = yaml.safe_load(f)
        self.hosts: dict[str, Host] = {}
        self._parse(data)

    def _parse(self, data: dict):
        hosts_section = data.get("all", {}).get("hosts", {})
        for name, attrs in hosts_section.items():
            attrs = attrs or {}
            self.hosts[name] = Host(
                name=name,
                ansible_user=attrs.get("ansible_user", "ckb"),
                ansible_host=attrs.get("ansible_host", ""),
                internal_ip=attrs.get("internal_ip"),
                extra={
                    k: v
                    for k, v in attrs.items()
                    if k not in ("ansible_user", "ansible_host", "internal_ip")
                },
            )

    def get(self, name: str) -> Host:
        return self.hosts[name]

    def all_hosts(self) -> list[Host]:
        return list(self.hosts.values())
