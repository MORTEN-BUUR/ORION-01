"""GSF HTTP client for execute, verify_audit, export_chain, mesh_sync."""

from __future__ import annotations

import json
from typing import Any

import httpx


def execute(
    base_url: str,
    action: str,
    payload: dict[str, Any],
    *,
    timeout: float = 30.0,
) -> dict[str, Any]:
    """Execute an action via POST /run."""
    with httpx.Client(timeout=timeout) as client:
        resp = client.post(
            f"{base_url.rstrip('/')}/run",
            json={"action": action, "payload": payload},
        )
        resp.raise_for_status()
        return resp.json()


def verify_audit(
    base_url: str,
    entries: list[dict[str, Any]],
    *,
    timeout: float = 30.0,
) -> dict[str, Any]:
    """Verify audit entries via POST /audit/verify."""
    with httpx.Client(timeout=timeout) as client:
        resp = client.post(
            f"{base_url.rstrip('/')}/audit/verify",
            json={"entries": entries},
        )
        resp.raise_for_status()
        return resp.json()


def export_chain(base_url: str, *, timeout: float = 30.0) -> dict[str, Any]:
    """Export audit chain via GET /audit/export."""
    with httpx.Client(timeout=timeout) as client:
        resp = client.get(f"{base_url.rstrip('/')}/audit/export", timeout=timeout)
        resp.raise_for_status()
        return resp.json()


def mesh_sync(
    base_url: str,
    entries: list[dict[str, Any]],
    peer_fingerprint: str,
    *,
    timeout: float = 30.0,
) -> dict[str, Any]:
    """Sync mesh via POST /mesh/sync."""
    with httpx.Client(timeout=timeout) as client:
        resp = client.post(
            f"{base_url.rstrip('/')}/mesh/sync",
            json={"entries": entries, "peer_fingerprint": peer_fingerprint},
        )
        resp.raise_for_status()
        return resp.json()


class GsfClient:
    """Client for GENESIS Sovereign Fabric API."""

    def __init__(self, base_url: str, timeout: float = 30.0) -> None:
        self.base_url = base_url.rstrip("/")
        self.timeout = timeout

    def execute(self, action: str, payload: dict[str, Any]) -> dict[str, Any]:
        return execute(self.base_url, action, payload, timeout=self.timeout)

    def verify_audit(self, entries: list[dict[str, Any]]) -> dict[str, Any]:
        return verify_audit(self.base_url, entries, timeout=self.timeout)

    def export_chain(self) -> dict[str, Any]:
        return export_chain(self.base_url, timeout=self.timeout)

    def mesh_sync(
        self,
        entries: list[dict[str, Any]],
        peer_fingerprint: str,
    ) -> dict[str, Any]:
        return mesh_sync(
            self.base_url,
            entries,
            peer_fingerprint,
            timeout=self.timeout,
        )
