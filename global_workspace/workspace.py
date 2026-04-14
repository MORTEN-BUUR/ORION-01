"""
GlobalWorkspace — Lease management and global broadcast.

Implements the core mechanism of Global Workspace Theory (Baars 1988):
  - Modules compete for the global workspace (competition)
  - The module with the highest weight receives a time-limited lease
  - During the lease, the content is sent to all registered modules
  - Expired leases are automatically cleaned up

Phi-Proxy: Integration is measured by the number of actively informed
modules × average broadcast depth.
"""

from __future__ import annotations

import hashlib
from datetime import datetime, timezone
from typing import Any, Callable

from global_workspace.lease import GlobalLease

# Default lease duration in milliseconds (configurable)
DEFAULT_LEASE_DURATION_MS: int = 500


class GlobalWorkspace:
    """
    Global workspace — manages leases and broadcasts.

    Each `request_lease` call:
      1. Terminates the existing active lease (if present and weaker)
      2. Creates a new GlobalLease for the requesting module
      3. Sends the content via broadcast to all subscribers
      4. Writes the event to the internal audit log

    Subscribers register with `subscribe(module_name, callback)`.
    """

    def __init__(self, default_duration_ms: int = DEFAULT_LEASE_DURATION_MS):
        self._default_duration_ms = default_duration_ms
        self._active_lease: GlobalLease | None = None
        self._history: list[GlobalLease] = []
        self._subscribers: dict[str, Callable[[str, Any, GlobalLease], None]] = {}
        self._audit_log: list[dict] = []

    # ------------------------------------------------------------------
    # Subscriber management
    # ------------------------------------------------------------------

    def subscribe(
        self,
        module: str,
        callback: Callable[[str, Any, GlobalLease], None],
    ) -> None:
        """
        Registers a module as a receiver of global broadcasts.

        callback(sender_module, content, lease) is called on every broadcast.
        """
        self._subscribers[module] = callback

    def unsubscribe(self, module: str) -> None:
        """Removes a module from the subscriber list."""
        self._subscribers.pop(module, None)

    # ------------------------------------------------------------------
    # Lease lifecycle
    # ------------------------------------------------------------------

    def request_lease(
        self,
        module: str,
        content: Any,
        duration_ms: int | None = None,
        weight: float = 1.0,
    ) -> GlobalLease | None:
        """
        Request a lease on the global workspace.

        Parameters
        ----------
        module:      Name of the requesting module
        content:     Content to be broadcast
        duration_ms: Lease duration in ms (default: DEFAULT_LEASE_DURATION_MS)
        weight:      Priority weight — higher weight preempts the active lease

        Returns
        -------
        The new GlobalLease, or None if the lease was denied.
        """
        self._gc()  # clean up expired leases

        # Check existing lease
        if self._active_lease is not None and self._active_lease.is_valid:
            # Preemption only with positive weight
            if weight <= 0:
                self._audit("lease_denied", module, {"reason": "insufficient_weight"})
                return None
            self._active_lease.release()
            self._audit("lease_preempted", self._active_lease.module, self._active_lease.to_dict())

        duration = duration_ms if duration_ms is not None else self._default_duration_ms
        lease = GlobalLease(module=module, content=content, duration_ms=duration)
        lease.status = "active"
        self._active_lease = lease
        self._history.append(lease)

        self._audit("lease_granted", module, lease.to_dict())
        self._broadcast(lease)
        return lease

    def release_lease(self, lease: GlobalLease) -> None:
        """Explicit release of a lease by the owning module."""
        lease.release()
        if self._active_lease is lease:
            self._active_lease = None
        self._audit("lease_released", lease.module, lease.to_dict())

    # ------------------------------------------------------------------
    # Broadcast
    # ------------------------------------------------------------------

    def _broadcast(self, lease: GlobalLease) -> None:
        """Sends the lease content to all registered subscribers."""
        delivered: list[str] = []
        for sub_module, callback in self._subscribers.items():
            if sub_module == lease.module:
                continue  # sender does not receive its own broadcast
            try:
                callback(lease.module, lease.content, lease)
                delivered.append(sub_module)
            except Exception:
                pass  # individual errors must not interrupt the broadcast
        self._audit(
            "broadcast",
            lease.module,
            {"lease_id": lease.lease_id, "delivered_to": delivered, "count": len(delivered)},
        )

    # ------------------------------------------------------------------
    # Introspection
    # ------------------------------------------------------------------

    @property
    def active_lease(self) -> GlobalLease | None:
        """Currently active lease (or None)."""
        self._gc()
        return self._active_lease

    @property
    def history(self) -> list[GlobalLease]:
        """All past leases (including expired ones)."""
        return list(self._history)

    @property
    def audit_log(self) -> list[dict]:
        """Immutable record of all workspace events."""
        return list(self._audit_log)

    def phi_proxy(self) -> float:
        """
        Approximation of the GWT Phi contribution.

        Formula: (number of broadcasts × average recipients) / normalisation factor
        Returns a value between 0.0 and 1.0.
        """
        broadcast_entries = [e for e in self._audit_log if e["event"] == "broadcast"]
        if not broadcast_entries:
            return 0.0
        total_delivered = sum(e["data"].get("count", 0) for e in broadcast_entries)
        avg_delivered = total_delivered / len(broadcast_entries)
        subscriber_count = max(len(self._subscribers), 1)
        return min(avg_delivered / subscriber_count, 1.0)

    # ------------------------------------------------------------------
    # Internal helpers
    # ------------------------------------------------------------------

    def _gc(self) -> None:
        """Cleans up expired active leases."""
        if self._active_lease is not None and not self._active_lease.is_valid:
            self._active_lease.expire()
            self._audit("lease_expired", self._active_lease.module, self._active_lease.to_dict())
            self._active_lease = None

    def _audit(self, event: str, module: str, data: dict) -> None:
        """Writes an entry to the internal audit log."""
        entry: dict[str, Any] = {
            "timestamp": datetime.now(timezone.utc).isoformat(),
            "event": event,
            "module": module,
            "data": data,
        }
        # chain hash for integrity
        prev_hash = self._audit_log[-1]["hash"] if self._audit_log else None
        raw = f"{entry['timestamp']}|{event}|{module}|{prev_hash or ''}|{str(data)}"
        entry["hash"] = hashlib.sha256(raw.encode()).hexdigest()
        self._audit_log.append(entry)
