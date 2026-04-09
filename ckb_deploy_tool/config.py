"""Centralized configuration management.

All default values live in ``default_config.yaml`` next to this file.
Callers use :func:`load_config` to get a dict, optionally providing an
override file that is deep-merged on top of the defaults.
"""

import copy
import os
import yaml

_CONFIG_DIR = os.path.dirname(os.path.abspath(__file__))
_DEFAULT_CONFIG_PATH = os.path.join(_CONFIG_DIR, "default_config.yaml")

_cache: dict | None = None


def _deep_merge(base: dict, override: dict) -> dict:
    """Recursively merge *override* into *base* (mutates *base*)."""
    for key, val in override.items():
        if key in base and isinstance(base[key], dict) and isinstance(val, dict):
            _deep_merge(base[key], val)
        else:
            base[key] = val
    return base


def load_config(override_path: str | None = None) -> dict:
    """Load the default config, optionally merged with an override file.

    The result is cached after the first call (per override_path=None).
    """
    global _cache
    if override_path is None and _cache is not None:
        return _cache

    with open(_DEFAULT_CONFIG_PATH) as f:
        cfg = yaml.safe_load(f)

    if override_path and os.path.exists(override_path):
        with open(override_path) as f:
            overrides = yaml.safe_load(f) or {}
        _deep_merge(cfg, overrides)

    if override_path is None:
        _cache = cfg
    return cfg


def get(key_path: str, default=None, config: dict | None = None):
    """Get a nested value using dot-separated path, e.g. ``'ckb.rpc_modules'``."""
    cfg = config if config is not None else load_config()
    keys = key_path.split(".")
    node = cfg
    for k in keys:
        if isinstance(node, dict) and k in node:
            node = node[k]
        else:
            return default
    return node
