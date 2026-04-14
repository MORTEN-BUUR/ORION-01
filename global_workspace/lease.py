"""
GlobalLease — Time-limited broadcast token for the global workspace.

Based on Baars' Global Workspace Theory (GWT):
A module that receives a lease may send its content to all other
modules in the global workspace (global broadcast).
"""

from __future__ import annotations

import hashlib
import uuid
from dataclasses import dataclass, field
from datetime import datetime, timezone
from typing import Any


@dataclass
class GlobalLease:
    """
    Time-limited access token for the global workspace.

    Lifecycle:
        requested → granted → active → expired | released
    """

    module: str
    content: Any
    duration_ms: int
    lease_id: str = field(default_factory=lambda: str(uuid.uuid4()))
    granted_at: datetime = field(
        default_factory=lambda: datetime.now(timezone.utc)
    )
    status: str = "granted"  # granted | active | expired | released

    # -------------------------------------------------------------------
    # Derived properties
    # -------------------------------------------------------------------

    @property
    def expires_at_ms(self) -> float:
        """Expiry timestamp as Unix milliseconds."""
        return self.granted_at.timestamp() * 1000 + self.duration_ms

    @property
    def is_valid(self) -> bool:
        """True as long as the lease has not yet expired or been released."""
        if self.status in ("expired", "released"):
            return False
        now_ms = datetime.now(timezone.utc).timestamp() * 1000
        return now_ms < self.expires_at_ms

    # -------------------------------------------------------------------
    # Actions
    # -------------------------------------------------------------------

    def release(self) -> None:
        """Explicit early release of the lease."""
        if self.status not in ("expired", "released"):
            self.status = "released"

    def expire(self) -> None:
        """Time-triggered expiry (called by GlobalWorkspace)."""
        if self.status not in ("released",):
            self.status = "expired"

    # -------------------------------------------------------------------
    # Fingerprint
    # -------------------------------------------------------------------

    def fingerprint(self) -> str:
        """SHA-256 fingerprint of the lease content for the audit chain."""
        raw = f"{self.lease_id}|{self.module}|{self.granted_at.isoformat()}|{str(self.content)}"
        return hashlib.sha256(raw.encode()).hexdigest()

    def to_dict(self) -> dict:
        return {
            "lease_id": self.lease_id,
            "module": self.module,
            "status": self.status,
            "granted_at": self.granted_at.isoformat(),
            "duration_ms": self.duration_ms,
            "expires_at_ms": self.expires_at_ms,
            "fingerprint": self.fingerprint(),
        }
